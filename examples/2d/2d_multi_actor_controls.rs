//! Generates a 30x30 world where multiple Actors can be told to move somewhere
//! with right click and left click
//!

use avian2d::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;

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
			PhysicsPlugins::default(),
			// PhysicsDebugPlugin::default(),
		))
		// .insert_resource(SubstepCount(30))
		.insert_resource(Gravity(Vec2::ZERO))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(
			Startup,
			(
				setup_visualisation,
				setup_navigation,
				core2d::create_wall_colliders,
			),
		)
		.add_systems(
			PreUpdate,
			(click_set_target_actor_a, click_set_target_actor_b),
		)
		.add_systems(
			Update,
			(
				core2d::actor_request_route::<core::ActorA>,
				core2d::actor_request_route::<core::ActorB>,
			),
		)
		.add_systems(
			FixedUpdate,
			(
				core2d::actor_update_route::<core::ActorA>,
				core2d::actor_steering::<core::ActorA>,
				core2d::stop_at_destination::<core::ActorA>,
				core2d::actor_update_route::<core::ActorB>,
				core2d::actor_steering::<core::ActorB>,
				core2d::stop_at_destination::<core::ActorB>,
			),
		)
		.run();
}

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	cmds.spawn(camera::get_camera_2d(2.0));
	let sprite_dimension = core2d::FIELD_SPRITE_DIMENSION;
	let origin = (0.0, 0.0);
	let size = (1920.0, 1920.0);
	let world_unit_size = core2d::WORLD_UNIT_SIZE;
	let actor_radius = core2d::ACTOR_RADIUS;
	let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);

	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields.ron";
	let sector_costs = SectorCostFields::from_ron(path, &dimensions);
	let fields = sector_costs.get_scaled_costs();
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
						color: Color::BLACK,
						custom_size: Some(Vec2::splat(core2d::FIELD_SPRITE_DIMENSION)),
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
	// create flowfield tiles
	let origin = (0.0, 0.0);
	let size = (1920.0, 1920.0);
	let world_unit_size = core2d::WORLD_UNIT_SIZE;
	let actor_radius = core2d::ACTOR_RADIUS;
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields.ron";
	cmds.spawn(FlowFieldTiles::from_ron(
		origin,
		size,
		world_unit_size,
		actor_radius,
		&path,
	));
	// create an actor controlled with right click
	cmds.spawn((
		Sprite {
			color: Color::srgb(230.0, 0.0, 255.0),
			..default()
		},
		Transform {
			translation: Vec3::new(928.0, 920.0, 1.0),
			scale: Vec3::new(16.0, 16.0, 1.0),
			..default()
		},
	))
	.insert(core::ActorA)
	.insert(core2d::Pathing::default())
	.insert(RigidBody::Dynamic)
	.insert(Collider::rectangle(1.0, 1.0))
	.insert(AngularDamping(1.0))
	.insert(CollisionLayers::new(
		[core2d::Layer::Actor],
		[core2d::Layer::Terrain],
	));
	// create an actor controlled with left click
	cmds.spawn((
		Sprite {
			color: Color::srgb(0.0, 230.0, 255.0),
			..default()
		},
		Transform {
			translation: Vec3::new(-928.0, -920.0, 1.0),
			scale: Vec3::new(16.0, 16.0, 1.0),
			..default()
		},
	))
	.insert(core::ActorB)
	.insert(core2d::Pathing::default())
	.insert(RigidBody::Dynamic)
	.insert(Collider::rectangle(1.0, 1.0))
	.insert(AngularDamping(1.0))
	.insert(CollisionLayers::new(
		[core2d::Layer::Actor],
		[core2d::Layer::Terrain],
	));
}

/// Handle user mouse clicks
fn click_set_target_actor_a(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_q: Query<&mut core2d::Pathing, With<core::ActorA>>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 2d world position of cursor
		let (camera, camera_transform) = camera_q.single().unwrap();
		let window = windows.single().unwrap();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position)
		else {
			return;
		};
		// set the actor target and abandon any existing pollable task
		for mut actor_pathing in &mut actor_q {
			let existing_route = &mut actor_pathing.pollable_route;
			// if let Some(poll) = existing_route {
			// 	let _ = poll.detach();
			// }
			*existing_route = None;
			actor_pathing.target = Some(world_position);
			actor_pathing.route = None;
		}
	}
}

/// Handle user mouse clicks
fn click_set_target_actor_b(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_q: Query<&mut core2d::Pathing, With<core::ActorB>>,
) {
	if mouse_button_input.just_released(MouseButton::Left) {
		// get 2d world position of cursor
		let (camera, camera_transform) = camera_q.single().unwrap();
		let window = windows.single().unwrap();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position)
		else {
			return;
		};
		// set the actor target and abandon any existing pollable task
		for mut actor_pathing in &mut actor_q {
			let existing_route = &mut actor_pathing.pollable_route;
			// if let Some(poll) = existing_route {
			// 	let _ = poll.detach();
			// }
			*existing_route = None;
			actor_pathing.target = Some(world_position);
			actor_pathing.route = None;
		}
	}
}
