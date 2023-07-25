//! Generates a 30x30 world where an actor can be told to move through a narrow snake-like path
//!

use std::collections::HashMap;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;
/// Timestep of actor movement system
const ACTOR_TIMESTEP: f32 = 0.25;

/// Pixel `x` length of the world
const PIXEL_LENGTH: u32 = 1920;
/// Pixel `y` depth of the world
const PIXEL_DEPTH: u32 = 1920;
/// Dimension of square sprites making up the world
const FIELD_SPRITE_DIMENSION: f32 = 64.0;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.insert_resource(FixedTime::new_from_secs(ACTOR_TIMESTEP))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup_visualisation, setup_navigation))
		.add_systems(Update, (user_input, actor_update_route))
		.add_systems(Update, (update_sprite_visuals_based_on_actor,))
		.add_systems(FixedUpdate, actor_steering)
		.run();
}

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct SectorLabel(u32, u32);

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct GridLabel(usize, usize);

/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;

/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Default, Component)]
struct Pathing {
	source_sector: Option<SectorID>,
	source_grid_cell: Option<FieldCell>,
	target_sector: Option<SectorID>,
	target_goal: Option<FieldCell>,
	portal_route: Option<Vec<(SectorID, FieldCell)>>,
}

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let map_length = 30; // in sprite count
	let map_depth = 30; // in sprite count
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
	let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	let sector_cost_fields = SectorCostFields::from_csv_dir(map_length, map_depth, dir);
	let fields = sector_cost_fields.get();
	// iterate over each sector field to place the sprites
	for (sector_id, field) in fields.iter() {
		// iterate over the dimensions of the field
		for (i, column) in field.get_field().iter().enumerate() {
			for (j, value) in column.iter().enumerate() {
				// grid origin is always in the top left
				let sprite_x = FIELD_SPRITE_DIMENSION;
				let sprite_y = FIELD_SPRITE_DIMENSION;
				let sector_offset = get_sector_xy_at_top_left(
					*sector_id,
					map_length * sprite_x as u32,
					map_depth * sprite_y as u32,
					sprite_x,
				);
				let x = sector_offset.x + 32.0 + (sprite_x * i as f32);
				let y = sector_offset.y - 32.0 - (sprite_y * j as f32);
				cmds.spawn(SpriteBundle {
					texture: asset_server.load(get_basic_icon(*value)),
					transform: Transform::from_xyz(x, y, 0.0),
					..default()
				})
				.insert(GridLabel(i, j))
				.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()));
			}
		}
	}
}
/// Spawn navigation related entities
fn setup_navigation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// create the entity handling the algorithm
	let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	let map_length = 30; // in sprite count
	let map_depth = 30; // in sprite count
	cmds.spawn(FlowFieldTilesBundle::from_csv(map_length, map_depth, &dir));
	// create the controllable actor in the top right corner
	cmds.spawn(SpriteBundle {
		texture: asset_server.load("2d/2d_actor_sprite.png"),
		transform: Transform::from_xyz(-928.0, -928.0, 1.0),
		..default()
	})
	.insert(Actor)
	.insert(Pathing::default());
}

