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
		.add_systems(Startup, setup_visualisation)
		.add_systems(Update, (update_sprites, click_update_cost))
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
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
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
				cmds.spawn(SpriteBundle {
					texture: asset_server.load(get_basic_icon(*value)),
					transform: Transform::from_xyz(x, y, 0.0),
					..default()
				})
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
	mut field_cell_q: Query<(&mut Handle<Image>, &FieldCellLabel, &SectorLabel)>,
	asset_server: Res<AssetServer>,) {
	for (sector_portals, sector_costfields) in &query {
		// store the ID of each sector and all the portal field coords in it
		let mut sector_portal_ids = HashMap::new();
		for (sector, portals) in sector_portals.get().iter() {
			let mut portal_ids = Vec::new();
			for ordinal_portals in portals.get() {
				for portal_node in ordinal_portals.iter() {
					portal_ids.push(portal_node.get_column_row());
				}
			}
			sector_portal_ids.insert(*sector, portal_ids);
		}
		// update all the sprites
		for (mut handle, field_cell_label, sector_label) in field_cell_q.iter_mut() {
			let sector_id = SectorID::new(sector_label.0, sector_label.1);
			let field_cell = FieldCell::new(field_cell_label.0, field_cell_label.1);
			let field = sector_costfields.get_scaled().get(&sector_id).unwrap();
			let cost = field.get_field_cell_value(field_cell);
			*handle = asset_server.load(get_basic_icon(cost));
			// lookup the sector and grid of a portal and overwrite as necessary
			if sector_portal_ids.contains_key(&sector_id) {
				let cell_id = sector_portal_ids
					.get(&sector_id)
					.unwrap();
				if cell_id.contains(&(field_cell_label.0, field_cell_label.1)) {
					let new_handle: Handle<Image> = asset_server.load("ordinal_icons/portals.png");
					*handle = new_handle;
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
/// If the current cost is `255` then
fn click_update_cost(
	mut tile_q: Query<(&SectorLabel, &FieldCellLabel, &mut Sprite)>,
	input: Res<ButtonInput<MouseButton>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	windows: Query<&Window, With<PrimaryWindow>>,
	dimensions_q: Query<(&MapDimensions, &SectorCostFields)>,
	mut event: EventWriter<EventUpdateCostfieldsCell>,
) {
	if input.just_released(MouseButton::Left) {
		let (camera, camera_transform) = camera_q.single();
		let window = windows.single();
		if let Some(world_position) = window
			.cursor_position()
			.and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
			.map(|ray| ray.origin.truncate())
		{
			let (map_dimensions, cost_fields) = dimensions_q.get_single().unwrap();
			if let Some((sector_id, field_cell)) =
				map_dimensions.get_sector_and_field_cell_from_xy(world_position)
			{
				let cost_field = cost_fields.get_baseline().get(&sector_id).unwrap();
				let value = cost_field.get_field_cell_value(field_cell);
				if value == 255 {
					let e = EventUpdateCostfieldsCell::new(field_cell, sector_id, 1);
					event.send(e);
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
					event.send(e);
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
}