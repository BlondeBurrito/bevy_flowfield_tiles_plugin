//! Generates a 30x30 world and endlessly spawns actors with randomised destinations
//!

use bevy::{
	diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
	prelude::*,
};

use bevy_flowfield_tiles_plugin::prelude::*;
use avian2d::prelude::*;
use rand::seq::SliceRandom;

/// Dimension of square sprites making up the world
const FIELD_SPRITE_DIMENSION: f32 = 64.0;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			FrameTimeDiagnosticsPlugin,
			PhysicsPlugins::default(),
			// PhysicsDebugPlugin::default(),
		))
		.insert_resource(SubstepCount(30))
		.insert_resource(Gravity(Vec2::ZERO))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup, create_wall_colliders, create_counters))
		.add_systems(
			Update,
			(
				get_or_request_route,
				spawn_actors,
				despawn_at_destination,
				update_counters,
				check_if_route_exhausted,
				despawn_tunneled_actors,
			),
		)
		.add_systems(Update, actor_steering)
		.run();
}

/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;

/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[allow(dead_code)]
#[derive(Default, Component)]
struct Pathing {
	target_position: Option<Vec2>,
	metadata: Option<RouteMetadata>,
	portal_route: Option<Vec<(SectorID, FieldCell)>>,
	has_los: bool,
}

/// Used in CollisionLayers so that actors don't collide with one another, only the terrain
#[allow(clippy::missing_docs_in_private_items)]
enum Layer {
	Actor,
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

/// Spawn sprites to represent the world and the FlowFieldsBundle
fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// prepare bundle
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields_continuous_layout.ron";
	let bundle =
		FlowFieldTilesBundle::from_ron(map_length, map_depth, sector_resolution, actor_size, &path);
	// use the bundle before spawning it to help create the sprites
	let map_dimensions = bundle.get_map_dimensions();
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
	let sector_cost_fields = bundle.get_sector_cost_fields();
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
						sprite: Sprite {
							color: Color::BLACK,
							..default()
						},
						transform: Transform {
							translation: Vec3::new(x, y, 0.0),
							scale: Vec3::new(FIELD_SPRITE_DIMENSION, FIELD_SPRITE_DIMENSION, 1.0),
							..default()
						},
						..default()
					})
					.insert(Collider::rectangle(1.0, 1.0))
					.insert(RigidBody::Static)
					.insert(CollisionLayers::new([Layer::Terrain], [Layer::Actor]));
				} else {
					cmds.spawn(SpriteBundle {
						texture: asset_server.load(get_basic_icon(*value)),
						transform: Transform::from_xyz(x, y, 0.0),
						..default()
					});
				}
			}
		}
	}
	// spawn the bundle
	cmds.spawn(bundle);
}

/// Spawn an actor every tick with a random starting position at the top of the
/// map and a random destination at the bottom
fn spawn_actors(
	mut cmds: Commands,
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
		map_data.get_sector_and_field_cell_from_xy(Vec2::new(start_x, start_y))
	{
		let t_sector = SectorID::new(target_sector.0, target_sector.1);
		let t_field = FieldCell::new(target_field_cell.0, target_field_cell.1);
		let pathing = Pathing {
			target_position: Some(
				map_data
					.get_xy_from_field_sector(t_sector, t_field)
					.unwrap(),
			),
			metadata: None,
			portal_route: None,
			has_los: false,
		};
		// request a path
		event.send(EventPathRequest::new(sector_id, field, t_sector, t_field));
		// spawn the actor which cna read the path later
		cmds.spawn(SpriteBundle {
			sprite: Sprite {
				color: Color::srgb(
					230.0,
					0.0,
					255.0,
				),
				..default()
			},
			transform: Transform {
				translation: Vec3::new(start_x, start_y, 1.0),
				scale: Vec3::new(16.0, 16.0, 1.0),
				..default()
			},
			..default()
		})
		.insert(Actor)
		.insert(RigidBody::Dynamic)
		.insert(Collider::rectangle(1.0, 1.0))
		.insert(CollisionLayers::new([Layer::Actor], [Layer::Terrain]))
		.insert(pathing);
	}
}

