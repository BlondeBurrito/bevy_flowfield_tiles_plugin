//! Generates a 30x30 world showing where Portals exist as purple squares
//!

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, setup_visualisation)
		.add_systems(Update, (show_portals,))
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
	let map_dimensions = MapDimensions::new(1920, 1920, 640);
	let sprite_dimension = 64.0;
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
	let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	let sector_cost_fields = SectorCostFields::from_csv_dir(
		map_dimensions.get_length(),
		map_dimensions.get_depth(),
		map_dimensions.get_sector_resolution(),
		dir,
	);
	let fields = sector_cost_fields.get();
	// iterate over each sector field to place the sprites
	for (sector_id, field) in fields.iter() {
		// iterate over the dimensions of the field
		for (i, column) in field.get_field().iter().enumerate() {
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
	// spawn the portals tracker
	let mut portals = SectorPortals::new(
		map_dimensions.get_length(),
		map_dimensions.get_depth(),
		map_dimensions.get_sector_resolution(),
	);
	// update default portals for cost fields
	for sector_id in sector_cost_fields.get().keys() {
		portals.update_portals(*sector_id, &sector_cost_fields, &map_dimensions);
	}
	cmds.spawn(portals);
}

/// Spawn navigation related entities
fn show_portals(
	portals_q: Query<&SectorPortals>,
	mut field_cell_q: Query<(&mut Handle<Image>, &FieldCellLabel, &SectorLabel)>,
	asset_server: Res<AssetServer>,
) {
	let sector_portals = portals_q.get_single().unwrap().get();
	// store the ID of each sector and all the portal field coords in it
	let mut sector_portal_ids = HashMap::new();
	for (sector, portals) in sector_portals.iter() {
		let mut portal_ids = Vec::new();
		for ordinal_portals in portals.get() {
			for portal_node in ordinal_portals.iter() {
				portal_ids.push(portal_node.get_column_row());
			}
		}
		sector_portal_ids.insert(*sector, portal_ids);
	}
	// switch grid sprites to indicate portals
	for (mut handle, field_cell_label, sector_label) in field_cell_q.iter_mut() {
		// lookup the sector and grid
		if sector_portal_ids.contains_key(&SectorID::new(sector_label.0, sector_label.1)) {
			let value = sector_portal_ids
				.get(&SectorID::new(sector_label.0, sector_label.1))
				.unwrap();
			if value.contains(&(field_cell_label.0, field_cell_label.1)) {
				let new_handle: Handle<Image> = asset_server.load("ordinal_icons/portals.png");
				*handle = new_handle;
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
