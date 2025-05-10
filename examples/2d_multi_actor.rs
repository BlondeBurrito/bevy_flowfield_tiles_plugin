//! Generates a 30x30 world where multiple actors can be told to move soomewhere with right click
//!

use avian2d::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;
use examples_utils::_2d::{
	actor_steering, check_if_route_exhausted, create_wall_colliders, get_or_request_route,
	stop_at_destination, Layer, Pathing, FIELD_SPRITE_DIMENSION,
};

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			PhysicsPlugins::default(),
			// PhysicsDebugPlugin::default(),
		))
		.insert_resource(SubstepCount(30))
		.insert_resource(Gravity(Vec2::ZERO))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(
			Startup,
			(setup_visualisation, setup_navigation, create_wall_colliders),
		)
		.add_systems(Update, (user_input, get_or_request_route::<Actor>))
		.add_systems(
			Update,
			(
				actor_steering::<Actor>,
				check_if_route_exhausted::<Actor>,
				stop_at_destination::<Actor>,
			),
		)
		.run();
}

/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
	let proj = Projection::Orthographic(OrthographicProjection {
		scale: 2.0,
		..OrthographicProjection::default_2d()
	});
	cmds.spawn((Camera2d, proj));
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
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
}
/// Spawn navigation related entities
fn setup_navigation(mut cmds: Commands) {
	// create the entity handling the algorithm
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
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
	// create the controllable actors
	let actor_positions = [
		Vec3::new(928.0, 928.0, 1.0),
		Vec3::new(864.0, 928.0, 1.0),
		Vec3::new(800.0, 928.0, 1.0),
		Vec3::new(-928.0, 928.0, 1.0),
		Vec3::new(-864.0, 928.0, 1.0),
		Vec3::new(-800.0, 928.0, 1.0),
		Vec3::new(-928.0, -928.0, 1.0),
		Vec3::new(-864.0, -928.0, 1.0),
		Vec3::new(-928.0, -864.0, 1.0),
		Vec3::new(928.0, -928.0, 1.0),
		Vec3::new(864.0, -928.0, 1.0),
		Vec3::new(928.0, -864.0, 1.0),
	];
	for pos in actor_positions.iter() {
		cmds.spawn((
			Sprite {
				color: Color::srgb(230.0, 0.0, 255.0),
				..default()
			},
			Transform {
				translation: Vec3::new(pos.x, pos.y, pos.z),
				scale: Vec3::new(16.0, 16.0, 1.0),
				..default()
			},
		))
		.insert(Actor)
		.insert(Pathing::default())
		.insert(RigidBody::Dynamic)
		.insert(Collider::rectangle(1.0, 1.0))
		.insert(AngularDamping(1.0))
		.insert(CollisionLayers::new([Layer::Actor], [Layer::Terrain]));
	}
}

/// Handle generating a PathRequest via right click
fn user_input(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	dimensions_q: Query<&MapDimensions>,
	mut actor_q: Query<&mut Pathing, With<Actor>>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 2d world positionn of cursor
		let (camera, camera_transform) = camera_q.single().unwrap();
		let window = windows.single().unwrap();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position)
		else {
			return;
		};
		let map_dimensions = dimensions_q.single().unwrap();
		if map_dimensions
			.get_sector_and_field_cell_from_xy(world_position)
			.is_some()
		{
			for mut pathing in actor_q.iter_mut() {
				// update the actor pathing
				pathing.target_position = Some(world_position);
				pathing.target_sector = None;
				pathing.portal_route = None;
				pathing.has_los = false;
			}
		} else {
			error!("Cursor out of bounds");
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
