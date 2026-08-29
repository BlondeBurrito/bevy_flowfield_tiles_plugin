//! Main set of helpers for running 2d examples
//!

use avian2d::prelude::*;
use bevy::{
	prelude::*,
	tasks::{Task, futures::check_ready},
};
use bevy_flowfield_tiles_plugin::prelude::*;

/// Dimension of square sprites making up the world
#[allow(dead_code)]
pub const FIELD_SPRITE_DIMENSION: f32 = 64.0;
/// Size of a unit of space
#[allow(dead_code)]
pub const WORLD_UNIT_SIZE: f32 = 64.0;
/// Radius of an actor
#[allow(dead_code)]
pub const ACTOR_RADIUS: f32 = 16.0;

/// Used in CollisionLayers so that actors don't collide with one another, only the terrain
#[derive(Default)]
#[allow(clippy::missing_docs_in_private_items, dead_code)]
pub enum Layer {
	Actor,
	#[default]
	Terrain,
}

// Determine collision interaction
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

/// Create collider entities around the world
#[allow(dead_code)]
pub fn get_wall_colliders(
	x_length: f32,
	y_length: f32,
) -> [(Transform, RigidBody, Collider, CollisionLayers); 4] {
	[
		(
			Transform::from_translation(Vec3::new(0.0, y_length / 2.0 + 16.0, 0.0)),
			RigidBody::Static,
			Collider::rectangle(x_length, 16.0),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		),
		(
			Transform::from_translation(Vec3::new(0.0, -y_length / 2.0 - 16.0, 0.0)),
			RigidBody::Static,
			Collider::rectangle(x_length, 16.0),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		),
		(
			Transform::from_translation(Vec3::new(-x_length / 2.0 - 16.0, 0.0, 0.0)),
			RigidBody::Static,
			Collider::rectangle(16.0, y_length),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		),
		(
			Transform::from_translation(Vec3::new(x_length / 2.0 + 16.0, 0.0, 0.0)),
			RigidBody::Static,
			Collider::rectangle(16.0, y_length),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		),
	]
}

/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Default, Component)]
pub struct Pathing {
	pub target: Option<Vec2>,
	pub pollable_route: Option<Task<Option<Vec<RouteStep>>>>,
	pub route: Option<Vec<RouteStep>>,
	pub request_ticks: u32,
}

/// Request a route if an actor of `T` has a target set
#[allow(dead_code)]
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
				let task = flowfield_tiles.get_route_2d(actor_tform.translation.truncate(), target);
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
const SPEED: f32 = 20000.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
#[allow(dead_code)]
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
				let actor_pos = tform.translation.truncate();
				let Some((sector, cell)) = flowfield_tiles
					.get_dimensions()
					.get_sector_and_field_cell_from_xy(actor_pos)
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
							if let Some(dir) = field.get_2d_dir(&cell) {
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
		let position = tform.translation.truncate();
		if let Some(target) = path.target
			&& (target - position).length_squared() < 36.0
		{
			// within 6 pixels of target
			vel.0 *= 0.0;
			path.target = None;
			path.pollable_route = None;
			path.route = None;
		}
	}
}
