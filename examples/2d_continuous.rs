//! Generates a 30x30 world and endlessly spawns actors with randomised destinations
//!

use bevy::{
	diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
	prelude::*,
};
use examples_utils::_2d::{
	actor_steering, check_if_route_exhausted, create_wall_colliders, get_or_request_route, Layer,
	Pathing, FIELD_SPRITE_DIMENSION,
};

use avian2d::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;
use rand::seq::IndexedRandom;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			FrameTimeDiagnosticsPlugin::default(),
			PhysicsPlugins::default(),
			// PhysicsDebugPlugin::default(),
		))
		.insert_resource(SubstepCount(6))
		.insert_resource(Gravity(Vec2::ZERO))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup, create_wall_colliders, create_counters))
		.add_systems(
			Update,
			(
				get_or_request_route::<Actor>,
				spawn_actors,
				despawn_at_destination,
				check_if_route_exhausted::<Actor>,
				despawn_tunneled_actors,
				update_fps_counter,
				update_actor_counter,
				update_dur_counter,
				update_flow_counter,
			),
		)
		.add_systems(Update, actor_steering::<Actor>)
		.run();
}

/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;

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
	let proj = Projection::Orthographic(OrthographicProjection {
		scale: 2.0,
		..OrthographicProjection::default_2d()
	});
	cmds.spawn((Camera2d, proj));
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
					cmds.spawn((
						Sprite {
							color: Color::BLACK,
							..default()
						},
						Transform {
							translation: Vec3::new(x, y, 0.0),
							scale: Vec3::new(FIELD_SPRITE_DIMENSION, FIELD_SPRITE_DIMENSION, 1.0),
							..default()
						},
					))
					.insert(Collider::rectangle(1.0, 1.0))
					.insert(RigidBody::Static)
					.insert(CollisionLayers::new([Layer::Terrain], [Layer::Actor]));
				} else {
					cmds.spawn((
						Sprite {
							image: asset_server.load(get_basic_icon(*value)),
							..default()
						},
						Transform::from_xyz(x, y, 0.0),
					));
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
	let starting_sector = starting_sectors.choose(&mut rand::rng()).unwrap();
	let starting_field = starting_field_cells.choose(&mut rand::rng()).unwrap();
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
	let target_sector = target_sectors.choose(&mut rand::rng()).unwrap();
	let target_field_cell = target_field_cells.choose(&mut rand::rng()).unwrap();

	let map_data = map.single().unwrap();
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
		event.write(EventPathRequest::new(sector_id, field, t_sector, t_field));
		// spawn the actor which cna read the path later
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
		.insert(Actor)
		.insert(RigidBody::Dynamic)
		.insert(Collider::circle(1.0))
		.insert(AngularDamping(1.0))
		.insert(CollisionLayers::new([Layer::Actor], [Layer::Terrain]))
		.insert(pathing);
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
				cmds.entity(entity).despawn();
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
	let dimensions = map.single().unwrap();
	for (entity, tform) in &actor_q {
		if tform.translation.x > (dimensions.get_length() as f32 / 2.0)
			|| tform.translation.x < -(dimensions.get_length() as f32 / 2.0)
		{
			cmds.entity(entity).despawn();
		}
		if tform.translation.y > (dimensions.get_depth() as f32 / 2.0)
			|| tform.translation.y < -(dimensions.get_depth() as f32 / 2.0)
		{
			cmds.entity(entity).despawn();
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
					font_size: 30.0,
					..default()
				},
				TextColor(Color::WHITE),
			))
			.with_child((
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
			))
			.with_child((
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
			))
			.with_child((
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
			))
			.with_child((
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
fn update_fps_counter(
	diagnostics: Res<DiagnosticsStore>,
	mut query: Query<&mut TextSpan, With<FpsCounter>>,
) {
	for mut text in &mut query {
		if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
			if let Some(val) = fps.smoothed() {
				**text = format!("{val:.2}");
			}
		}
	}
}
/// Update the actor count counter
fn update_actor_counter(
	actors: Query<&Actor>,
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
	cache_q: Query<&FlowFieldCache>,
	mut query: Query<&mut TextSpan, With<FlowCounter>>,
) {
	for mut text in &mut query {
		let mut field_count = 0;
		for cache in &cache_q {
			field_count = cache.get().len();
		}
		**text = format!("{field_count:.2}");
	}
}
