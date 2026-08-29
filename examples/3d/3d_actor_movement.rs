//! Creates a 3d world with an actor represented by a blue capsule which can be moved with right click
//!

use avian3d::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;
use std::time::Duration;

// to reduce code duplication certain constants and systems that make up
// the steering pipeline are sourced from helper modules
// NB: the steering systems are very primitive - they do the bare minimum to
// help showcase bevy_flowfield_tiles_plugin
#[path = "../helpers/camera.rs"]
mod camera;
#[path = "../helpers/core.rs"]
mod core;
#[path = "../helpers/core3d.rs"]
mod core3d;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			PhysicsPlugins::default(),
			// PhysicsDebugPlugin::default(),
		))
		.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(
			core3d::ACTOR_TIMESTEP,
		)))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup_visualisation, setup_navigation))
		.add_systems(PreUpdate, click_set_target)
		.add_systems(Update, core3d::actor_request_route::<core::Actor>)
		.add_systems(
			FixedUpdate,
			(
				core3d::actor_update_route::<core::Actor>,
				core3d::actor_steering::<core::Actor>,
				core3d::stop_at_destination::<core::Actor>,
			),
		)
		.run();
}

/// Spawn the map
fn setup_visualisation(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mesh_mats: ResMut<Assets<StandardMaterial>>,
) {
	cmds.spawn(core3d::get_light());
	cmds.spawn(camera::get_camera_3d());
	// get field representation of world to help with spawning entities
	let origin = (0.0, 0.0);
	let size = (30.0, 30.0);
	let world_unit_size = core3d::WORLD_UNIT_SIZE;
	let actor_radius = core3d::ACTOR_RADIUS;
	let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields_continuous_layout.ron";
	let sector_cost_fields = SectorCostFields::from_ron(path, &dimensions);
	// create plane for the world
	cmds.spawn((
		Transform::from_translation(Vec3::new(0.0, -0.1, 0.0)),
		Mesh3d(meshes.add(Cuboid::new(size.0, 0.2, size.1))),
		MeshMaterial3d(mesh_mats.add(StandardMaterial::from_color(Color::WHITE))),
	));
	// create walls from costfields
	let fields = sector_cost_fields.get_scaled_costs();
	for (sector_id, costfield) in fields.iter() {
		let sector_top_left = dimensions.get_sector_corner_xyz(sector_id);

		for (i, cost) in costfield.get().iter().enumerate() {
			let z_i = i / FIELD_RESOLUTION;
			let x_i = i % FIELD_RESOLUTION;
			// grid origin is always in the top left
			let x = sector_top_left.x
				+ core3d::WORLD_UNIT_SIZE / 2.0
				+ (core3d::WORLD_UNIT_SIZE * x_i as f32);
			let z = sector_top_left.z
				+ core3d::WORLD_UNIT_SIZE / 2.0
				+ (core3d::WORLD_UNIT_SIZE * z_i as f32);
			// add colliders to impassable cells
			if *cost == 255 {
				cmds.spawn((
					Transform::from_xyz(x, 0.0, z),
					Mesh3d(meshes.add(Cuboid::new(
						core3d::WORLD_UNIT_SIZE,
						core3d::WORLD_UNIT_SIZE,
						core3d::WORLD_UNIT_SIZE,
					))),
					MeshMaterial3d(mesh_mats.add(StandardMaterial::from_color(Color::BLACK))),
				))
				.insert(Collider::cuboid(
					core3d::WORLD_UNIT_SIZE,
					core3d::WORLD_UNIT_SIZE,
					core3d::WORLD_UNIT_SIZE,
				))
				.insert(RigidBody::Static)
				.insert(CollisionLayers::new(
					[core3d::Layer::Terrain],
					[core3d::Layer::Actor],
				));
			}
		}
	}
	// collider walls around everything
	let outer_walls = core3d::get_wall_colliders(size.0, size.1);
	for bundle in outer_walls {
		cmds.spawn(bundle);
	}
}

/// Spawn navigation related entities
fn setup_navigation(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mesh_mats: ResMut<Assets<StandardMaterial>>,
) {
	// create the entity handling the algorithm
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields_continuous_layout.ron";
	cmds.spawn(FlowFieldTiles::from_ron(
		(0.0, 0.0),
		(30.0, 30.0),
		core3d::WORLD_UNIT_SIZE,
		core3d::ACTOR_RADIUS,
		&path,
	));
	// create the controllable actor in the top right corner
	let mesh = meshes.add(Mesh::from(bevy::math::primitives::Capsule3d {
		radius: core3d::ACTOR_RADIUS,
		half_length: core3d::ACTOR_HEIGHT / 2.0,
	}));
	let material = mesh_mats.add(StandardMaterial {
		base_color: Color::Srgba(Srgba::BLUE),
		..default()
	});
	cmds.spawn((
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(14.5, 1.0, -14.5),
		core::Actor,
		RigidBody::Dynamic,
		Collider::capsule(core3d::ACTOR_RADIUS, core3d::ACTOR_HEIGHT / 2.0),
		CollisionLayers::new([core3d::Layer::Actor], [core3d::Layer::Terrain]),
		AngularDamping(1.0),
		LockedAxes::new()
			.lock_translation_y()
			.lock_rotation_x()
			.lock_rotation_z(),
		core3d::Pathing::default(),
	));
}

/// Handle user mouse clicks
fn click_set_target(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_q: Query<&mut core3d::Pathing, With<core::Actor>>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 2d world position of cursor
		let (camera, camera_transform) = camera_q.single().unwrap();
		let window = windows.single().unwrap();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(world_position) = camera.viewport_to_world(camera_transform, cursor_position) else {
			return;
		};
		if let Some(intersect) = world_position.plane_intersection_point(
			Vec3::new(0.0, 0.0, 0.0),
			InfinitePlane3d { normal: Dir3::Y },
		) {
			// set the actor target and abandon any existing pollable task
			let mut actor_pathing = actor_q.single_mut().unwrap();
			let existing_route = &mut actor_pathing.pollable_route;
			// if let Some(poll) = existing_route {
			// 	let _ = poll.detach();
			// }
			*existing_route = None;
			actor_pathing.target = Some(intersect);
			actor_pathing.route = None;
		}
	}
}
