//! Generates a 30x30 world and endlessly spawns actors with randomised destinations.
//!
//! You can use your LeftMouseButton to flip Costfield values between 1 and 255
//!

use bevy::{
	diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
	prelude::*,
	window::PrimaryWindow,
};

use avian2d::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;
use examples_utils::_2d::{
	actor_steering, check_if_route_exhausted, create_wall_colliders, get_or_request_route, Layer,
	Pathing, FIELD_SPRITE_DIMENSION,
};
use rand::seq::SliceRandom;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			FrameTimeDiagnosticsPlugin,
			PhysicsPlugins::default(),
			// PhysicsDebugPlugin::default(),
		))
		.insert_resource(SubstepCount(6))
		.insert_resource(Gravity(Vec2::ZERO))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup, create_wall_colliders, create_counters))
		.add_systems(PreUpdate, click_update_cost)
		// .insert_resource(Time::<Fixed>::from_seconds(0.1))
		// .add_systems(FixedUpdate, spawn_actors)
		.add_systems(
			Update,
			(
				spawn_actors,
				get_or_request_route::<Actor>,
				check_if_route_exhausted::<Actor>,
				despawn_at_destination,
				actor_steering::<Actor>,
				update_fps_counter,
				update_actor_counter,
				update_dur_counter,
				update_flow_counter
			),
		)
		.add_systems(PostUpdate, despawn_tunneled_actors)
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

/// Spawn sprites to represent the world and the FlowFieldsBundle
fn setup(mut cmds: Commands) {
	// prepare bundle
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	let bundle = FlowFieldTilesBundle::new(map_length, map_depth, sector_resolution, actor_size);
	// use the bundle before spawning it to help create the sprites
	let map_dimensions = bundle.get_map_dimensions();
	let mut proj = OrthographicProjection::default_2d();
	proj.scale = 2.0;
	cmds.spawn((
		Camera2d,
		proj
	));
	let sector_cost_fields = bundle.get_sector_cost_fields();
	let fields = sector_cost_fields.get_baseline();
	// iterate over each sector field to place the sprites
	for (sector_id, field) in fields.iter() {
		// iterate over the dimensions of the field
		for (i, column) in field.get().iter().enumerate() {
			for (j, _value) in column.iter().enumerate() {
				// grid origin is always in the top left
				let sector_offset = map_dimensions.get_sector_corner_xy(*sector_id);
				let x = sector_offset.x + 32.0 + (FIELD_SPRITE_DIMENSION * i as f32);
				let y = sector_offset.y - 32.0 - (FIELD_SPRITE_DIMENSION * j as f32);
				// start with sprites for everying being pathable
				cmds.spawn((Sprite {
						color: Color::WHITE,
						..default()
					},
					Transform {
						translation: Vec3::new(x, y, 0.0),
						scale: Vec3::new(FIELD_SPRITE_DIMENSION, FIELD_SPRITE_DIMENSION, 1.0),
						..default()
					},)
				)
				.insert(FieldCellLabel(i, j))
				.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()));
			}
		}
	}
	// spawn the bundle
	cmds.spawn(bundle);
}

/// Left clicking on a tile/field will flip the value of it in the [CostField]
///
/// If the current cost is `1` then it is updated to `255` and a [Collider] is inserted denoting an impassable field.
///
/// If the current cost is `255` then
fn click_update_cost(
	mut cmds: Commands,
	mut tile_q: Query<(Entity, &SectorLabel, &FieldCellLabel, &mut Sprite)>,
	input: Res<ButtonInput<MouseButton>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	windows: Query<&Window, With<PrimaryWindow>>,
	dimensions_q: Query<(&MapDimensions, &SectorCostFields)>,
	mut event: EventWriter<EventUpdateCostfieldsCell>,
) {
	if input.just_released(MouseButton::Left) {
		let (camera, camera_transform) = camera_q.single();
		let window = windows.single();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
			return;
		};
			let (map_dimensions, cost_fields) = dimensions_q.get_single().unwrap();
			if let Some((sector_id, field_cell)) =
				map_dimensions.get_sector_and_field_cell_from_xy(world_position)
			{
				let cost_field = cost_fields.get_baseline().get(&sector_id).unwrap();
				let value = cost_field.get_field_cell_value(field_cell);
				if value == 255 {
					let e = EventUpdateCostfieldsCell::new(field_cell, sector_id, 1);
					event.send(e);
					// remove collider from tile
					for (entity, sector_label, field_label, mut sprite) in &mut tile_q {
						if (sector_label.0, sector_label.1) == sector_id.get()
							&& (field_label.0, field_label.1) == field_cell.get_column_row()
						{
							sprite.color = Color::WHITE;
							cmds.entity(entity).remove::<Collider>();
							cmds.entity(entity).remove::<RigidBody>();
							cmds.entity(entity).remove::<CollisionLayers>();
						}
					}
				} else {
					let e = EventUpdateCostfieldsCell::new(field_cell, sector_id, 255);
					event.send(e);
					// add collider to tile
					for (entity, sector_label, field_label, mut sprite) in &mut tile_q {
						if (sector_label.0, sector_label.1) == sector_id.get()
							&& (field_label.0, field_label.1) == field_cell.get_column_row()
						{
							sprite.color = Color::BLACK;
							cmds.entity(entity).insert((
								Collider::rectangle(1.0, 1.0),
								RigidBody::Static,
								CollisionLayers::new([Layer::Terrain], [Layer::Actor]),
							));
						}
					}
				}
			}
	}
}

