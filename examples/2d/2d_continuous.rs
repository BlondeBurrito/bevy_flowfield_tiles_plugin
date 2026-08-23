//! Generates a 30x30 world and endlessly spawns actors with randomised destinations
//!

use bevy::{
	diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
	prelude::*,
};
use bevy_flowfield_tiles_plugin::prelude::*;

use avian2d::prelude::*;

// to reduce code duplication certain constants and systems that make up
// the steering pipeline are sourced from helper modules
// NB: the steering systems are very primitive - they do the bare minimum to
// help showcase bevy_flowfield_tiles_plugin
#[path = "../helpers/camera.rs"]
mod camera;
#[path = "../helpers/cell_icons.rs"]
mod cell_icons;
#[path = "../helpers/core.rs"]
mod core;
#[path = "../helpers/core2d.rs"]
mod core2d;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			FrameTimeDiagnosticsPlugin::default(),
			PhysicsPlugins::default(),
			// PhysicsDebugPlugin::default(),
		))
		// .insert_resource(SubstepCount(6))
		.insert_resource(Gravity(Vec2::ZERO))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(
			Startup,
			(
				setup_visualisation,
				setup_navigation,
				core2d::create_wall_colliders,
				create_counters,
			),
		)
		.add_systems(
			Update,
			(
				core2d::actor_request_route::<core::Actor>,
				update_fps_counter,
				update_actor_counter,
				update_dur_counter,
				update_flow_counter,
			),
		)
		.add_systems(
			FixedUpdate,
			(
				core2d::actor_update_route::<core::Actor>,
				core2d::actor_steering::<core::Actor>,
				// check_if_route_exhausted::<core::Actor>,
				core2d::stop_at_destination::<core::Actor>,
				spawn_actors,
				despawn_at_destination,
			),
		)
		.run();
}

/// Size of the world in pixels, must be a factor of ACTOR_SIZE
const WORLD_SIZE: (f32, f32) = (1920.0, 1920.0);
/// Side length of a sector in pixels
const SECTOR_LEN: f32 = core2d::WORLD_UNIT_SIZE * FIELD_RESOLUTION as f32;

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	cmds.spawn(camera::get_camera_2d(2.0));
	let sprite_dimension = core2d::FIELD_SPRITE_DIMENSION;
	let origin = (0.0, 0.0);
	let size = WORLD_SIZE;
	let world_unit_size = core2d::WORLD_UNIT_SIZE;
	let actor_radius = core2d::ACTOR_RADIUS;
	let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields_continuous_layout.ron";
	let sector_cost_fields = SectorCostFields::from_ron(path, &dimensions);
	let fields = sector_cost_fields.get_scaled_costs();
	// iterate over each sector field to place the sprites
	for (sector_id, costfield) in fields.iter() {
		let sector_top_left = dimensions.get_sector_corner_xy(sector_id);

		for (i, cost) in costfield.get().iter().enumerate() {
			let y_i = i / FIELD_RESOLUTION;
			let x_i = i % FIELD_RESOLUTION;
			// grid origin is always in the top left
			let x = sector_top_left.x + sprite_dimension / 2.0 + (sprite_dimension * x_i as f32);
			let y = sector_top_left.y - sprite_dimension / 2.0 - (sprite_dimension * y_i as f32);
			// add colliders to impassable cells
			if *cost == 255 {
				cmds.spawn((
					Sprite {
						custom_size: Some(Vec2::new(64.0, 64.0)),
						image: asset_server.load(cell_icons::get_basic_icon(*cost)),
						..default()
					},
					Transform::from_xyz(x, y, 0.0),
				))
				.insert(Collider::rectangle(
					core2d::FIELD_SPRITE_DIMENSION,
					core2d::FIELD_SPRITE_DIMENSION,
				))
				.insert(RigidBody::Static)
				.insert(CollisionLayers::new(
					[core2d::Layer::Terrain],
					[core2d::Layer::Actor],
				));
			} else {
				cmds.spawn((
					Sprite {
						image: asset_server.load(cell_icons::get_basic_icon(*cost)),
						..default()
					},
					Transform::from_xyz(x, y, 0.0),
				));
			}
		}
	}
}

/// Spawn navigation related entities
fn setup_navigation(mut cmds: Commands) {
	// create the entity handling the algorithm
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields_continuous_layout.ron";
	let origin = (0.0, 0.0);
	let size = WORLD_SIZE;
	let world_unit_size = core2d::WORLD_UNIT_SIZE;
	let actor_radius = core2d::ACTOR_RADIUS;
	cmds.spawn(FlowFieldTiles::from_ron(
		origin,
		size,
		world_unit_size,
		actor_radius,
		&path,
	));
}

