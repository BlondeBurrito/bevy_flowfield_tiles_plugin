//! Loads a 3d model with an actor represented by a blue sphere which can be moved with right click
//!

use std::time::Duration;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;
/// Timestep of actor movement system
const ACTOR_TIMESTEP: f32 = 0.25;

/// Length`x` of the world
const MAP_LENGTH: u32 = 30;
/// Depth `z` of the world
const MAP_DPETH: u32 = 30;
/// Factor of sectors to create
const SECTOR_RESOLUTION: u32 = 10;
/// Size of the actor perpendicular to its forward direction
const ACTOR_SIZE: f32 = 0.5;
fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(
			ACTOR_TIMESTEP,
		)))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup_visualisation, setup_navigation))
		.add_systems(Update, (user_input, actor_update_route))
		.add_systems(FixedUpdate, (actor_steering, apply_velocity).chain())
		.run();
}

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
	target_position: Option<Vec3>,
	target_sector: Option<SectorID>,
	target_goal: Option<FieldCell>,
	portal_route: Option<Vec<(SectorID, FieldCell)>>,
	has_los: bool,
}

/// Spawn the map
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let mut tform = Transform::from_translation(Vec3::new(0.0, 40.0, 10.0));
	tform.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
	cmds.spawn((Camera3d::default(), tform));
	cmds.spawn(SceneRoot(asset_server.load("3d/3d_map.gltf#Scene0")));
	cmds.spawn((
		Transform::from_xyz(0.0, 50.0, 0.0),
		PointLight {
			intensity: 9000.0,
			range: 100.,
			shadows_enabled: true,
			..default()
		},
	));
}

/// Dir and magnitude of actor movement
#[derive(Component, Default)]
struct Velocity(Vec3);

/// Spawn navigation related entities
fn setup_navigation(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	// create the entity handling the algorithm
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	cmds.spawn(FlowFieldTilesBundle::from_ron(
		MAP_LENGTH,
		MAP_DPETH,
		SECTOR_RESOLUTION,
		ACTOR_SIZE,
		&path,
	));
	// create the controllable actor in the top right corner
	let mesh = meshes.add(Mesh::from(bevy::math::primitives::Sphere { radius: 0.5 }));
	let material = materials.add(StandardMaterial {
		base_color: Color::Srgba(Srgba::BLUE),
		..default()
	});
	cmds.spawn((
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(14.5, 1.0, -14.5),
	))
	.insert(Actor)
	.insert(Velocity::default())
	.insert(Pathing::default());
}

/// Handle generating a PathRequest via right click
fn user_input(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	dimensions_q: Query<&MapDimensions>,
	mut actor_q: Query<(&Transform, &mut Pathing), With<Actor>>,
	mut event: EventWriter<EventPathRequest>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 3d world positionn of cursor
		let (camera, camera_transform) = camera_q.single().unwrap();
		let window = windows.single().unwrap();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(ray_3d) = camera.viewport_to_world(camera_transform, cursor_position) else {
			return;
		};
		if let Some(world_position) = ray_3d
			.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
			.map(|distance| ray_3d.get_point(distance))
		{
			let map_dimensions = dimensions_q.single().unwrap();
			info!("World cursor position: {:?}", world_position);
			if let Some((target_sector_id, goal_id)) =
				map_dimensions.get_sector_and_field_cell_from_xyz(world_position)
			{
				info!(
					"Cursor sector_id {:?}, goal_id in sector {:?}",
					target_sector_id, goal_id
				);
				let (tform, mut pathing) = actor_q.single_mut().unwrap();
				let (source_sector_id, source_field_cell) = map_dimensions
					.get_sector_and_field_cell_from_xyz(tform.translation)
					.unwrap();
				info!(
					"Actor sector_id {:?}, goal_id in sector {:?}",
					source_sector_id, source_field_cell
				);
				event.write(EventPathRequest::new(
					source_sector_id,
					source_field_cell,
					target_sector_id,
					goal_id,
				));
				// update the actor pathing
				pathing.source_sector = Some(source_sector_id);
				pathing.source_field_cell = Some(source_field_cell);
				pathing.target_position = Some(world_position);
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
	let mut pathing = actor_q.single_mut().unwrap();
	if pathing.target_goal.is_some() && pathing.portal_route.is_none() {
		let route_cache = route_q.single().unwrap();
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
/// Actor speed measured in pixels per fixed tick
const SPEED: f32 = 1.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
fn actor_steering(
	mut actor_q: Query<(&mut Velocity, &mut Transform, &mut Pathing), With<Actor>>,
	flow_cache_q: Query<(&FlowFieldCache, &MapDimensions)>,
) {
	let (mut velocity, tform, mut pathing) = actor_q.single_mut().unwrap();
	let (flow_cache, map_dimensions) = flow_cache_q.single().unwrap();

	if pathing.target_goal.is_some() {
		let op_target_sector = pathing.target_sector;
		// lookup the overarching route
		if let Some(route) = pathing.portal_route.as_mut() {
			// info!("Route: {:?}", route);
			// find the current actors postion in grid space
			let (curr_actor_sector, curr_actor_field_cell) = map_dimensions
				.get_sector_and_field_cell_from_xyz(tform.translation)
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
					if let Some(field) =
						flow_cache.get_field(*sector, op_target_sector.unwrap(), *goal)
					{
						// based on actor field cell find the directional vector it should move in
						let cell_value = field.get_field_cell_value(curr_actor_field_cell);
						if has_line_of_sight(cell_value) {
							pathing.has_los = true;
							let mut dir = pathing.target_position.unwrap() - tform.translation;
							dir.y = 0.0;
							velocity.0 = dir.normalize() * SPEED;
							break 'routes;
						}
						let dir = get_3d_direction_unit_vector_from_bits(cell_value);
						// info!("In sector {:?}, in field cell {:?}", sector, curr_actor_field_cell);
						// info!("Direction to move: {}", dir);
						velocity.0 = dir * SPEED;
					}
					break 'routes;
				}
			}
		}
	}
}

/// Move the actor
fn apply_velocity(mut actor_q: Query<(&Velocity, &mut Transform), With<Actor>>) {
	for (velocity, mut tform) in actor_q.iter_mut() {
		tform.translation += velocity.0;
	}
}
