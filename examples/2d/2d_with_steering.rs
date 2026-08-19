//! Generates a 30x30 world where an actor can be told to navigate to a point with a right click
//!
//! Using left-click cells can be flipped between passable and impassable to mutate the costfields
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
		.add_systems(PreUpdate, (click_set_target, click_update_cost))
		.add_systems(Update, core2d::actor_request_route::<core::Actor>)
		.add_systems(Update, (update_sprite_visuals_based_on_actor,))
		.add_systems(
			FixedUpdate,
			(
				core2d::actor_update_route::<core::Actor>,
				core2d::actor_steering::<core::Actor>,
				// check_if_route_exhausted::<core::Actor>,
				core2d::stop_at_destination::<core::Actor>,
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
				.insert(core::FieldCellLabel(x_i, y_i))
				.insert(core::SectorLabel(
					sector_id.get_column(),
					sector_id.get_row(),
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
				))
				.insert(core::FieldCellLabel(x_i, y_i))
				.insert(core::SectorLabel(
					sector_id.get_column(),
					sector_id.get_row(),
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
	let size = (1920.0, 1920.0);
	let world_unit_size = core2d::WORLD_UNIT_SIZE;
	let actor_radius = core2d::ACTOR_RADIUS;
	cmds.spawn(FlowFieldTiles::from_ron(
		origin,
		size,
		world_unit_size,
		actor_radius,
		&path,
	));

	// create the controllable actor in the top right corner
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
	.insert(core::Actor)
	.insert(core2d::Pathing::default())
	.insert(RigidBody::Dynamic)
	.insert(Collider::circle(1.0))
	.insert(AngularDamping(1.0))
	.insert(CollisionLayers::new(
		[core2d::Layer::Actor],
		[core2d::Layer::Terrain],
	));
}

/// Handle user mouse clicks
fn click_set_target(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_q: Query<&mut core2d::Pathing, With<core::Actor>>,
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
		let mut actor_pathing = actor_q.single_mut().unwrap();
		let existing_route = &mut actor_pathing.pollable_route;
		// if let Some(poll) = existing_route {
		// 	let _ = poll.detach();
		// }
		*existing_route = None;
		actor_pathing.target = Some(world_position);
		actor_pathing.route = None;
	}
}

/// Left clicking on a tile/field will flip the value of it in the [CostField]
///
/// If the current cost is `1` then it is updated to `255`. If the current cost
/// is `255` then it is flipped to `1`
fn click_update_cost(
	input: Res<ButtonInput<MouseButton>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	windows: Query<&Window, With<PrimaryWindow>>,
	mut flow_q: Query<&mut FlowFieldTiles>,
) {
	if input.just_released(MouseButton::Left) {
		let (camera, camera_transform) = camera_q.single().unwrap();
		let window = windows.single().unwrap();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position)
		else {
			return;
		};
		let mut flowfield_tiles = flow_q.single_mut().unwrap();
		let dimensions = flowfield_tiles.get_dimensions();
		let costfields = flowfield_tiles.get_sector_cost_fields().clone();

		if let Some((sector_id, field_cell)) =
			dimensions.get_sector_and_field_cell_from_xy(world_position)
		{
			let read_costfields = costfields.read().unwrap();
			let costfield = read_costfields.get_scaled_costs().get(&sector_id).unwrap();
			let value = costfield.get_field_cell_value(field_cell);
			if value == 255 {
				flowfield_tiles.add_costfield_update_2d(world_position, 1);
			} else {
				flowfield_tiles.add_costfield_update_2d(world_position, 255);
			}
		}
	}
}

/// Whenever the actor has a path assigned attempt to get the current flowfield and update all the map sprites to visualise the directions of flow
fn update_sprite_visuals_based_on_actor(
	actor_q: Query<&core2d::Pathing, (With<core::Actor>, Changed<core2d::Pathing>)>,
	flowfield_tiles_q: Query<&FlowFieldTiles>,
	mut field_cell_q: Query<(&mut Sprite, &core::FieldCellLabel, &core::SectorLabel)>,
	asset_server: Res<AssetServer>,
) {
	for pathing in &actor_q {
		let flowfield_tiles = flowfield_tiles_q.single().unwrap();
		if let Some(route) = &pathing.route {
			if let Some(step) = route.first() {
				if let Some(flowfield) = flowfield_tiles.read_flowfield(step) {
					for (mut sprite, field_cell_label, sector_label) in field_cell_q.iter_mut() {
						let sector = SectorID::new(sector_label.0, sector_label.1);
						if sector == *step.get_sector() {
							let flow_value = flowfield.get_field_cell_value(FieldCell::new(
								field_cell_label.0,
								field_cell_label.1,
							));
							let icon = cell_icons::get_ord_icon(flow_value);
							let new_handle: Handle<Image> = asset_server.load(icon);
							sprite.image = new_handle;
						}
					}
				}
			}
		}
	}
}