/// Handle generating a PathRequest via right click
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
			if let Some((target_sector_id, goal_id)) = get_sector_and_field_id_from_xy(
				world_position,
				PIXEL_LENGTH,
				PIXEL_DEPTH,
				FIELD_SPRITE_DIMENSION,
			) {
				info!(
					"Cursor sector_id {:?}, goal_id in sector {:?}",
					target_sector_id, goal_id
				);
				let (tform, mut pathing) = actor_q.get_single_mut().unwrap();
				let (source_sector_id, source_grid_cell) = get_sector_and_field_id_from_xy(
					tform.translation.truncate(),
					PIXEL_LENGTH,
					PIXEL_DEPTH,
					FIELD_SPRITE_DIMENSION,
				)
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
			pathing.target_sector.unwrap(),
			pathing.target_goal.unwrap(),
		) {
			pathing.portal_route = Some(route.clone());
		}
	}
}
/// Actor speed measured in pixels per fixed tick
const SPEED: f32 = 64.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
fn actor_steering(
	mut actor_q: Query<(&mut Transform, &mut Pathing), With<Actor>>,
	flow_cache_q: Query<&FlowFieldCache>,
) {
	let (mut tform, mut pathing) = actor_q.get_single_mut().unwrap();
	let flow_cache = flow_cache_q.get_single().unwrap();

	if pathing.target_goal.is_some() {
		// lookup the overarching route
		if let Some(route) = pathing.portal_route.as_mut() {
			// info!("Route: {:?}", route);
			// find the current actors postion in grid space
			let (curr_actor_sector, curr_actor_grid) = get_sector_and_field_id_from_xy(
				tform.translation.truncate(),
				PIXEL_LENGTH,
				PIXEL_DEPTH,
				FIELD_SPRITE_DIMENSION,
			)
			.unwrap();
			// tirm the actor stored route as it makes progress
			// this ensures it doesn't use a previous goal from
			// a sector it has already been through when it needs
			// to pass through it again as part of a different part of the route
			if curr_actor_sector != route.first().unwrap().0 {
				route.remove(0);
			}
			// lookup the relevant sector-goal of this sector
			'routes: for (sector, goal) in route.iter() {
				if *sector == curr_actor_sector {
					// get the flow field
					if let Some(field) = flow_cache.get_field(*sector, *goal) {
						// based on actor grid cell find the directional vector it should move in
						let cell_value = field.get_grid_value(curr_actor_grid);
						let dir = get_2d_direction_unit_vector_from_bits(cell_value);
						// info!("In sector {:?}, in grid cell {:?}", sector, curr_actor_grid);
						// info!("Direction to move: {}", dir);
						let velocity = dir * SPEED;
						// move the actor based on the velocity
						tform.translation += velocity.extend(0.0);
					}
					break 'routes;
				}
			}
		}
	}
}
/// Get asset path to sprite icons
fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}

/// Whenever the actor has a path assigned attempt to get the current flowfield and update all the map sprites to visualise the directions of flow
fn update_sprite_visuals_based_on_actor(
	actor_q: Query<&Pathing, With<Actor>>,
	flowfield_q: Query<&FlowFieldCache>,
	costfield_q: Query<&SectorCostFields>,
	mut grid_q: Query<(&mut Handle<Image>, &GridLabel, &SectorLabel)>,
	asset_server: Res<AssetServer>,
) {
	let f_cache = flowfield_q.get_single().unwrap();
	let sc_cache = costfield_q.get_single().unwrap();
	let pathing = actor_q.get_single().unwrap();
	if let Some(route) = &pathing.portal_route {
		let mut route_map: HashMap<SectorID, FieldCell> = HashMap::new();
		for (s, g) in route.iter() {
			route_map.insert(*s, *g);
		}
		for (mut handle, grid_label, sector_label) in grid_q.iter_mut() {
			// look for the value in the route_map if it's part of the flow, otherwise use the cost field
			if route_map.contains_key(&SectorID::new(sector_label.0, sector_label.1)) {
				let goal = route_map
					.get(&SectorID::new(sector_label.0, sector_label.1))
					.unwrap();
				if let Some(flowfield) =
					f_cache.get_field(SectorID::new(sector_label.0, sector_label.1), *goal)
				{
					let flow_value = flowfield.get_grid_value(FieldCell::new(grid_label.0, grid_label.1));
					let icon = get_ord_icon(flow_value);
					let new_handle: Handle<Image> = asset_server.load(icon);
					*handle = new_handle;
				}
			} else {
				let value = sc_cache
					.get()
					.get(&SectorID::new(sector_label.0, sector_label.1))
					.unwrap()
					.get_grid_value(FieldCell::new(grid_label.0, grid_label.1));
				let icon = get_basic_icon(value);
				let new_handle: Handle<Image> = asset_server.load(icon);
				*handle = new_handle;
			}
		}
	}
}
/// Get the asset path to ordinal icons
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
