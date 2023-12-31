//! Generates a 30x30 world and endlessly spawns actors with randomised destinations
//!

use std::time::Duration;

use bevy::{
	diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
	prelude::*,
	sprite::collide_aabb::{collide, Collision},
};
use bevy_flowfield_tiles_plugin::prelude::*;
use rand::seq::SliceRandom;

/// Timestep of actor movement system
const ACTOR_TIMESTEP: f32 = 1.0 / 60.0;
/// Dimension of square sprites making up the world
const FIELD_SPRITE_DIMENSION: f32 = 64.0;

fn main() {
	App::new()
		.add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin))
		.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(
			ACTOR_TIMESTEP,
		)))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(
			Startup,
			(
				setup_visualisation,
				setup_navigation,
				create_wall_colliders,
				create_counters,
			),
		)
		.add_systems(
			Update,
			(
				actor_update_route,
				spawn_actors,
				despawn_at_destination,
				update_fps_counter,
				update_actor_counter,
			),
		)
		.add_systems(
			FixedUpdate,
			(actor_steering, collision_detection, apply_velocity).chain(),
		)
		.run();
}

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct SectorLabel(u32, u32);

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct FieldCellLabel(usize, usize);

/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;

/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[allow(dead_code)]
#[derive(Default, Component)]
struct Pathing {
	source_sector: Option<SectorID>,
	source_field_cell: Option<FieldCell>,
	target_position: Option<Vec2>,
	target_sector: Option<SectorID>,
	target_goal: Option<FieldCell>,
	portal_route: Option<Vec<(SectorID, FieldCell)>>,
	current_direction: Option<Vec2>,
	/// Helps to steer the actor around corners when it is very close to an impassable field cell and reduces the likihood on tunneling
	previous_direction: Option<Vec2>,
	has_los: bool,
}
/// Dir and magnitude of actor movement
#[derive(Component, Default)]
struct Velocity(Vec2);

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields_continuous_layout.ron";
	let sector_cost_fields = SectorCostFields::from_ron(path, &map_dimensions);
	let fields = sector_cost_fields.get_baseline();
	// iterate over each sector field to place the sprites
	for (sector_id, field) in fields.iter() {
		// iterate over the dimensions of the field
		for (i, column) in field.get().iter().enumerate() {
			for (j, value) in column.iter().enumerate() {
				// grid origin is always in the top left
				let sector_offset = map_dimensions.get_sector_corner_xy(*sector_id);
				let x = sector_offset.x + 32.0 + (FIELD_SPRITE_DIMENSION * i as f32);
				let y = sector_offset.y - 32.0 - (FIELD_SPRITE_DIMENSION * j as f32);
				// add colliders to impassable cells
				if *value == 255 {
					cmds.spawn(SpriteBundle {
						texture: asset_server.load(get_basic_icon(*value)),
						transform: Transform::from_xyz(x, y, 0.0),
						..default()
					})
					.insert(FieldCellLabel(i, j))
					.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()))
					.with_children(|p| {
						// required to have a collider sized correctly
						p.spawn(SpatialBundle {
							transform: Transform::from_scale(Vec3::new(
								FIELD_SPRITE_DIMENSION,
								FIELD_SPRITE_DIMENSION,
								1.0,
							)),
							..default()
						})
						.insert(Collider);
					});
				} else {
					cmds.spawn(SpriteBundle {
						texture: asset_server.load(get_basic_icon(*value)),
						transform: Transform::from_xyz(x, y, 0.0),
						..default()
					})
					.insert(FieldCellLabel(i, j))
					.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()));
				}
			}
		}
	}
}

/// Spawn navigation related entities
fn setup_navigation(mut cmds: Commands) {
	// create the entity handling the algorithm
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields_continuous_layout.ron";
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	cmds.spawn(FlowFieldTilesBundle::from_ron(
		map_length,
		map_depth,
		sector_resolution,
		actor_size,
		&path,
	));
}

