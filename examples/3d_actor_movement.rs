//! Loads a 3d model with an actor represented by a blue sphere which can be moved with right click
//!

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

/// Spawn the map
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let mut camera = Camera3dBundle::default();
	camera.transform.translation = Vec3::new(0.0, 40.0, 10.0);
	camera.transform.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
	cmds.spawn(camera);
	cmds.spawn(SceneBundle {
		scene: asset_server.load("3d/3d_map.gltf#Scene0"),
		..default()
	});
	cmds.spawn(PointLightBundle {
		point_light: PointLight {
			intensity: 9000.0,
			range: 100.,
			shadows_enabled: true,
			..default()
		},
		transform: Transform::from_xyz(0.0, 50.0, 0.0),
		..default()
	});
}

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
		&path,
	));
	// create the controllable actor in the top right corner
	let mesh = meshes.add(
		Mesh::try_from(shape::Icosphere {
			radius: 0.5,
			subdivisions: 32,
		})
		.unwrap(),
	);
	let material = materials.add(Color::BLUE.into());
	cmds.spawn(PbrBundle {
		mesh,
		material,
		transform: Transform::from_xyz(14.5, 1.0, -14.5),
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
	dimensions_q: Query<&MapDimensions>,
	mut actor_q: Query<(&Transform, &mut Pathing), With<Actor>>,
	mut event: EventWriter<EventPathRequest>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 3d world positionn of cursor
		let (camera, camera_transform) = camera_q.single();
		let window = windows.single();
		let ray_point = window
			.cursor_position()
			.and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
			.map(|ray| {
				ray.intersect_plane(Vec3::ZERO, Vec3::Y)
					.map(|distance| ray.get_point(distance))
			});
		if let Some(op_world_position) = ray_point {
			let map_dimensions = dimensions_q.get_single().unwrap();
			let world_position = op_world_position.unwrap();
			info!("World cursor position: {:?}", world_position);
			if let Some((target_sector_id, goal_id)) =
				map_dimensions.get_sector_and_field_cell_from_xyz(world_position)
			{
				info!(
					"Cursor sector_id {:?}, goal_id in sector {:?}",
					target_sector_id, goal_id
				);
				let (tform, mut pathing) = actor_q.get_single_mut().unwrap();
				let (source_sector_id, source_field_cell) = map_dimensions
					.get_sector_and_field_cell_from_xyz(tform.translation)
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
			pathing.target_sector.unwrap(),
			pathing.target_goal.unwrap(),
		) {
			pathing.portal_route = Some(route.clone());
		}
	}
}
/// Actor speed measured in pixels per fixed tick
const SPEED: f32 = 1.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
fn actor_steering(
	mut actor_q: Query<(&mut Transform, &mut Pathing), With<Actor>>,
	flow_cache_q: Query<(&FlowFieldCache, &MapDimensions)>,
) {
	let (mut tform, mut pathing) = actor_q.get_single_mut().unwrap();
	let (flow_cache, map_dimensions) = flow_cache_q.get_single().unwrap();

	if pathing.target_goal.is_some() {
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
					if let Some(field) = flow_cache.get_field(*sector, *goal) {
						// based on actor field cell find the directional vector it should move in
						let cell_value = field.get_field_cell_value(curr_actor_field_cell);
						let dir = get_3d_direction_unit_vector_from_bits(cell_value);
						// info!("In sector {:?}, in field cell {:?}", sector, curr_actor_field_cell);
						// info!("Direction to move: {}", dir);
						let velocity = dir * SPEED;
						// move the actor based on the velocity
						tform.translation += velocity;
					}
					break 'routes;
				}
			}
		}
	}
}
