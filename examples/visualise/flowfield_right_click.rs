//! Generates a single [FlowField] visualisation which uses right-mouse input to set a goal position, causing the visualisation to update to graphically show the flow field lines from a !static! actor position
//!

use bevy::{
	prelude::*,
	tasks::{Task, futures::check_ready},
	window::PrimaryWindow,
};
use bevy_flowfield_tiles_plugin::v2::{
	bundle::FlowFieldTiles,
	flowfields::{
		fields::{
			Field, FieldCell,
			cost_field::CostField,
			flow_field::{get_ordinal_from_bits, has_line_of_sight, is_goal, is_wall},
		},
		route_cache::RouteStep,
		utilities::{FIELD_RESOLUTION, Ordinal},
	},
	plugin::FlowFieldTilesPlugin,
};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup,))
		.add_systems(PreUpdate, user_input)
		.add_systems(
			Update,
			(actor_update_route, update_sprite_visuals_based_on_actor),
		)
		.run();
}
/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct FieldCellLabel(usize, usize);
/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;
/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Default, Component)]
struct Pathing {
	target: Option<Vec2>,
	pollable_route: Option<Task<Option<Vec<RouteStep>>>>,
	route: Option<Vec<RouteStep>>,
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
	let actor_size = world_unit_size / 2.0;
	cmds.spawn(FlowFieldTiles::from_ron(
		(0.0, 0.0),
		(map_length, map_depth),
		world_unit_size,
		actor_size,
		&s_path,
	));
	// use the impression of the cost field to just init node images
	let costfield = CostField::from_ron(c_path);
	// create a blank visualisation
	cmds.spawn(Camera2d);
	for (i, c) in costfield.get().iter().enumerate() {
		// grid origin is always in the top left
		let y_i = i / FIELD_RESOLUTION;
		let x_i = i % FIELD_RESOLUTION;
		let x = -(map_length) / 2.0 + sprite_dimension / 2.0 + (sprite_dimension * x_i as f32);
		let y = map_depth / 2.0 - sprite_dimension / 2.0 - (sprite_dimension * y_i as f32);
		cmds.spawn((
			Sprite {
				image: asset_server.load(get_basic_icon(*c)),
				..default()
			},
			Transform::from_xyz(x, y, 0.0),
		))
		.insert(FieldCellLabel(x_i, y_i));
	}
	// create the controllable actor but hide it with `z` behind everything
	cmds.spawn((
		Sprite {
			image: asset_server.load("2d/2d_actor_sprite.png"),
			..default()
		},
		Transform::from_xyz(0.0, 0.0, -1.0),
	))
	.insert(Actor)
	.insert(Pathing::default());
}

/// Handle user mouse clicks
fn user_input(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_q: Query<(&Transform, &mut Pathing), With<Actor>>,
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
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<Actor>>) {
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
	actor_q: Query<&Pathing, (With<Actor>, Changed<Pathing>)>,
	flowfield_tiles_q: Query<&FlowFieldTiles>,
	mut field_cell_q: Query<(&mut Sprite, &FieldCellLabel)>,
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
					let icon = get_ord_icon(flow_value);
					let new_handle: Handle<Image> = asset_server.load(icon);
					sprite.image = new_handle;
				}
			}
		}
	}
}
/// Get asset path of sprite assets
fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}
/// Get asset path of ordinal icon
fn get_ord_icon(value: u8) -> String {
	if is_goal(value) {
		return String::from("ordinal_icons/goal.png");
	}
	//
	if has_line_of_sight(value) {
		return String::from("ordinal_icons/los.png");
	}
	//
	if is_wall(value) {
		return String::from("ordinal_icons/impassable.png");
	}
	let ordinal = get_ordinal_from_bits(value);
	match ordinal {
		Ordinal::North => String::from("ordinal_icons/north.png"),
		Ordinal::East => String::from("ordinal_icons/east.png"),
		Ordinal::South => String::from("ordinal_icons/south.png"),
		Ordinal::West => String::from("ordinal_icons/west.png"),
		Ordinal::NorthEast => String::from("ordinal_icons/north_east.png"),
		Ordinal::SouthEast => String::from("ordinal_icons/south_east.png"),
		Ordinal::SouthWest => String::from("ordinal_icons/south_west.png"),
		Ordinal::NorthWest => String::from("ordinal_icons/north_west.png"),
		Ordinal::Zero => String::from("ordinal_icons/impassable.png"),
	}
}
