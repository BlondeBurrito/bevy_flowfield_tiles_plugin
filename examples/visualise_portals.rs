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
struct GridLabel(usize, usize);

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let map_length = 30; // in sprite count
	let map_depth = 30; // in sprite count
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
	let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	let sector_cost_fields = SectorCostFields::from_csv_dir(map_length, map_depth, dir);
	let fields = sector_cost_fields.get();
	// iterate over each sector field to place the sprites
	for (sector_id, field) in fields.iter() {
		// iterate over the dimensions of the field
		for (i, column) in field.get_field().iter().enumerate() {
			for (j, value) in column.iter().enumerate() {
				// grid origin is always in the top left
				let sprite_x = 64.0;
				let sprite_y = 64.0;
				let sector_offset = get_sector_xy_at_top_left(
					*sector_id,
					map_length * sprite_x as u32,
					map_depth * sprite_y as u32,
					sprite_x,
				);
				let x = sector_offset.x + 32.0 + (sprite_x * i as f32);
				let y = sector_offset.y - 32.0 - (sprite_y * j as f32);
				cmds.spawn(SpriteBundle {
					texture: asset_server.load(get_basic_icon(*value)),
					transform: Transform::from_xyz(x, y, 0.0),
					..default()
				})
				.insert(GridLabel(i, j))
				.insert(SectorLabel(sector_id.0, sector_id.1));
			}
		}
	}
	// spawn the portals tracker
	let mut portals = SectorPortals::new(map_length, map_depth);
	// update default portals for cost fields
	for sector_id in sector_cost_fields.get().keys() {
		portals.update_portals(*sector_id, &sector_cost_fields, map_length, map_depth);
	}
	cmds.spawn(portals);
}

/// Spawn navigation related entities
fn show_portals(
	portals_q: Query<&SectorPortals>,
	mut grid_q: Query<(&mut Handle<Image>, &GridLabel, &SectorLabel)>,
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
	for (mut handle, grid_label, sector_label) in grid_q.iter_mut() {
		// lookup the sector and grid
		if sector_portal_ids.contains_key(&(sector_label.0, sector_label.1)) {
			let value = sector_portal_ids
				.get(&(sector_label.0, sector_label.1))
				.unwrap();
			if value.contains(&(grid_label.0, grid_label.1)) {
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