/// If an actor has a target coordinate then obtain a route for it - if a route doesn't exist then send an event so that one is calculated
fn get_or_request_route(
	route_q: Query<(&RouteCache, &MapDimensions)>,
	mut actor_q: Query<(&Transform, &mut Pathing), With<Actor>>,
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
						if let Some((metadata, route)) = route_cahe.get_route_with_metadata(
							source_sector,
							source_field,
							target_sector,
							goal_id,
						) {
							pathing.metadata = Some(*metadata);
							pathing.portal_route = Some(route.clone());
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
const SPEED: f32 = 40000.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
fn actor_steering(
	mut actor_q: Query<(&mut LinearVelocity, &mut Transform, &mut Pathing), With<Actor>>,
	flow_cache_q: Query<(&FlowFieldCache, &MapDimensions)>,
	time_step: Res<Time>,
) {
	let (flow_cache, map_dimensions) = flow_cache_q.get_single().unwrap();
	for (mut velocity, tform, mut pathing) in actor_q.iter_mut() {
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
						if let Some(field) = flow_cache.get_field(*sector, *goal) {
							// based on actor field cell find the directional vector it should move in
							let cell_value = field.get_field_cell_value(curr_actor_field_cell);
							if has_line_of_sight(cell_value) {
								pathing.has_los = true;
								let dir =
									pathing.target_position.unwrap() - tform.translation.truncate();
								velocity.0 = dir.normalize() * SPEED * time_step.delta_seconds();
								break 'routes;
							}
							let dir = get_2d_direction_unit_vector_from_bits(cell_value);
							if dir.x == 0.0 && dir.y == 0.0 {
								warn!("Stuck");
								pathing.portal_route = None;
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

/// If an actor has drained their route then they are most likely lost due to portals changing, clear their route so they may request a fresh one
///
/// This may also happen if an actor has collided with a corner that has bounced it into a different sector
fn check_if_route_exhausted(mut actor_q: Query<(&mut Pathing, &mut LinearVelocity), With<Actor>>) {
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

/// Despawn an actor once it has reached its goal
fn despawn_at_destination(
	mut cmds: Commands,
	actors: Query<(Entity, &Pathing, &Transform), With<Actor>>,
) {
	for (entity, path, tform) in actors.iter() {
		let position = tform.translation.truncate();
		if let Some(target) = path.target_position {
			if (target - position).length_squared() < 36.0 {
				// within 6 pixels of target
				// so despawn
				cmds.entity(entity).despawn_recursive();
			}
		}
	}
}
/// If an impassable tile is placed directly on top of an actor it may achieve
/// such a high velocity from the collision that it can "tunnel" through the
/// border colliders of the world and be forever spinning through space. If an
/// actor is out-of-bounds of the world then despawn it
fn despawn_tunneled_actors(
	mut cmds: Commands,
	actor_q: Query<(Entity, &Transform), With<Actor>>,
	map: Query<&MapDimensions>,
) {
	let dimensions = map.get_single().unwrap();
	for (entity, tform) in &actor_q {
		if tform.translation.x > (dimensions.get_length() as f32 / 2.0)
			|| tform.translation.x < -(dimensions.get_length() as f32 / 2.0)
		{
			cmds.entity(entity).despawn_recursive();
		}
		if tform.translation.y > (dimensions.get_depth() as f32 / 2.0)
			|| tform.translation.y < -(dimensions.get_depth() as f32 / 2.0)
		{
			cmds.entity(entity).despawn_recursive();
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
			RigidBody::Static,
			Collider::rectangle(1.0, 1.0),
			CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
		));
	}
}

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
		let categories = vec!["FPS: ", "Actors: ", "Dur(s): ", "Gen Flows: "];
		for categroy in categories {
			p.spawn(NodeBundle::default()).with_children(|p| {
				p.spawn(TextBundle::from_sections([
					TextSection::new(
						categroy,
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
				]));
			});
		}
	});
}

/// Update the counters for FPS, number of actors, time elapased and current fields cached
fn update_counters(
	diagnostics: Res<DiagnosticsStore>,
	actors: Query<&Actor>,
	time: Res<Time>,
	cache_q: Query<&FlowFieldCache>,
	mut query: Query<&mut Text>,
) {
	for mut text in &mut query {
		match text.sections[0].value.as_str() {
			"FPS: " => {
				if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
					if let Some(val) = fps.average() {
						text.sections[1].value = format!("{val:.2}");
					}
				}
			}
			"Actors: " => {
				let mut actor_count = 0;
				for _ in actors.iter() {
					actor_count += 1;
				}
				text.sections[1].value = format!("{actor_count:.2}");
			}
			"Dur(s): " => {
				let elapsed = time.elapsed().as_secs_f32();
				text.sections[1].value = format!("{elapsed:.2}");
			}
			"Gen Flows: " => {
				let mut field_count = 0;
				for cache in &cache_q {
					field_count = cache.get().len();
				}
				text.sections[1].value = format!("{field_count:.2}");
			}
			_ => {}
		}
	}
}
