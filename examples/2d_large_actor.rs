//! Generates a 30x30 world where a large actor demonstrates restricted navigation
//!

use avian2d::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;
use examples_utils::_2d::{
	actor_steering, check_if_route_exhausted, create_wall_colliders, get_or_request_route,
	stop_at_destination, Layer, Pathing, FIELD_SPRITE_DIMENSION,
};
use std::collections::HashMap;

/// Determines what areas are valid for pathing
const ACTOR_SIZE: f32 = 96.0;

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
		.add_systems(Update, (update_sprite_visuals_based_on_actor,))
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

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct SectorLabel(u32, u32);

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct FieldCellLabel(usize, usize);

/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution, ACTOR_SIZE);
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
				let sprite_x = FIELD_SPRITE_DIMENSION;
				let sprite_y = FIELD_SPRITE_DIMENSION;
				let sector_offset = map_dimensions.get_sector_corner_xy(*sector_id);
				let x = sector_offset.x + 32.0 + (sprite_x * i as f32);
				let y = sector_offset.y - 32.0 - (sprite_y * j as f32);
				// add colliders to impassable cells
				if *value == 255 {
					cmds.spawn((
						Sprite {
							custom_size: Some(Vec2::new(64.0, 64.0)),
							image: asset_server.load(get_basic_icon(*value)),
							..default()
						},
						Transform::from_xyz(x, y, 0.0),
					))
					.insert(FieldCellLabel(i, j))
					.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()))
					.insert(Collider::rectangle(
						FIELD_SPRITE_DIMENSION,
						FIELD_SPRITE_DIMENSION,
					))
					.insert(RigidBody::Static)
					.insert(CollisionLayers::new([Layer::Terrain], [Layer::Actor]));
				} else {
					cmds.spawn((
						Sprite {
							image: asset_server.load(get_basic_icon(*value)),
							..default()
						},
						Transform::from_xyz(x, y, 0.0),
					))
					.insert(FieldCellLabel(i, j))
					.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()));
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
	cmds.spawn(FlowFieldTilesBundle::from_ron(
		map_length,
		map_depth,
		sector_resolution,
		ACTOR_SIZE,
		&path,
	));
	// create the controllable actor in the top right corner
	cmds.spawn((
		Sprite {
			color: Color::srgb(230.0, 0.0, 255.0),
			..default()
		},
		Transform {
			translation: Vec3::new(886.0, 886.0, 1.0),
			scale: Vec3::new(ACTOR_SIZE, 16.0, 1.0),
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
			let mut pathing = actor_q.single_mut().unwrap();
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

/// Whenever the actor has a path assigned attempt to get the current flowfield and update all the map sprites to visualise the directions of flow
fn update_sprite_visuals_based_on_actor(
	actor_q: Query<&Pathing, With<Actor>>,
	flowfield_q: Query<&FlowFieldCache>,
	costfield_q: Query<&SectorCostFields>,
	mut field_cell_q: Query<(&mut Sprite, &FieldCellLabel, &SectorLabel)>,
	asset_server: Res<AssetServer>,
) {
	let f_cache = flowfield_q.single().unwrap();
	let sc_cache = costfield_q.single().unwrap();
	let pathing = actor_q.single().unwrap();
	if let Some(route) = &pathing.portal_route {
		let mut route_map: HashMap<SectorID, FieldCell> = HashMap::new();
		for (s, g) in route.iter() {
			route_map.insert(*s, *g);
		}
		for (mut sprite, field_cell_label, sector_label) in field_cell_q.iter_mut() {
			// look for the value in the route_map if it's part of the flow, otherwise use the cost field
			if route_map.contains_key(&SectorID::new(sector_label.0, sector_label.1)) {
				let goal = route_map
					.get(&SectorID::new(sector_label.0, sector_label.1))
					.unwrap();
				if let Some(flowfield) = f_cache.get_field(
					SectorID::new(sector_label.0, sector_label.1),
					pathing.target_sector.unwrap(),
					*goal,
				) {
					let flow_value = flowfield.get_field_cell_value(FieldCell::new(
						field_cell_label.0,
						field_cell_label.1,
					));
					let icon = get_ord_icon(flow_value);
					let new_handle: Handle<Image> = asset_server.load(icon);
					sprite.image = new_handle;
				}
			} else {
				let value = sc_cache
					.get_baseline()
					.get(&SectorID::new(sector_label.0, sector_label.1))
					.unwrap()
					.get_field_cell_value(FieldCell::new(field_cell_label.0, field_cell_label.1));
				let icon = get_basic_icon(value);
				let new_handle: Handle<Image> = asset_server.load(icon);
				sprite.image = new_handle;
			}
		}
	}
}
/// Get the asset path to ordinal icons
fn get_ord_icon(value: u8) -> String {
	if is_goal(value) {
		String::from("ordinal_icons/goal.png")
	} else if has_line_of_sight(value) {
		String::from("ordinal_icons/los.png")
	} else {
		//
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
}
