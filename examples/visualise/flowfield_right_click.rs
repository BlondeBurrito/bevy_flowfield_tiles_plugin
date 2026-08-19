//! Generates a single [FlowField] visualisation which uses right-mouse input
//! to set a goal position, causing the visualisation to update and graphically
//! show the flowfield lines from a !static! actor position
//!

use bevy::{prelude::*, tasks::futures::check_ready, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;

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
		.add_plugins(DefaultPlugins)
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup,))
		.add_systems(PreUpdate, click_set_target)
		.add_systems(
			Update,
			(actor_update_route, update_sprite_visuals_based_on_actor),
		)
		.run();
}

/// Init bundle and setup world and actor
fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// create the entity handling the algorithm
	let s_path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfield_single.ron";
	let c_path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/costfield_impassable.ron";
	let map_length = 640.0;
	let map_depth = 640.0;
	let sprite_dimension = 64.0;
	let world_unit_size = sprite_dimension;
	let actor_radius = world_unit_size / 2.0;
	cmds.spawn(FlowFieldTiles::from_ron(
		(0.0, 0.0),
		(map_length, map_depth),
		world_unit_size,
		actor_radius,
		&s_path,
	));
	// use the impression of the cost field to just init node images
	let costfield = CostField::from_ron(c_path);
	// create a blank visualisation
	cmds.spawn(camera::get_camera_2d(1.0));
	for (i, c) in costfield.get().iter().enumerate() {
		// grid origin is always in the top left
		let y_i = i / FIELD_RESOLUTION;
		let x_i = i % FIELD_RESOLUTION;
		let x = -(map_length) / 2.0 + sprite_dimension / 2.0 + (sprite_dimension * x_i as f32);
		let y = map_depth / 2.0 - sprite_dimension / 2.0 - (sprite_dimension * y_i as f32);
		cmds.spawn((
			Sprite {
				image: asset_server.load(cell_icons::get_basic_icon(*c)),
				..default()
			},
			Transform::from_xyz(x, y, 0.0),
		))
		.insert(core::FieldCellLabel(x_i, y_i));
	}
	// create the actor but hide it with `z` behind everything
	cmds.spawn((
		Sprite {
			image: asset_server.load("2d/2d_actor_sprite.png"),
			..default()
		},
		Transform::from_xyz(0.0, 0.0, -1.0),
	))
	.insert(core::Actor)
	.insert(core2d::Pathing::default());
}

/// Handle user mouse clicks
fn click_set_target(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_q: Query<(&Transform, &mut core2d::Pathing), With<core::Actor>>,
	flow_q: Query<&FlowFieldTiles>,
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
		// get the actor position
		let (actor_tform, mut actor_pathing) = actor_q.single_mut().unwrap();
		// ask for a route
		for flowfield_tiles in &flow_q {
			let task =
				flowfield_tiles.get_route_2d(actor_tform.translation.truncate(), world_position);
			if let Some(t) = task {
				actor_pathing.target = Some(world_position);
				actor_pathing.pollable_route = Some(t);
				actor_pathing.route = None;
			}
		}
	}
}
/// There is a delay between the actor sending a path request and a route
/// becoming available. This checks to see if the route is available
fn actor_update_route(mut actor_q: Query<&mut core2d::Pathing, With<core::Actor>>) {
	let mut pathing = actor_q.single_mut().unwrap();
	if let Some(mut poll) = pathing.pollable_route.as_mut() {
		if let Some(route) = check_ready(&mut poll) {
			// task finished
			pathing.pollable_route = None;
			pathing.route = route;
		}
	}
}

/// Whenever the actor has a path assigned attempt to get the current flowfield and update all the map sprites to visualise the directions of flow
fn update_sprite_visuals_based_on_actor(
	actor_q: Query<&core2d::Pathing, (With<core::Actor>, Changed<core2d::Pathing>)>,
	flowfield_tiles_q: Query<&FlowFieldTiles>,
	mut field_cell_q: Query<(&mut Sprite, &core::FieldCellLabel)>,
	asset_server: Res<AssetServer>,
) {
	for pathing in &actor_q {
		let flowfield_tiles = flowfield_tiles_q.single().unwrap();
		if let Some(route) = &pathing.route {
			if let Some(flowfield) = flowfield_tiles.read_flowfield(&route[0]) {
				for (mut sprite, field_cell_label) in field_cell_q.iter_mut() {
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
