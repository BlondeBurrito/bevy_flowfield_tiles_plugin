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
struct GridLabel(usize, usize);
/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;
/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[derive(Default, Component)]
struct Pathing {
	source_sector: Option<(u32, u32)>,
	source_grid_cell: Option<(usize, usize)>,
	target_sector: Option<(u32, u32)>,
	target_goal: Option<(usize, usize)>,
	portal_route: Option<Vec<((u32, u32), (usize, usize))>>,
}

fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// create the entity handling the algorithm
	let s_path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_field_single.ron";
	let c_path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/cost_field_impassable.ron";
	let map_length = 10; // in sprite count
	let map_depth = 10; // in sprite count
	cmds.spawn(FlowFieldTilesBundle::new_from_disk(
		map_length, map_depth, &s_path,
	));
	// use the impression of the cost field to just init node images
	let cost_field = CostField::from_file(String::from(c_path));
	// create a blank visualisation
	cmds.spawn(Camera2dBundle::default());
	for (i, column) in cost_field.get_field().iter().enumerate() {
		for (j, value) in column.iter().enumerate() {
			// grid origin is always in the top left
			let sprite_x = 64.0;
			let sprite_y = 64.0;
			let x = -sprite_x * map_length as f32 / 2.0 + 32.0 + (64.0 * i as f32);
			let y = sprite_y * map_depth as f32 / 2.0 - 32.0 - (64.0 * j as f32);
			cmds.spawn(SpriteBundle {
				texture: asset_server.load(get_basic_icon(*value)),
				transform: Transform::from_xyz(x, y, 0.0),
				..default()
			})
			.insert(GridLabel(i, j));
		}
	}
	// create the controllable actor
	cmds.spawn(SpriteBundle {
		texture: asset_server.load("2d_actor_sprite.png"),
		transform: Transform::from_xyz(0.0, 0.0, -1.0),
		..default()
	})
	.insert(Actor)
	.insert(Pathing::default());
}

fn user_input(
	mouse_button_input: Res<Input<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
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
			info!("World cursor position: {}", world_position);
			if let Some((target_sector_id, goal_id)) =
				get_sector_and_field_id_from_xy(world_position, 640, 640, 64.0)
			{
				info!(
					"Cursor sector_id {:?}, goal_id in sector {:?}",
					target_sector_id, goal_id
				);
				let (tform, mut pathing) = actor_q.get_single_mut().unwrap();
				let (source_sector_id, source_grid_cell) =
					get_sector_and_field_id_from_xy(tform.translation.truncate(), 640, 640, 64.0)
						.unwrap();
				info!(
					"Actor sector_id {:?}, goal_id in sector {:?}",
					source_sector_id, source_grid_cell
				);
				event.send(EventPathRequest::new(
					source_sector_id,
					source_grid_cell,
					target_sector_id,
					goal_id,
				));
				// update the actor pathing
				pathing.source_sector = Some(source_sector_id);
				pathing.source_grid_cell = Some(source_grid_cell);
				pathing.target_sector = Some(target_sector_id);
				pathing.target_goal = Some(goal_id);
			} else {
				error!("Cursor out of bounds");
			}
		}
	}
}
/// There is a delay between the actor sending a path request and a route becoming available. This checks to see if the route is available and adds a copy to the actor
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<Actor>>, route_q: Query<&RouteCache>) {
	let mut pathing = actor_q.get_single_mut().unwrap();
	if let Some(_) = pathing.target_goal {
		let route_cache = route_q.get_single().unwrap();
		if let Some(route) = route_cache.get_route(
			pathing.source_sector.unwrap(),
			pathing.target_sector.unwrap(),
			pathing.target_goal.unwrap(),
		) {
			pathing.portal_route = Some(route.clone());
		}
	}
}

/// Whenever the actor has a path assigned attempt to get the current flowfield and update all the map sprites to visualise the directions of flow
fn update_sprite_visuals_based_on_actor(
	actor_q: Query<&Pathing, With<Actor>>,
	flowfield_q: Query<&FlowFieldCache>,
	mut grid_q: Query<(&mut Handle<Image>, &GridLabel)>,
	asset_server: Res<AssetServer>,
) {
	let pathing = actor_q.get_single().unwrap();
	let cache = flowfield_q.get_single().unwrap();
	if let Some(route) = &pathing.portal_route {
		let op_flowfield = cache.get_field(route[0].0, route[0].1);
		if let Some(flowfield) = op_flowfield {
			for (mut handle, grid_label) in grid_q.iter_mut() {
				let flow_value = flowfield.get_grid_value(grid_label.0, grid_label.1);
				let icon = get_ord_icon(flow_value);
				let new_handle: Handle<Image> = asset_server.load(icon);
				*handle = new_handle;
			}
		}
	}
}

fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}

fn get_ord_icon(value: u8) -> String {
	// temp
	if value == 64 {
		return String::from("ordinal_icons/goal.png");
	}
	//
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