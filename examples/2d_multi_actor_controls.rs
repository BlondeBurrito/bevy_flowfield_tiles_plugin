//! Generates a 30x30 world where multiple Actors can be told to move soomewhere with right click and left click
//!

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;
/// Timestep of actor movement system
const ACTOR_TIMESTEP: f32 = 0.25;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.insert_resource(FixedTime::new_from_secs(ACTOR_TIMESTEP))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup_visualisation, setup_navigation))
		.add_systems(Update, (user_input, actor_update_route))
		.add_systems(FixedUpdate, actor_steering)
		.run();
}

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct SectorLabel(u32, u32);

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct GridLabel(usize, usize);

/// Labels the Actor to enable getting its [Transform] easily
#[derive(Component)]
struct ActorA;
/// Labels the Actor to enable getting its [Transform] easily
#[derive(Component)]
struct ActorB;

/// Attached to the Actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Default, Component)]
struct Pathing {
	source_sector: Option<(u32, u32)>,
	source_grid_cell: Option<(usize, usize)>,
	target_sector: Option<(u32, u32)>,
	target_goal: Option<(usize, usize)>,
	portal_route: Option<Vec<((u32, u32), (usize, usize))>>,
}

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let map_length = 30; // in sprite count
	let map_depth = 30; // in sprite count
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let sector_cost_fields = SectorCostFields::from_file(path);
	let fields = sector_cost_fields.get();
	// iterate over each sector field to place the sprites
	for (sector_id, field) in fields.iter() {
		// iterate over the dimensions of the field
		for (i, column) in field.get_field().iter().enumerate() {
			for (j, value) in column.iter().enumerate() {
				// grid origin is always in the top left
				let sprite_x = 64.0;
				let sprite_y = 64.0;
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
				.insert(SectorLabel(sector_id.0, sector_id.1));
			}
		}
	}
}
/// Spawn navigation related entities
fn setup_navigation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// create the entity handling the algorithm
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let map_length = 30; // in sprite count
	let map_depth = 30; // in sprite count
	cmds.spawn(FlowFieldTilesBundle::new_from_disk(
		map_length, map_depth, &path,
	));
	// create an actor controlled with right click
	cmds.spawn(SpriteBundle {
		texture: asset_server.load("2d_actor_sprite.png"),
		transform: Transform::from_xyz(928.0, 928.0, 1.0),
		..default()
	})
	.insert(ActorA)
	.insert(Pathing::default());
	// create an actor controlled with left click
	cmds.spawn(SpriteBundle {
		texture: asset_server.load("2d_actor_blue_sprite.png"),
		transform: Transform::from_xyz(-928.0, -928.0, 1.0),
		..default()
	})
	.insert(ActorB)
	.insert(Pathing::default());
}