/// Spawn an actor every tick with a random starting position at the top of the
/// map and a random destination at the bottom
fn spawn_actors(
	mut cmds: Commands,
	asset_server: Res<AssetServer>,
	map: Query<&MapDimensions>,
	mut event: EventWriter<EventPathRequest>,
) {
	// pick a start
	let starting_sectors = [(0, 0), (1, 0), (2, 0)];
	let starting_field_cells = [
		(0, 0),
		(1, 0),
		(2, 0),
		(3, 0),
		(4, 0),
		(5, 0),
		(6, 0),
		(7, 0),
		(8, 0),
		(9, 0),
	];
	let starting_sector = starting_sectors.choose(&mut rand::thread_rng()).unwrap();
	let starting_field = starting_field_cells
		.choose(&mut rand::thread_rng())
		.unwrap();
	let start_y = 928.0;
	let start_x = ((-928 + starting_sector.0 * 640) + (starting_field.0 * 64)) as f32;

	// pick an end
	let target_sectors = [(0, 2), (1, 2), (2, 2)];
	let target_field_cells = [
		(0, 9),
		(1, 9),
		(2, 9),
		(3, 9),
		(4, 9),
		(5, 9),
		(6, 9),
		(7, 9),
		(8, 9),
		(9, 9),
	];
	let target_sector = target_sectors.choose(&mut rand::thread_rng()).unwrap();
	let target_field_cell = target_field_cells.choose(&mut rand::thread_rng()).unwrap();

	let map_data = map.get_single().unwrap();
	if let Some((sector_id, field)) =
		map_data.get_sector_and_field_id_from_xy(Vec2::new(start_x, start_y))
	{
		let t_sector = SectorID::new(target_sector.0, target_sector.1);
		let t_field = FieldCell::new(target_field_cell.0, target_field_cell.1);
		let pathing = Pathing {
			source_sector: Some(sector_id),
			source_field_cell: Some(field),
			target_position: Some(
				map_data
					.get_xy_from_field_sector(t_sector, t_field)
					.unwrap(),
			),
			target_sector: Some(t_sector),
			target_goal: Some(t_field),
			portal_route: None,
			current_direction: None,
			previous_direction: None,
			has_los: false,
		};
		// request a path
		event.send(EventPathRequest::new(sector_id, field, t_sector, t_field));
		// spawn the actor which cna read the path later
		cmds.spawn(SpriteBundle {
			texture: asset_server.load("2d/2d_actor_sprite.png"),
			transform: Transform::from_xyz(start_x, start_y, 1.0),
			..default()
		})
		.insert(Actor)
		.insert(Velocity::default())
		.insert(pathing)
		.with_children(|p| {
			p.spawn(SpatialBundle {
				transform: Transform::from_scale(Vec3::new(16.0, 16.0, 1.0)),
				..default()
			});
		});
	}
}

