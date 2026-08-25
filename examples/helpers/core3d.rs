//! Main set of helpers for running 3d examples
//!

use avian3d::prelude::*;
use bevy::{
	prelude::*,
	tasks::{Task, futures::check_ready},
};
use bevy_flowfield_tiles_plugin::prelude::*;

/// Timestep of actor movement system
pub const ACTOR_TIMESTEP: f32 = 1.0 / 64.0;

/// Size of a unit of space
#[allow(dead_code)]
pub const WORLD_UNIT_SIZE: f32 = 1.0;
/// Radius of an actor
#[allow(dead_code)]
pub const ACTOR_RADIUS: f32 = 0.3;
/// Height of an actor
#[allow(dead_code)]
pub const ACTOR_HEIGHT: f32 = 0.6;

/// Used in CollisionLayers so that actors don't collide with one another, only the terrain
#[derive(Default)]
#[allow(clippy::missing_docs_in_private_items, dead_code)]
pub enum Layer {
	Actor,
	#[default]
	Terrain,
}

// Determine collision interaction
#[cfg(not(tarpaulin_include))]
impl PhysicsLayer for Layer {
	fn to_bits(&self) -> u32 {
		match self {
			Layer::Actor => 1,
			Layer::Terrain => 2,
		}
	}

	fn all_bits() -> u32 {
		0b11
	}
}

/// Get light source bundle
#[cfg(not(tarpaulin_include))]
pub fn get_light() -> (DirectionalLight, Transform) {
	(
		DirectionalLight {
			illuminance: light_consts::lux::OVERCAST_DAY,
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_xyz(0.0, 15.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
	)
}

/// Create collider entities around the world
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub fn get_wall_colliders(
	x_length: f32,
	z_length: f32,
) -> [(Transform, RigidBody, Collider, CollisionLayers); 4] {
	[
		(
			Transform::from_translation(Vec3::new(0.0, ACTOR_HEIGHT / 2.0, -z_length / 2.0 - 0.5)),
			RigidBody::Static,
			Collider::cuboid(x_length, ACTOR_HEIGHT, 0.5),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		),
		(
			Transform::from_translation(Vec3::new(0.0, ACTOR_HEIGHT / 2.0, z_length / 2.0 + 0.5)),
			RigidBody::Static,
			Collider::cuboid(x_length, ACTOR_HEIGHT, 0.5),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		),
		(
			Transform::from_translation(Vec3::new(-x_length / 2.0 - 0.5, ACTOR_HEIGHT / 2.0, 0.0)),
			RigidBody::Static,
			Collider::cuboid(0.5, ACTOR_HEIGHT, z_length),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		),
		(
			Transform::from_translation(Vec3::new(x_length / 2.0 + 0.5, ACTOR_HEIGHT / 2.0, 0.0)),
			RigidBody::Static,
			Collider::cuboid(0.5, ACTOR_HEIGHT, z_length),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		),
	]
}

/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Default, Component)]
pub struct Pathing {
	pub target: Option<Vec3>,
	pub pollable_route: Option<Task<Option<Vec<RouteStep>>>>,
	pub route: Option<Vec<RouteStep>>,
	pub request_ticks: u32,
}

/// Request a route if an actor of `T` has a target set
pub fn actor_request_route<T: Component>(
	mut actor_q: Query<(&Transform, &mut Pathing), With<T>>,
	flow_q: Query<&FlowFieldTiles>,
) {
	// get the actor position
	for (actor_tform, mut actor_pathing) in &mut actor_q {
		if let Some(target) = actor_pathing.target
			&& actor_pathing.route.is_none()
			&& actor_pathing.pollable_route.is_none()
		{
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

/// There is a delay between the actor sending a path request and a route
/// becoming available. This checks to see if the route is available
#[allow(dead_code)]
pub fn actor_update_route<T: Component>(mut actor_q: Query<&mut Pathing, With<T>>) {
	for mut pathing in &mut actor_q {
		if let Some(mut poll) = pathing.pollable_route.as_mut()
			&& let Some(route) = check_ready(&mut poll)
		{
			// task finished
			pathing.pollable_route = None;
			pathing.route = route;
		}
	}
}

/// Actor speed
#[allow(dead_code)]
const SPEED: f32 = 300.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub fn actor_steering<T: Component>(
	mut actor_q: Query<(&mut LinearVelocity, &mut Transform, &mut Pathing), With<T>>,
	flow_q: Query<&FlowFieldTiles>,
	time_step: Res<Time>,
) {
	let flowfield_tiles = flow_q.single().unwrap();
	for (mut velocity, tform, mut pathing) in actor_q.iter_mut() {
		// only proceed for actors with a route
		if let Some(steps) = &mut pathing.route {
			if let Some(step) = steps.first() {
				// get actor position in terms of sector and cell
				let actor_pos = tform.translation;
				let Some((sector, cell)) = flowfield_tiles
					.get_dimensions()
					.get_sector_and_field_cell_from_xyz(actor_pos)
				else {
					// actor is out of bounds of Dimensions, do something about it...
					warn!("Actor is out of bounds");
					continue;
				};
				if *step.get_sector() == sector {
					// attempt to get the FlowField, the field is built inside of
					// an AsyncTaskPool so it may take a moment for it to become
					// available
					if let Some(field) = flowfield_tiles.read_flowfield(step) {
						if field.has_los(&cell) {
							// has LOS can move straight to goal
							let dir = (pathing.target.unwrap() - actor_pos).normalize();
							velocity.0 = dir * SPEED * time_step.delta_secs();
						} else {
							if let Some(dir) = field.get_3d_dir(&cell) {
								// move along the flow
								velocity.0 = dir * SPEED * time_step.delta_secs();
							}
						}
					} else {
						// if a costfield has been changed then the RouteStep may no longer
						// be valid, meaning no FlowField will be generated for it.
						// count ticks and if too many remove route so a new request
						// will be sent
						pathing.request_ticks += 1;
						if pathing.request_ticks > 300 {
							pathing.request_ticks = 0;
							pathing.pollable_route = None;
							pathing.route = None;
						}
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

/// Stop an actor once it has reached its goal
#[allow(dead_code)]
pub fn stop_at_destination<T: Component>(
	mut actors: Query<(&mut LinearVelocity, &mut Pathing, &Transform), With<T>>,
) {
	for (mut vel, mut path, tform) in &mut actors {
		let position = tform.translation.with_y(0.0);
		if let Some(target) = path.target
			&& (target - position).length_squared() < 0.25
		{
			vel.0 *= 0.0;
			path.target = None;
			path.pollable_route = None;
			path.route = None;
		}
	}
}
