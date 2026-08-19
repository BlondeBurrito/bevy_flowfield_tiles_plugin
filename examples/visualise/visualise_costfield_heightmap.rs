//! Demonstrates using a greyscale image heightmap as a means of initialising
//! the SectorCostFields.
//!
//! The heightmap is a 30x30 px png.
//!

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_systems(Startup, (setup,))
		.run();
}
/// Init world
fn setup(mut cmds: Commands) {
	// setup the field
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/heightmap.png";
	let map_dimensions = Dimensions::new((0.0, 0.0), (960.0, 960.0), 32.0, 1.0);
	let sector_costfields = SectorCostFields::from_heightmap(&map_dimensions, path);
	// create a UI grid
	cmds.spawn(Camera2d);
	cmds.spawn((
		Node {
			// background canvas
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			display: Display::Grid,
			grid_auto_flow: GridAutoFlow::Column,
			grid_template_columns: vec![
				GridTrack::px(300.0),
				GridTrack::px(300.0),
				GridTrack::px(300.0),
			],
			grid_template_rows: vec![
				GridTrack::px(300.0),
				GridTrack::px(300.0),
				GridTrack::px(300.0),
			],
			..Default::default()
		},
		BackgroundColor(Color::NONE),
	))
	.with_children(|p| {
		// create a box for each sector
		for costfield in sector_costfields.get_scaled_costs().values() {
			p.spawn((
				Node {
					width: Val::Px(300.0),
					height: Val::Px(300.0),
					flex_direction: FlexDirection::Row,
					flex_wrap: FlexWrap::Wrap,
					..Default::default()
				},
				BackgroundColor(Color::WHITE),
			))
			.with_children(|p| {
				for value in costfield.get().iter() {
					p.spawn(Node {
						width: Val::Percent(10.0),
						height: Val::Percent(10.0),
						justify_content: JustifyContent::Center,
						align_items: AlignItems::Center,
						..Default::default()
					})
					.with_children(|p| {
						p.spawn((
							Text::new(value.to_string()),
							TextFont {
								font: FontSource::Monospace,
								font_size: FontSize::Px(15.0),
								..default()
							},
							TextColor(Color::BLACK),
						));
					});
				}
			});
		}
	});
}