/// There is a delay between the actor sending a path request and a route becoming available. This checks to see if the route is available and adds a copy to the actor
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<Actor>>, route_q: Query<&RouteCache>) {
	for mut pathing in actor_q.iter_mut() {
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
}
/// Actor speed measured in pixels per fixed tick
const SPEED: f32 = 250.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
fn actor_steering(
	mut actor_q: Query<(&mut Velocity, &mut Transform, &mut Pathing), With<Actor>>,
	flow_cache_q: Query<(&FlowFieldCache, &MapDimensions)>,
	time_step: Res<Time>,
) {
	let (flow_cache, map_dimensions) = flow_cache_q.get_single().unwrap();
	for (mut velocity, tform, mut pathing) in actor_q.iter_mut() {
		if pathing.target_goal.is_some() {
			// lookup the overarching route
			if let Some(route) = pathing.portal_route.as_mut() {
				// find the current actors postion in grid space
				let (curr_actor_sector, curr_actor_field_cell) = map_dimensions
					.get_sector_and_field_id_from_xy(tform.translation.truncate())
					.unwrap();
				// tirm the actor stored route as it makes progress
				// this ensures it doesn't use a previous goal from
				// a sector it has already been through when it needs
				// to pass through it again as part of a different part of the route
				if let Some(f) = route.first() {
					if curr_actor_sector != f.0 {
						// route.remove(0);
					}
				}
				// lookup the relevant sector-goal of this sector
				'routes: for (sector, goal) in route.iter() {
					if *sector == curr_actor_sector {
						// get the flow field
						if let Some(field) = flow_cache.get_field(*sector, *goal) {
							// based on actor field cell find the directional vector it should move in
							let cell_value = field.get_field_cell_value(curr_actor_field_cell);
							if has_line_of_sight(cell_value) {
								pathing.has_los = true;
								let dir =
									pathing.target_position.unwrap() - tform.translation.truncate();
								velocity.0 = dir.normalize() * SPEED * time_step.delta_seconds();
								// pathing.previous_direction = Some(-dir.normalize());
								break 'routes;
							}
							let dir = get_2d_direction_unit_vector_from_bits(cell_value);
							if pathing.current_direction.is_none() {
								pathing.current_direction = Some(dir);
							} else if pathing.current_direction.unwrap() != dir {
								pathing.previous_direction = pathing.current_direction;
								pathing.current_direction = Some(dir);
							}
							velocity.0 = dir * SPEED * time_step.delta_seconds();
						}
						break 'routes;
					}
				}
			}
		}
	}
}

/// Move the actor
fn apply_velocity(mut actor_q: Query<(&Velocity, &mut Transform), With<Actor>>) {
	for (velocity, mut tform) in actor_q.iter_mut() {
		tform.translation += velocity.0.extend(0.0);
	}
}

/// Despawn an actor once it has reached its goal
fn despawn_at_destination(
	mut cmds: Commands,
	actors: Query<(Entity, &Pathing, &Transform), With<Actor>>,
	map: Query<&MapDimensions>,
) {
	for (entity, path, tform) in actors.iter() {
		// get actors current sector and field
		let map_data = map.get_single().unwrap();
		if let Some((current_sector, current_field)) =
			map_data.get_sector_and_field_id_from_xy(tform.translation.truncate())
		{
			// if its reached its destination despawn it
			if let Some(target_sector) = path.target_sector {
				if let Some(target_goal) = path.target_goal {
					if current_sector == target_sector && current_field == target_goal {
						cmds.entity(entity).despawn_recursive();
					}
				}
			}
		}
	}
}
/// Get asset path of sprite icons
fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}

/// Added to entities that should block actors
#[derive(Component)]
struct Collider;

/// Create collider entities around the world
fn create_wall_colliders(mut cmds: Commands) {
	let top_location = Vec3::new(0.0, FIELD_SPRITE_DIMENSION * 15.0, 0.0);
	let top_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION * 30.0,
		FIELD_SPRITE_DIMENSION / 2.0,
		1.0,
	);
	let bottom_location = Vec3::new(0.0, -FIELD_SPRITE_DIMENSION * 15.0, 0.0);
	let bottom_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION * 30.0,
		FIELD_SPRITE_DIMENSION / 2.0,
		1.0,
	);
	let left_location = Vec3::new(-FIELD_SPRITE_DIMENSION * 15.0, 0.0, 0.0);
	let left_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION / 2.0,
		FIELD_SPRITE_DIMENSION * 30.0,
		1.0,
	);
	let right_location = Vec3::new(FIELD_SPRITE_DIMENSION * 15.0, 0.0, 0.0);
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
			SpriteBundle {
				transform: Transform {
					translation: *loc,
					scale: *scale,
					..default()
				},
				sprite: Sprite {
					color: Color::BLACK,
					..default()
				},
				..default()
			},
			Collider,
		));
	}
}

