//! Generates a 30x30 world showing where Portals exist as purple squares.
//!
//! By LeftClicking tiles can be flipped bewteen being impassable and passable
//! to showcase Portals being regenerated across Sectors
//!

use std::collections::HashMap;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup_visualisation, create_counter))
		.add_systems(Update, (update_sprites, click_update_cost, update_counter))
		.run();
}

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct SectorLabel(u32, u32);

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct FieldCellLabel(usize, usize);

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let sprite_dimension = 64.0;
	let proj = Projection::Orthographic(OrthographicProjection {
		scale: 2.0,
		..OrthographicProjection::default_2d()
	});
	cmds.spawn((Camera2d, proj));
	// let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	// let sector_cost_fields = SectorCostFields::from_csv_dir(&map_dimensions, dir);
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields_continuous_layout.ron";
	let bundle = FlowFieldTilesBundle::from_ron(1920, 1920, 640, 16.0, &path);
	let map_dimensions = bundle.get_map_dimensions();
	let sector_cost_fields = bundle.get_sector_cost_fields();
	let fields = sector_cost_fields.get_baseline();
	// iterate over each sector field to place the sprites
	for (sector_id, field) in fields.iter() {
		// iterate over the dimensions of the field
		for (i, column) in field.get().iter().enumerate() {
			for (j, value) in column.iter().enumerate() {
				// grid origin is always in the top left
				let sector_offset = map_dimensions.get_sector_corner_xy(*sector_id);
				let x = sector_offset.x + 32.0 + (sprite_dimension * i as f32);
				let y = sector_offset.y - 32.0 - (sprite_dimension * j as f32);
				cmds.spawn((
					Sprite::from_image(asset_server.load(get_basic_icon(*value))),
					Transform::from_xyz(x, y, 0.0),
				))
				.insert(FieldCellLabel(i, j))
				.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()));
			}
		}
	}
	cmds.spawn(bundle);
}

/// Redraw sprites when Portals are changed
fn update_sprites(
	query: Query<(&SectorPortals, &SectorCostFields), Changed<SectorPortals>>,
	mut field_cell_q: Query<(&mut Sprite, &FieldCellLabel, &SectorLabel)>,
	asset_server: Res<AssetServer>,
) {
	for (sector_portals, sector_costfields) in &query {
		// store the ID of each sector and all the portal field coords in it
		let mut sector_portal_ids = HashMap::new();
		for (sector, portals) in sector_portals.get().iter() {
			let mut portal_ids = Vec::new();
			let ords = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
			for ord in ords.iter() {
				for cell in portals.get(ord).iter() {
					portal_ids.push(cell.get_column_row());
				}
			}
			sector_portal_ids.insert(*sector, portal_ids);
		}
		// update all the sprites
		for (mut sprite, field_cell_label, sector_label) in field_cell_q.iter_mut() {
			let sector_id = SectorID::new(sector_label.0, sector_label.1);
			let field_cell = FieldCell::new(field_cell_label.0, field_cell_label.1);
			let field = sector_costfields.get_scaled().get(&sector_id).unwrap();
			let cost = field.get_field_cell_value(field_cell);
			sprite.image = asset_server.load(get_basic_icon(cost));
			// lookup the sector and grid of a portal and overwrite as necessary
			if sector_portal_ids.contains_key(&sector_id) {
				let cell_id = sector_portal_ids.get(&sector_id).unwrap();
				if cell_id.contains(&(field_cell_label.0, field_cell_label.1)) {
					let new_handle: Handle<Image> = asset_server.load("ordinal_icons/portals.png");
					sprite.image = new_handle;
				}
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

/// Left clicking on a tile/field will flip the value of it in the [CostField]
///
/// If the current cost is `1` then it is updated to `255` and a [Collider] is inserted denoting an impassable field.
///
/// If the current cost is `255` then it is flipped to `1` and the collider removed
fn click_update_cost(
	mut tile_q: Query<(&SectorLabel, &FieldCellLabel, &mut Sprite)>,
	input: Res<ButtonInput<MouseButton>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	windows: Query<&Window, With<PrimaryWindow>>,
	dimensions_q: Query<(&MapDimensions, &SectorCostFields)>,
	mut event: MessageWriter<EventUpdateCostfieldsCell>,
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
		let (map_dimensions, cost_fields) = dimensions_q.single().unwrap();
		if let Some((sector_id, field_cell)) =
			map_dimensions.get_sector_and_field_cell_from_xy(world_position)
		{
			let cost_field = cost_fields.get_baseline().get(&sector_id).unwrap();
			let value = cost_field.get_field_cell_value(field_cell);
			if value == 255 {
				let e = EventUpdateCostfieldsCell::new(field_cell, sector_id, 1);
				event.write(e);
				// remove collider from tile
				for (sector_label, field_label, mut sprite) in &mut tile_q {
					if (sector_label.0, sector_label.1) == sector_id.get()
						&& (field_label.0, field_label.1) == field_cell.get_column_row()
					{
						sprite.color = Color::WHITE;
					}
				}
			} else {
				let e = EventUpdateCostfieldsCell::new(field_cell, sector_id, 255);
				event.write(e);
				// add collider to tile
				for (sector_label, field_label, mut sprite) in &mut tile_q {
					if (sector_label.0, sector_label.1) == sector_id.get()
						&& (field_label.0, field_label.1) == field_cell.get_column_row()
					{
						sprite.color = Color::BLACK;
					}
				}
			}
		}
	}
}

/// Create UI counters to measure the FPS and number of actors
fn create_counter(mut cmds: Commands) {
	cmds.spawn(Node {
		flex_direction: FlexDirection::Column,
		..default()
	})
	.with_children(|p| {
		let categories = vec!["Portals: "];
		for category in categories {
			p.spawn(Node::default()).with_children(|p| {
				p.spawn((
					Text::new(category),
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
				));
			});
		}
	});
}

/// Update the counters for FPS, number of actors, time elapased and current fields cached
fn update_counter(sector_portals_q: Query<&SectorPortals>, mut query: Query<&mut TextSpan>) {
	let mut portal_count = 0;
	let sp = sector_portals_q.single().unwrap();
	for portals in sp.get().values() {
		let ords = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
		for ord in ords.iter() {
			portal_count += portals.get(ord).len();
		}
	}
	for mut text in &mut query {
		**text = format!("{portal_count:.2}");
		// if text.sections[0].value.as_str() == "Portals: " {
		// text.sections[1].value = format!("{portal_count:.2}");
		// }
	}
}
