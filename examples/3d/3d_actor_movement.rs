//! Loads a 3d model with an actor represented by a blue sphere which can be moved with right click
//!

use bevy::{
	prelude::*,
	tasks::{Task, futures::check_ready},
	window::PrimaryWindow,
};
use bevy_flowfield_tiles_plugin::prelude::*;
use std::time::Duration;

// to reduce code duplication certain constants and systems that make up
// the steering pipeline are sourced from helper modules
// NB: the steering systems are very primitive - they do the bare minimum to
// help showcase bevy_flowfield_tiles_plugin
#[path = "../helpers/camera.rs"]
mod camera;
#[path = "../helpers/core.rs"]
mod core;

/// Timestep of actor movement system
const ACTOR_TIMESTEP: f32 = 0.25;

/// Size of the actor perpendicular to its forward direction
const ACTOR_RADIUS: f32 = 0.5;
fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(
			ACTOR_TIMESTEP,
		)))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup_visualisation, setup_navigation))
		.add_systems(PreUpdate, click_set_target)
		.add_systems(Update, actor_request_route)
		.add_systems(
			FixedUpdate,
			(
				actor_update_route,
				actor_steering,
				stop_at_destination,
				apply_velocity,
			),
		)
		.run();
}

/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Default, Component)]
struct Pathing {
	pub target: Option<Vec3>,
	pub pollable_route: Option<Task<Option<Vec<RouteStep>>>>,
	pub route: Option<Vec<RouteStep>>,
}

/// Spawn the map
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	cmds.spawn(camera::get_camera_3d());
	cmds.spawn(WorldAssetRoot(
		asset_server.load(GltfAssetLabel::Scene(0).from_asset("3d/3d_map.gltf")),
	));
	cmds.spawn((
		Transform::from_xyz(0.0, 50.0, 0.0),
		PointLight {
			intensity: 9000.0,
			range: 100.,
			shadow_maps_enabled: true,
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
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields.ron";
	cmds.spawn(FlowFieldTiles::from_ron(
		(0.0, 0.0),
		(30.0, 30.0),
		1.0,
		ACTOR_RADIUS,
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
	.insert(core::Actor)
	.insert(Velocity::default())
	.insert(Pathing::default());
}

/// Handle user mouse clicks
fn click_set_target(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_q: Query<&mut Pathing, With<core::Actor>>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 2d world position of cursor
		let (camera, camera_transform) = camera_q.single().unwrap();
		let window = windows.single().unwrap();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(world_position) = camera.viewport_to_world(camera_transform, cursor_position) else {
			return;
		};
		if let Some(intersect) = world_position.plane_intersection_point(
			Vec3::new(0.0, 0.0, 0.0),
			InfinitePlane3d { normal: Dir3::Y },
		) {
			// set the actor target and abandon any existing pollable task
			let mut actor_pathing = actor_q.single_mut().unwrap();
			let existing_route = &mut actor_pathing.pollable_route;
			// if let Some(poll) = existing_route {
			// 	let _ = poll.detach();
			// }
			*existing_route = None;
			actor_pathing.target = Some(intersect);
			actor_pathing.route = None;
		}
	}
}

/// Request a route if an actor of `T` has a target set
fn actor_request_route(
	mut actor_q: Query<(&Transform, &mut Pathing), With<core::Actor>>,
	flow_q: Query<&FlowFieldTiles>,
) {
	// get the actor position
	for (actor_tform, mut actor_pathing) in &mut actor_q {
		if let Some(target) = actor_pathing.target {
			if actor_pathing.route.is_none() && actor_pathing.pollable_route.is_none() {
				// ask for a route
				for flowfield_tiles in &flow_q {
					let task = flowfield_tiles.get_route_3d(actor_tform.translation, target);
					if let Some(t) = task {
						actor_pathing.pollable_route = Some(t);
						actor_pathing.route = None;
					}
				}
			}
		}
	}
}

/// There is a delay between the actor sending a path request and a route
/// becoming available. This checks to see if the route is available
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<core::Actor>>) {
	for mut pathing in &mut actor_q {
		if let Some(mut poll) = pathing.pollable_route.as_mut() {
			if let Some(route) = check_ready(&mut poll) {
				// task finished
				pathing.pollable_route = None;
				pathing.route = route;
			}
		}
	}
}

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
#[cfg(not(tarpaulin_include))]
fn actor_steering(
	mut actor_q: Query<(&mut Velocity, &mut Transform, &mut Pathing), With<core::Actor>>,
	flow_q: Query<&FlowFieldTiles>,
	time_step: Res<Time>,
) {
	let flowfield_tiles = flow_q.single().unwrap();
	for (mut velocity, tform, mut pathing) in actor_q.iter_mut() {
		if let Some(steps) = &mut pathing.route {
			if let Some(step) = steps.first() {
				// get actor position in terms of sector and cell
				let actor_pos = tform.translation;
				let Some((sector, cell)) = flowfield_tiles
					.get_dimensions()
					.get_sector_and_field_cell_from_xyz(actor_pos)
				else {
					// actor is out of bounds of Dimensions, do something about it...
					continue;
				};
				if *step.get_sector() == sector {
					if let Some(field) = flowfield_tiles.read_flowfield(step) {
						if field.has_los(&cell) {
							// has LOS can move straight to goal
							let dir = (pathing.target.unwrap() - actor_pos).normalize();
							velocity.0 = dir * SPEED * time_step.delta_secs();
						} else {
							if let Some(dir) = field.get_3d_dir(&cell) {
								// move along the flow
								// velocity.0 = dir * SPEED * time_step.delta_secs();
								velocity.0 = dir * SPEED;
							}
						}
					} else {
						//TODO count ticks to re-request entire route?
					}
				} else {
					// actor is not in the sector denoted by the RouteStep
					// schedule first step removal
					steps.remove(0); //TODO VecDeque
				}
			} else {
				// steps is empty meaning it is exhausted. This might happen if an actor
				// has a collision and is knocked into a sector not along their path.
				// The actor should prepare to request a new route
				// Setting route to None while pathing.target is still set will cause
				// a new request for a route to be sent in a different system
				pathing.route = None;
			}
		}
	}
}

/// Actor speed measured in pixels per fixed tick
const SPEED: f32 = 1.0;

/// Move the actor
fn apply_velocity(mut actor_q: Query<(&Velocity, &mut Transform), With<core::Actor>>) {
	for (velocity, mut tform) in actor_q.iter_mut() {
		tform.translation += velocity.0;
	}
}

/// Stop an actor once it has reached its goal
fn stop_at_destination(
	mut actors: Query<(&mut Velocity, &mut Pathing, &Transform), With<core::Actor>>,
) {
	for (mut vel, mut path, tform) in &mut actors {
		let position = tform.translation;
		if let Some(target) = path.target {
			if (target - position).length_squared() < 0.5 {
				vel.0 *= 0.0;
				path.target = None;
				path.pollable_route = None;
				path.route = None;
			}
		}
	}
}
