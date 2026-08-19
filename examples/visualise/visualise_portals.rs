//! Generates a 30x30 world showing where Portals exist as purple squares.
//!
//! By LeftClicking tiles can be flipped between being impassable and passable
//! to showcase Portals being regenerated across Sectors
//!

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;

#[path = "../helpers/cell_icons.rs"]
mod cell_icons;
#[path = "../helpers/core.rs"]
mod core;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup_visualisation, create_counter))
		.add_systems(Update, (update_sprites, click_update_cost, update_counter))
		.run();
}

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let sprite_dimension = 64.0;
	let proj = Projection::Orthographic(OrthographicProjection {
		scale: 2.0,
		..OrthographicProjection::default_2d()
	});
	cmds.spawn((Camera2d, proj));

	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields_continuous_layout.ron";
	let flowfield_tiles = FlowFieldTiles::from_ron((0.0, 0.0), (1920.0, 1920.0), 64.0, 16.0, &path);
	let dimensions = flowfield_tiles.get_dimensions();
	let costfields = flowfield_tiles.get_sector_cost_fields();
	let read_costfields = costfields.read().unwrap();

	// iter over each sector creating sprites
	for (sector, costfield) in read_costfields.get_scaled_costs().iter() {
		let sector_top_left = dimensions.get_sector_corner_xy(sector);

		for (i, cost) in costfield.get().iter().enumerate() {
			let y_i = i / FIELD_RESOLUTION;
			let x_i = i % FIELD_RESOLUTION;
			// grid origin is always in the top left
			let x = sector_top_left.x + sprite_dimension / 2.0 + (sprite_dimension * x_i as f32);
			let y = sector_top_left.y - sprite_dimension / 2.0 - (sprite_dimension * y_i as f32);

			cmds.spawn((
				Sprite::from_image(asset_server.load(cell_icons::get_basic_icon(*cost))),
				Transform::from_xyz(x, y, 0.0),
			))
			.insert(core::FieldCellLabel(x_i, y_i))
			.insert(core::SectorLabel(sector.get_column(), sector.get_row()));
		}
	}
	// remove read lock so the entity can be created
	drop(read_costfields);
	cmds.spawn(flowfield_tiles);
}

/// Redraw sprites when Portals are changed (...very inefficiently)
fn update_sprites(
	flowfields_q: Query<&FlowFieldTiles, Changed<FlowFieldTiles>>,
	mut fieldcell_q: Query<(&mut Sprite, &core::FieldCellLabel, &core::SectorLabel)>,
	asset_server: Res<AssetServer>,
) {
	for flowfield_tiles in &flowfields_q {
		let mut portal_ids = vec![];

		let portals = flowfield_tiles.get_portals();
		let read_portals = portals.read().unwrap();

		for (sector, windows) in read_portals.get_portals().iter() {
			let portal_windows = windows.get_all();
			for window in portal_windows.iter() {
				for cell_index in window.get_all_window_cells().iter() {
					portal_ids.push((*sector, FieldCell::from_index(*cell_index)));
				}
			}
		}
		drop(read_portals);

		let costfields = flowfield_tiles.get_sector_cost_fields();
		let read_costfields = costfields.read().unwrap();
		for (mut sprite, cell_label, sector_label) in &mut fieldcell_q {
			if portal_ids.contains(&(
				SectorID::new(sector_label.0, sector_label.1),
				FieldCell::new(cell_label.0, cell_label.1),
			)) {
				let new_handle: Handle<Image> = asset_server.load("ordinal_icons/portals.png");
				sprite.image = new_handle;
			} else {
				let costfield = read_costfields
					.get_scaled_costs()
					.get(&SectorID::new(sector_label.0, sector_label.1))
					.unwrap();
				let c = costfield.get_field_cell_value(FieldCell::new(cell_label.0, cell_label.1));
				sprite.image = asset_server.load(cell_icons::get_basic_icon(c));
			}
		}
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

/// Create a counter to display number of portals
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
						font: FontSource::Monospace,
						font_size: FontSize::Px(30.0),
						..default()
					},
					TextColor(Color::WHITE),
				))
				.with_child((
					TextSpan::default(),
					TextFont {
						font: FontSource::Monospace,
						font_size: FontSize::Px(30.0),
						..default()
					},
				));
			});
		}
	});
}

/// Update the counter
fn update_counter(
	flow_q: Query<&FlowFieldTiles, Changed<FlowFieldTiles>>,
	mut query: Query<&mut TextSpan>,
) {
	let mut portal_count = 0;
	let flowfield_tiles = flow_q.single().unwrap();
	let portals = flowfield_tiles.get_portals();
	let read_portals = portals.read().unwrap();
	for windows in read_portals.get_portals().values() {
		portal_count += windows.get_all().len();
	}
	for mut text in &mut query {
		**text = format!("{portal_count:.2}");
		// if text.sections[0].value.as_str() == "Portals: " {
		// text.sections[1].value = format!("{portal_count:.2}");
		// }
	}
}
