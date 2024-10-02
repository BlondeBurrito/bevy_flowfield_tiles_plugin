//! Generates a 30x30 world where an actor can be told to move through a narrow snake-like path
//!
//TODO visualisation creates impassable blocks when route goes back on self

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
		//TODO this system is updating sprite textures wrong, black boxes over tiles
		// .add_systems(Update, (update_sprite_visuals_based_on_actor,))
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
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
	let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	let sector_cost_fields = SectorCostFields::from_csv_dir(&map_dimensions, dir);
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
}
/// Spawn navigation related entities
fn setup_navigation(mut cmds: Commands) {
	// create the entity handling the algorithm
	let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	cmds.spawn(FlowFieldTilesBundle::from_csv(
		map_length,
		map_depth,
		sector_resolution,
		actor_size,
		&dir,
	));
	// create the controllable actor in the top right corner
	cmds.spawn(SpriteBundle {
		sprite: Sprite {
			color: Color::srgb(230.0, 0.0, 255.0),
			..default()
		},
		transform: Transform {
			translation: Vec3::new(-928.0, -900.0, 1.0),
			scale: Vec3::new(16.0, 16.0, 1.0),
			..default()
		},
		..default()
	})
	.insert(Actor)
	.insert(Pathing::default())
	.insert(RigidBody::Dynamic)
	.insert(Collider::rectangle(1.0, 1.0))
	.insert(AngularDamping(1.0))
	.insert(CollisionLayers::new([Layer::Actor], [Layer::Terrain]));
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
		let (camera, camera_transform) = camera_q.single();
		let window = windows.single();
		if let Some(world_position) = window
			.cursor_position()
			.and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
			.map(|ray| ray.origin.truncate())
		{
			let map_dimensions = dimensions_q.get_single().unwrap();
			if map_dimensions
				.get_sector_and_field_cell_from_xy(world_position)
				.is_some()
			{
				let mut pathing = actor_q.get_single_mut().unwrap();
				// update the actor pathing
				pathing.target_position = Some(world_position);
				pathing.target_sector = None;
				pathing.portal_route = None;
				pathing.has_los = false;
			} else {
				error!("Cursor out of bounds");
			}
		}
	}
}

/// Get asset path to sprite icons
fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}