/// Spawn an actor every tick with a random starting position at the top of the
/// map and a random destination at the bottom
fn spawn_actors(
	mut cmds: Commands,
	map: Query<&MapDimensions>,
	mut event: EventWriter<EventPathRequest>,
	actors_q: Query<&Actor>,
) {
	let mut a_count = 0;
	for _ in &actors_q {
		a_count += 1;
	}
	if a_count > 1500 {
		return;
	}
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
			target_sector: None,
			portal_route: None,
			has_los: false,
		};
		// request a path
		event.send(EventPathRequest::new(sector_id, field, t_sector, t_field));
		// spawn the actor which can read the path later
		cmds.spawn((Sprite {
				color: Color::srgb(230.0, 0.0, 255.0),
				..default()
			},
			Transform {
				translation: Vec3::new(start_x, start_y, 1.0),
				scale: Vec3::new(16.0, 16.0, 1.0),
				..default()
			},)
		)
		.insert(Actor)
		.insert(RigidBody::Dynamic)
		.insert(Collider::circle(1.0))
		.insert(CollisionLayers::new([Layer::Actor], [Layer::Terrain]))
		.insert(AngularDamping(1.0))
		.insert(pathing);
	}
}

// /// Every route is timestamped, when routes are recalculated they will have new timestamps.
// ///
// /// Compare the timestamp of a route an actor has stored with what's in the
// /// cache and clear it if it's old so that a new route can be requested
// fn check_if_route_is_old(
// 	route_q: Query<&RouteCache, Changed<RouteCache>>,
// 	mut actor_q: Query<&mut Pathing, With<Actor>>,
// ) {
// 	let cache = route_q.get_single().unwrap();
// 	for mut pathing in &mut actor_q {
// 		if let Some(metadata) = pathing.metadata {
// 			if let Some((cache_metadata, _route)) = cache.get_routes().get_key_value(&metadata) {
// 				if metadata.get_time_generated() != cache_metadata.get_time_generated() {
// 					// cached route is newer meaning fields have been rebuilt
// 					// reset cached pathing so a new route can be requested
// 					pathing.target_sector = None;
// 					pathing.portal_route = None;
// 				}
// 			}
// 		}
// 	}
// }

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
/// Labels FPS text
#[derive(Component)]
struct FpsCounter;
/// Labels actor counter text
#[derive(Component)]
struct ActorCounter;
/// Labels time elapsed text
#[derive(Component)]

struct DurationCounter;
/// Labels number of flows generated text
#[derive(Component)]
struct FlowCounter;

/// Create UI counters to measure the FPS and number of actors
fn create_counters(mut cmds: Commands) {
	cmds.spawn(Node {
			flex_direction: FlexDirection::Column,
			..default()
		},
	)
	.with_children(|p| {
		p.spawn(Node::default()).with_children(|p| {
			p.spawn((
				Text::new("FPS: "),
				TextFont {
					font_size: 30.0,
					..default()
				},
				TextColor(Color::WHITE),
			)).with_child((
				TextSpan::default(),
				TextFont {
					font_size: 30.0,
					..default()
				},
				TextColor(Color::WHITE),
				FpsCounter,
			));
		});
	p.spawn(Node::default()).with_children(|p| {
		p.spawn((
			Text::new("Actors: "),
			TextFont {
				font_size: 30.0,
				..default()
			},
			TextColor(Color::WHITE),
		)).with_child((
			TextSpan::default(),
			TextFont {
				font_size: 30.0,
				..default()
			},
			TextColor(Color::WHITE),
			ActorCounter,
		));
	});
	p.spawn(Node::default()).with_children(|p| {
		p.spawn((
			Text::new("Dur(s): "),
			TextFont {
				font_size: 30.0,
				..default()
			},
			TextColor(Color::WHITE),
		)).with_child((
			TextSpan::default(),
			TextFont {
				font_size: 30.0,
				..default()
			},
			TextColor(Color::WHITE),
			DurationCounter,
		));
	});
	p.spawn(Node::default()).with_children(|p| {
		p.spawn((
			Text::new("Gen Flows: "),
			TextFont {
				font_size: 30.0,
				..default()
			},
			TextColor(Color::WHITE),
		)).with_child((
			TextSpan::default(),
			TextFont {
				font_size: 30.0,
				..default()
			},
			TextColor(Color::WHITE),
			FlowCounter,
		));
	});
	});
}

/// Update the FPS counter
fn update_fps_counter(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut TextSpan, With<FpsCounter>>) {
	for mut text in &mut query {
		if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
			if let Some(val) = fps.smoothed() {
				**text = format!("{val:.2}");
			}
		}
	}
}
/// Update the actor count counter
fn update_actor_counter(actors: Query<&Actor>, mut query: Query<&mut TextSpan, With<ActorCounter>>) {
	for mut text in &mut query {
		let mut actor_count = 0;
				for _ in actors.iter() {
					actor_count += 1;
				}
				**text = format!("{actor_count:.2}");
	}
}
/// Update the counter for how long the simulation has been running
fn update_dur_counter(time: Res<Time>, mut query: Query<&mut TextSpan, With<DurationCounter>>) {
	for mut text in &mut query {
		let elapsed = time.elapsed().as_secs_f32();
				**text = format!("{elapsed:.2}");
	}
}
/// Update the counter for the number of flow fields generated
fn update_flow_counter(cache_q: Query<&FlowFieldCache>, mut query: Query<&mut TextSpan, With<FlowCounter>>) {
	for mut text in &mut query {
		let mut field_count = 0;
				for cache in &cache_q {
					field_count = cache.get().len();
				}
				**text = format!("{field_count:.2}");
	}
}