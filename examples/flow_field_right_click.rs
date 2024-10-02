//! Generates a single [FlowField] visualisation which uses right-mouse input to set a goal position, causing the visualisation to update to graphically show the flow field lines from a !static! actor position
//!

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup,))
		.add_systems(
			Update,
			(
				user_input,
				actor_update_route,
				update_sprite_visuals_based_on_actor,
			),
		)
		.run();
}
/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct FieldCellLabel(usize, usize);
/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;
/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Default, Component)]
struct Pathing {
	source_sector: Option<SectorID>,
	source_field_cell: Option<FieldCell>,
	target_sector: Option<SectorID>,
	target_goal: Option<FieldCell>,
	portal_route: Option<Vec<(SectorID, FieldCell)>>,
}
/// Init bundle and setup world and actor
fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// create the entity handling the algorithm
	let s_path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_field_single.ron";
	let c_path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/cost_field_impassable.ron";
	let map_length = 640;
	let map_depth = 640;
	let sector_resolution = 640;
	let sprite_dimension = 64.0;
	let actor_size = 16.0;
	cmds.spawn(FlowFieldTilesBundle::from_ron(
		map_length,
		map_depth,
		sector_resolution,
		actor_size,
		&s_path,
	));
	// use the impression of the cost field to just init node images
	let cost_field = CostField::from_ron(c_path);
	// create a blank visualisation
	cmds.spawn(Camera2dBundle::default());
	for (i, column) in cost_field.get().iter().enumerate() {
		for (j, value) in column.iter().enumerate() {
			// grid origin is always in the top left
			let x = -(map_length as f32) / 2.0 + 32.0 + (sprite_dimension * i as f32);
			let y = map_depth as f32 / 2.0 - 32.0 - (sprite_dimension * j as f32);
			cmds.spawn(SpriteBundle {
				texture: asset_server.load(get_basic_icon(*value)),
				transform: Transform::from_xyz(x, y, 0.0),
				..default()
			})
			.insert(FieldCellLabel(i, j));
		}
	}
	// create the controllable actor
	cmds.spawn(SpriteBundle {
		texture: asset_server.load("2d/2d_actor_sprite.png"),
		transform: Transform::from_xyz(0.0, 0.0, -1.0),
		..default()
	})
	.insert(Actor)
	.insert(Pathing::default());
}
/// Handle user mouse clicks
fn user_input(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	dimensions_q: Query<&MapDimensions>,
	mut actor_q: Query<(&Transform, &mut Pathing), With<Actor>>,
	mut event: EventWriter<EventPathRequest>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 2d world positionn of cursor
		let (camera, camera_transform) = camera_q.single();
		let window = windows.single();
		if let Some(world_position) = window
			.cursor_position()
			.and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
			.map(|ray| ray.origin.truncate())
		{
			let map_dimensions = dimensions_q.get_single().unwrap();
			info!("World cursor position: {}", world_position);
			if let Some((target_sector_id, goal_id)) =
				map_dimensions.get_sector_and_field_cell_from_xy(world_position)
			{
				info!(
					"Cursor sector_id {:?}, goal_id in sector {:?}",
					target_sector_id, goal_id
				);
				let (tform, mut pathing) = actor_q.get_single_mut().unwrap();
				let (source_sector_id, source_field_cell) = map_dimensions
					.get_sector_and_field_cell_from_xy(tform.translation.truncate())
					.unwrap();
				info!(
					"Actor sector_id {:?}, goal_id in sector {:?}",
					source_sector_id, source_field_cell
				);
				event.send(EventPathRequest::new(
					source_sector_id,
					source_field_cell,
					target_sector_id,
					goal_id,
				));
				// update the actor pathing
				pathing.source_sector = Some(source_sector_id);
				pathing.source_field_cell = Some(source_field_cell);
				pathing.target_sector = Some(target_sector_id);
				pathing.target_goal = Some(goal_id);
				pathing.portal_route = None;
			} else {
				error!("Cursor out of bounds");
			}
		}
	}
}
/// There is a delay between the actor sending a path request and a route becoming available. This checks to see if the route is available and adds a copy to the actor
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<Actor>>, route_q: Query<&RouteCache>) {
	let mut pathing = actor_q.get_single_mut().unwrap();
	if pathing.target_goal.is_some() && pathing.portal_route.is_none() {
		let route_cache = route_q.get_single().unwrap();
		if let Some(route) = route_cache.get_route(
			pathing.source_sector.unwrap(),
			pathing.source_field_cell.unwrap(),
			pathing.target_sector.unwrap(),
			pathing.target_goal.unwrap(),
		) {
			pathing.portal_route = Some(route.get().clone());
		}
	}
}

/// Whenever the actor has a path assigned attempt to get the current flowfield and update all the map sprites to visualise the directions of flow
fn update_sprite_visuals_based_on_actor(
	actor_q: Query<&Pathing, (With<Actor>, Changed<Pathing>)>,
	flowfield_q: Query<&FlowFieldCache>,
	mut field_cell_q: Query<(&mut Handle<Image>, &FieldCellLabel)>,
	asset_server: Res<AssetServer>,
) {
	for pathing in &actor_q {
		let cache = flowfield_q.get_single().unwrap();
		if let Some(route) = &pathing.portal_route {
			let op_flowfield =
				cache.get_field(route[0].0, pathing.target_sector.unwrap(), route[0].1);
			if let Some(flowfield) = op_flowfield {
				for (mut handle, field_cell_label) in field_cell_q.iter_mut() {
					let flow_value = flowfield.get_field_cell_value(FieldCell::new(
						field_cell_label.0,
						field_cell_label.1,
					));
					let icon = get_ord_icon(flow_value);
					let new_handle: Handle<Image> = asset_server.load(icon);
					*handle = new_handle;
				}
			}
		}
	}
}
/// Get asset path of psrite assets
fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}
/// Get asset path of ordinal icon
fn get_ord_icon(value: u8) -> String {
	// temp
	if value == 64 {
		return String::from("ordinal_icons/goal.png");
	}
	//
	if has_line_of_sight(value) {
		return String::from("ordinal_icons/los.png");
	}
	let ordinal = get_ordinal_from_bits(value);
	match ordinal {
		Ordinal::North => String::from("ordinal_icons/north.png"),
		Ordinal::East => String::from("ordinal_icons/east.png"),
		Ordinal::South => String::from("ordinal_icons/south.png"),
		Ordinal::West => String::from("ordinal_icons/west.png"),
		Ordinal::NorthEast => String::from("ordinal_icons/north_east.png"),
		Ordinal::SouthEast => String::from("ordinal_icons/south_east.png"),
		Ordinal::SouthWest => String::from("ordinal_icons/south_west.png"),
		Ordinal::NorthWest => String::from("ordinal_icons/north_west.png"),
		Ordinal::Zero => String::from("ordinal_icons/impassable.png"),
	}
}