/// Handle generating a PathRequest via right click
#[allow(clippy::type_complexity)]
fn user_input(
	mouse_button_input: Res<Input<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_a_q: Query<(&Transform, &mut Pathing), (With<ActorA>, Without<ActorB>)>,
	mut actor_b_q: Query<(&Transform, &mut Pathing), (With<ActorB>, Without<ActorA>)>,
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
				get_sector_and_field_id_from_xy(world_position, 30 * 64, 30 * 64, 64.0)
			{
				info!(
					"Cursor sector_id {:?}, goal_id in sector {:?}",
					target_sector_id, goal_id
				);
				for (tform, mut pathing) in actor_a_q.iter_mut() {
					let (source_sector_id, source_grid_cell) = get_sector_and_field_id_from_xy(
						tform.translation.truncate(),
						30 * 64,
						30 * 64,
						64.0,
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
					// update the Actor pathing
					pathing.source_sector = Some(source_sector_id);
					pathing.source_grid_cell = Some(source_grid_cell);
					pathing.target_sector = Some(target_sector_id);
					pathing.target_goal = Some(goal_id);
					pathing.portal_route = None;
				}
			} else {
				error!("Cursor out of bounds");
			}
		}
	}
	if mouse_button_input.just_released(MouseButton::Left) {
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
				get_sector_and_field_id_from_xy(world_position, 30 * 64, 30 * 64, 64.0)
			{
				info!(
					"Cursor sector_id {:?}, goal_id in sector {:?}",
					target_sector_id, goal_id
				);
				for (tform, mut pathing) in actor_b_q.iter_mut() {
					let (source_sector_id, source_grid_cell) = get_sector_and_field_id_from_xy(
						tform.translation.truncate(),
						30 * 64,
						30 * 64,
						64.0,
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
					// update the Actor pathing
					pathing.source_sector = Some(source_sector_id);
					pathing.source_grid_cell = Some(source_grid_cell);
					pathing.target_sector = Some(target_sector_id);
					pathing.target_goal = Some(goal_id);
					pathing.portal_route = None;
				}
			} else {
				error!("Cursor out of bounds");
			}
		}
	}
}
/// There is a delay between the Actor sending a path request and a route becoming available. This checks to see if the route is available and adds a copy to the Actor
fn actor_update_route(
	mut actor_a_q: Query<&mut Pathing, (With<ActorA>, Without<ActorB>)>,
	mut actor_b_q: Query<&mut Pathing, (With<ActorB>, Without<ActorA>)>,
	route_q: Query<&RouteCache>,
) {
	for mut pathing in actor_a_q.iter_mut() {
		if pathing.target_goal.is_some() {
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
	for mut pathing in actor_b_q.iter_mut() {
		if pathing.target_goal.is_some() {
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
}
/// Actor speed measured in pixels per fixed tick
const SPEED: f32 = 64.0;

/// If the Actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the Actor
#[allow(clippy::type_complexity)]
fn actor_steering(
	mut actor_a_q: Query<(&mut Transform, &mut Pathing), (With<ActorA>, Without<ActorB>)>,
	mut actor_b_q: Query<(&mut Transform, &mut Pathing), (With<ActorB>, Without<ActorA>)>,
	flow_cache_q: Query<&FlowFieldCache>,
) {
	let flow_cache = flow_cache_q.get_single().unwrap();
	for (mut tform, pathing) in actor_a_q.iter_mut() {
		if pathing.target_goal.is_some() {
			// lookup the overarching route
			if let Some(route) = &pathing.portal_route {
				// info!("Route: {:?}", route);
				// find the current actors postion in grid space
				let (curr_actor_sector, curr_actor_grid) = get_sector_and_field_id_from_xy(
					tform.translation.truncate(),
					30 * 64,
					30 * 64,
					64.0,
				)
				.unwrap();
				// lookup the relevant sector-goal of this sector
				'routes: for (sector, goal) in route.iter() {
					if *sector == curr_actor_sector {
						// get the flow field
						if let Some(field) = flow_cache.get_field(*sector, *goal) {
							// based on actor grid cell find the directional vector it should move in
							let cell_value =
								field.get_grid_value(curr_actor_grid.0, curr_actor_grid.1);
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
	for (mut tform, pathing) in actor_b_q.iter_mut() {
		if pathing.target_goal.is_some() {
			// lookup the overarching route
			if let Some(route) = &pathing.portal_route {
				// info!("Route: {:?}", route);
				// find the current actors postion in grid space
				let (curr_actor_sector, curr_actor_grid) = get_sector_and_field_id_from_xy(
					tform.translation.truncate(),
					30 * 64,
					30 * 64,
					64.0,
				)
				.unwrap();
				// lookup the relevant sector-goal of this sector
				'routes: for (sector, goal) in route.iter() {
					if *sector == curr_actor_sector {
						// get the flow field
						if let Some(field) = flow_cache.get_field(*sector, *goal) {
							// based on actor grid cell find the directional vector it should move in
							let cell_value =
								field.get_grid_value(curr_actor_grid.0, curr_actor_grid.1);
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
}
/// Get the asset path to sprite icons
fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}
