//! Helpers used in 2d examples
//!

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;

/// Dimension of square sprites making up the world
pub const FIELD_SPRITE_DIMENSION: f32 = 64.0;

/// Used in CollisionLayers so that actors don't collide with one another, only the terrain
#[derive(Default)]
#[allow(clippy::missing_docs_in_private_items)]
pub enum Layer {
	Actor,
	#[default]
	Terrain,
}

// weird bug when using #derive where it thinks the crate bevy_xpbd_3d is being used >(
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
pub fn create_wall_colliders(mut cmds: Commands) {
	let top_location = Vec3::new(0.0, FIELD_SPRITE_DIMENSION * 15.0, 1.0);
	let top_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION * 30.0,
		FIELD_SPRITE_DIMENSION / 2.0,
		1.0,
	);
	let bottom_location = Vec3::new(0.0, -FIELD_SPRITE_DIMENSION * 15.0, 1.0);
	let bottom_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION * 30.0,
		FIELD_SPRITE_DIMENSION / 2.0,
		1.0,
	);
	let left_location = Vec3::new(-FIELD_SPRITE_DIMENSION * 15.0, 0.0, 1.0);
	let left_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION / 2.0,
		FIELD_SPRITE_DIMENSION * 30.0,
		1.0,
	);
	let right_location = Vec3::new(FIELD_SPRITE_DIMENSION * 15.0, 0.0, 1.0);
	let right_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION / 2.0,
		FIELD_SPRITE_DIMENSION * 30.0,
		1.0,
	);

	let walls = [
		(top_location, top_scale),
		(bottom_location, bottom_scale),
		(left_location, left_scale),
		(right_location, right_scale),
	];

	for (loc, scale) in walls.iter() {
		cmds.spawn((
			Sprite {
				color: Color::BLACK,
					..default()
			},
			Transform{
				translation: *loc,
					scale: *scale,
					..default()
			},
			RigidBody::Static,
			Collider::rectangle(1.0, 1.0),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		));
	}
}

/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[allow(dead_code)]
#[derive(Default, Component)]
pub struct Pathing {
	pub target_position: Option<Vec2>,
	pub target_sector: Option<SectorID>,
	pub portal_route: Option<Vec<(SectorID, FieldCell)>>,
	pub has_los: bool,
}

/// If an actor has a target coordinate then obtain a route for it - if a route doesn't exist then send an event so that one is calculated
pub fn get_or_request_route<T: Component>(
	route_q: Query<(&RouteCache, &MapDimensions)>,
	mut actor_q: Query<(&Transform, &mut Pathing), With<T>>,
	mut event: EventWriter<EventPathRequest>,
) {
	let (route_cahe, map_dimensions) = route_q.get_single().unwrap();
	for (tform, mut pathing) in &mut actor_q {
		if let Some(target) = pathing.target_position {
			// actor has no route, look one up or request one
			if pathing.portal_route.is_none() {
				if let Some((source_sector, source_field)) =
					map_dimensions.get_sector_and_field_cell_from_xy(tform.translation.truncate())
				{
					if let Some((target_sector, goal_id)) =
						map_dimensions.get_sector_and_field_cell_from_xy(target)
					{
						// if a route is calculated get it
						if let Some(route) = route_cahe.get_route(
							source_sector,
							source_field,
							target_sector,
							goal_id,
						) {
							pathing.target_sector = Some(target_sector);
							pathing.portal_route = Some(route.get().clone());
						} else {
							// request a route
							event.send(EventPathRequest::new(
								source_sector,
								source_field,
								target_sector,
								goal_id,
							));
						}
					}
				}
			}
		}
	}
}

/// Actor speed
const SPEED: f32 = 30000.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
pub fn actor_steering<T: Component>(
	mut actor_q: Query<(&mut LinearVelocity, &mut Transform, &mut Pathing), With<T>>,
	flow_cache_q: Query<(&FlowFieldCache, &MapDimensions)>,
	time_step: Res<Time>,
) {
	let (flow_cache, map_dimensions) = flow_cache_q.get_single().unwrap();
	for (mut velocity, tform, mut pathing) in actor_q.iter_mut() {
		let op_target_sector = pathing.target_sector;
		// lookup the overarching route
		if let Some(route) = pathing.portal_route.as_mut() {
			// find the current actors postion in grid space
			if let Some((curr_actor_sector, curr_actor_field_cell)) =
				map_dimensions.get_sector_and_field_cell_from_xy(tform.translation.truncate())
			{
				// trim the actor stored route as it makes progress
				// this ensures it doesn't use a previous goal from
				// a sector it has already been through when it needs
				// to pass through it again as part of a different part of the route
				if let Some(f) = route.first() {
					if curr_actor_sector != f.0 {
						route.remove(0);
					}
				}
				// lookup the relevant sector-goal of this sector
				'routes: for (sector, goal) in route.iter() {
					if *sector == curr_actor_sector {
						// get the flow field
						if let Some(target_sector) = op_target_sector {
							if let Some(field) = flow_cache.get_field(*sector, target_sector, *goal)
							{
								// based on actor field cell find the directional vector it should move in
								let cell_value = field.get_field_cell_value(curr_actor_field_cell);
								if has_line_of_sight(cell_value) {
									pathing.has_los = true;
									let dir = pathing.target_position.unwrap()
										- tform.translation.truncate();
									velocity.0 =
										dir.normalize() * SPEED * time_step.delta_secs();
									break 'routes;
								}
								let dir = get_2d_direction_unit_vector_from_bits(cell_value);
								if dir.x == 0.0 && dir.y == 0.0 {
									warn!("Stuck");
									pathing.portal_route = None;
								}
								velocity.0 = dir * SPEED * time_step.delta_secs();
							}
						}
						break 'routes;
					}
				}
			}
		}
	}
}

/// Stop an actor once it has reached its goal
pub fn stop_at_destination<T: Component>(
	mut actors: Query<(&mut LinearVelocity, &mut Pathing, &Transform), With<T>>,
) {
	for (mut vel, mut path, tform) in &mut actors {
		let position = tform.translation.truncate();
		if let Some(target) = path.target_position {
			if (target - position).length_squared() < 36.0 {
				// within 6 pixels of target
				// so despawn
				vel.0 *= 0.0;
				path.target_position = None;
				path.target_sector = None;
				path.portal_route = None;
			}
		}
	}
}

/// If an actor has drained their route then they are most likely lost due to portals changing, clear their route so they may request a fresh one
///
/// This may also happen if an actor has collided with a corner that has bounced it into a different sector
pub fn check_if_route_exhausted<T: Component>(
	mut actor_q: Query<(&mut Pathing, &mut LinearVelocity), With<T>>,
) {
	for (mut pathing, mut vel) in &mut actor_q {
		if let Some(route) = &pathing.portal_route {
			if route.is_empty() {
				// actor has exhuasted it's route, it's lost, clear route so a new one can be requested
				warn!("Exhausted route, a new one will be requested, has an actor had a collision knocking into a different sector?");
				vel.0 *= 0.0;
				pathing.portal_route = None;
			}
		}
	}
}