/// Rebound actors when they begin to overlap an impassable area
fn collision_detection(
	mut actor_q: Query<(&mut Velocity, &Transform, &Children, &Pathing), With<Actor>>,
	actor_child_q: Query<&Transform>,
	static_colliders: Query<(&Parent, &Transform), With<Collider>>,
	parent_colliders: Query<&Transform>,
	time_step: Res<Time>,
) {
	for (mut velocity, actor_tform, children, pathing) in actor_q.iter_mut() {
		for (parent, child_collider_tform) in static_colliders.iter() {
			let parent_collider_tform = parent_colliders.get(parent.get()).unwrap();
			for &child in children {
				let tform = actor_child_q.get(child).unwrap();
				let collision = collide(
					actor_tform.translation,
					tform.scale.truncate(),
					parent_collider_tform.translation,
					child_collider_tform.scale.truncate(),
				);
				if let Some(collision) = collision {
					// direct the actor away from the collider
					match collision {
						Collision::Left => {
							velocity.0.x *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0.y = dir.y * SPEED * time_step.delta_seconds() * 2.0;
							}
						}
						Collision::Right => {
							velocity.0.x *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0.y = dir.y * SPEED * time_step.delta_seconds() * 2.0;
							}
						}
						Collision::Top => {
							velocity.0.y *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0.x = dir.x * SPEED * time_step.delta_seconds() * 2.0;
							}
						}
						Collision::Bottom => {
							velocity.0.y *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0.x = dir.x * SPEED * time_step.delta_seconds() * 2.0;
							}
						}
						Collision::Inside => {
							// velocity.0 *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0 = dir * SPEED * time_step.delta_seconds() * 3.0;
							}
						}
					}
				}
			}
		}
	}
}

/// Label the FPS counter
#[derive(Component)]
struct FPSCounter;

/// Label the FPS counter
#[derive(Component)]
struct ActorCounter;

/// Create UI counters to measure the FPS and number of actors
fn create_counters(mut cmds: Commands) {
	cmds.spawn(NodeBundle {
		style: Style {
			flex_direction: FlexDirection::Column,
			..default()
		},
		..default()
	})
	.with_children(|p| {
		p.spawn(NodeBundle::default()).with_children(|p| {
			p.spawn(TextBundle::from_sections([
				TextSection::new(
					"FPS: ",
					TextStyle {
						font_size: 30.0,
						color: Color::WHITE,
						..default()
					},
				),
				TextSection::from_style(TextStyle {
					font_size: 30.0,
					color: Color::WHITE,
					..default()
				}),
			]))
			.insert(FPSCounter);
		});
		p.spawn(NodeBundle::default()).with_children(|p| {
			p.spawn(TextBundle::from_sections([
				TextSection::new(
					"Actors: ",
					TextStyle {
						font_size: 30.0,
						color: Color::WHITE,
						..default()
					},
				),
				TextSection::from_style(TextStyle {
					font_size: 30.0,
					color: Color::WHITE,
					..default()
				}),
			]))
			.insert(ActorCounter);
		});
	});
}

/// Updates the FPS field ech tick
fn update_fps_counter(
	diagnostics: Res<DiagnosticsStore>,
	mut query: Query<&mut Text, (With<FPSCounter>, Without<ActorCounter>)>,
) {
	let mut text = query.single_mut();
	if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
		if let Some(val) = fps.average() {
			text.sections[1].value = format!("{val:.2}");
		}
	}
}

/// Updates the FPS field ech tick
fn update_actor_counter(
	actors: Query<&Actor>,
	mut query: Query<&mut Text, (With<ActorCounter>, Without<FPSCounter>)>,
) {
	let mut text = query.single_mut();
	let mut actor_count = 0;
	for _ in actors.iter() {
		actor_count += 1;
	}
	text.sections[1].value = format!("{actor_count:.2}");
}