/// Spawn an actor every tick with a random starting position at the top of the
/// map and a random destination at the bottom
fn spawn_actors(mut cmds: Commands) {
	let sector_column_max = (WORLD_SIZE.0 / SECTOR_LEN) as usize;
	let sector_row_max = (WORLD_SIZE.1 / SECTOR_LEN) as usize;
	// pick a start
	let starting_sector_column = rand::random_range(0..sector_column_max);
	let starting_cell_column = rand::random_range(0..10);
	let starting_sector = (starting_sector_column, 0);
	let starting_field = (starting_cell_column, 0);
	let start_y = (WORLD_SIZE.1 / 2.0) - core2d::WORLD_UNIT_SIZE / 2.0;
	let start_x = (-WORLD_SIZE.0 / 2.0)
		+ core2d::WORLD_UNIT_SIZE / 2.0
		+ (starting_sector.0 as f32 * SECTOR_LEN)
		+ (starting_field.0 as f32 * core2d::WORLD_UNIT_SIZE);

	// pick an end
	let target_sector_column = rand::random_range(0..sector_column_max);
	let target_cell_column = rand::random_range(0..10);
	let target_sector = (target_sector_column, sector_row_max - 1);
	let target_field_cell = (target_cell_column, 9);
	let target_y = (-WORLD_SIZE.1 / 2.0) + core2d::WORLD_UNIT_SIZE / 2.0;
	let target_x = ((-WORLD_SIZE.0 / 2.0)
		+ core2d::WORLD_UNIT_SIZE / 2.0
		+ (target_sector.0 as f32 * SECTOR_LEN))
		+ (target_field_cell.0 as f32 * core2d::WORLD_UNIT_SIZE);

	cmds.spawn((
		Sprite {
			color: Color::srgb(230.0, 0.0, 255.0),
			..default()
		},
		Transform {
			translation: Vec3::new(start_x, start_y, 1.0),
			scale: Vec3::new(16.0, 16.0, 1.0),
			..default()
		},
	))
	.insert(core::Actor)
	.insert(core2d::Pathing {
		target: Some(Vec2::new(target_x, target_y)),
		pollable_route: None,
		route: None,
	})
	.insert(RigidBody::Dynamic)
	.insert(Collider::circle(1.0))
	.insert(AngularDamping(1.0))
	.insert(CollisionLayers::new(
		[core2d::Layer::Actor],
		[core2d::Layer::Terrain],
	));
}

/// Despawn an actor once it has reached its goal
fn despawn_at_destination(
	mut cmds: Commands,
	actors: Query<(Entity, &core2d::Pathing), With<core::Actor>>,
) {
	for (entity, path) in actors.iter() {
		// system from helper core2d removes target value on arrival,
		// so when it is equal to None they are at the goal and done
		if path.target.is_none() {
			cmds.entity(entity).despawn();
		}
	}
}
// /// If an impassable tile is placed directly on top of an actor it may achieve
// /// such a high velocity from the collision that it can "tunnel" through the
// /// border colliders of the world and be forever spinning through space. If an
// /// actor is out-of-bounds of the world then despawn it
// fn despawn_tunneled_actors(
// 	mut cmds: Commands,
// 	actor_q: Query<(Entity, &Transform), With<Actor>>,
// 	map: Query<&MapDimensions>,
// ) {
// 	let dimensions = map.single().unwrap();
// 	for (entity, tform) in &actor_q {
// 		if tform.translation.x > (dimensions.get_length() as f32 / 2.0)
// 			|| tform.translation.x < -(dimensions.get_length() as f32 / 2.0)
// 		{
// 			cmds.entity(entity).despawn();
// 		}
// 		if tform.translation.y > (dimensions.get_depth() as f32 / 2.0)
// 			|| tform.translation.y < -(dimensions.get_depth() as f32 / 2.0)
// 		{
// 			cmds.entity(entity).despawn();
// 		}
// 	}
// }

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
	})
	.with_children(|p| {
		p.spawn(Node::default()).with_children(|p| {
			p.spawn((
				Text::new("FPS: "),
				TextFont {
					font: FontSource::Monospace,
					font_size: FontSize::Px(30.0),
					..default()
				},
				TextColor(Color::WHITE),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Monospace,
					font_size: FontSize::Px(30.0),
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
					font: FontSource::Monospace,
					font_size: FontSize::Px(30.0),
					..default()
				},
				TextColor(Color::WHITE),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Monospace,
					font_size: FontSize::Px(30.0),
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
					font: FontSource::Monospace,
					font_size: FontSize::Px(30.0),
					..default()
				},
				TextColor(Color::WHITE),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Monospace,
					font_size: FontSize::Px(30.0),
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
					font: FontSource::Monospace,
					font_size: FontSize::Px(30.0),
					..default()
				},
				TextColor(Color::WHITE),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Monospace,
					font_size: FontSize::Px(30.0),
					..default()
				},
				TextColor(Color::WHITE),
				FlowCounter,
			));
		});
	});
}

/// Update the FPS counter
fn update_fps_counter(
	diagnostics: Res<DiagnosticsStore>,
	mut query: Query<&mut TextSpan, With<FpsCounter>>,
) {
	for mut text in &mut query {
		if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
			&& let Some(val) = fps.smoothed()
		{
			**text = format!("{val:.2}");
		}
	}
}
/// Update the actor count counter
fn update_actor_counter(
	actors: Query<&core::Actor>,
	mut query: Query<&mut TextSpan, With<ActorCounter>>,
) {
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
fn update_flow_counter(
	flow_q: Query<&FlowFieldTiles>,
	mut query: Query<&mut TextSpan, With<FlowCounter>>,
) {
	for mut text in &mut query {
		let mut field_count = 0;
		for flowfield_tiles in &flow_q {
			field_count = flowfield_tiles.flowfield_cache().get_cache().len();
		}
		**text = format!("{field_count:.2}");
	}
}
