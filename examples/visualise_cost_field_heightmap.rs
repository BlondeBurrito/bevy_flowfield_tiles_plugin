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
fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// setup the field
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/heightmap.png";
	let map_dimensions = MapDimensions::new(960, 960, 320, 1.0);
	let sector_cost_fields = SectorCostFields::from_heightmap(&map_dimensions, path);
	// create a UI grid
	cmds.spawn(Camera2dBundle::default());
	cmds.spawn(NodeBundle {
		// background canvas
		style: Style {
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			display: Display::Grid,
			grid_auto_flow: GridAutoFlow::Column,
			grid_template_columns: vec![GridTrack::px(300.0), GridTrack::px(300.0), GridTrack::px(300.0)],
			grid_template_rows: vec![GridTrack::px(300.0), GridTrack::px(300.0), GridTrack::px(300.0)],
			..Default::default()
		},
		background_color: BackgroundColor(Color::NONE),
		..Default::default()
	})
	.with_children(|p| {
		// create a box for each sector
		for field in sector_cost_fields.get_scaled().values() {
			p.spawn(NodeBundle {
				style: Style {
					width: Val::Px(300.0),
					height: Val::Px(300.0),
					flex_direction: FlexDirection::Row,
					..Default::default()
				},
				background_color: BackgroundColor(Color::WHITE),
				..Default::default()
			}).with_children(|p| {
				// create each column from the field
			for array in field.get().iter() {
				p.spawn(NodeBundle {
					style: Style {
						width: Val::Percent(10.0),
						height: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						..Default::default()
					},
					..Default::default()
				})
				.with_children(|p| {
					// create each row value of the column
					for value in array.iter() {
						p.spawn(NodeBundle {
							style: Style {
								width: Val::Percent(100.0),
								height: Val::Percent(10.0),
								justify_content: JustifyContent::Center,
								align_items: AlignItems::Center,
								..Default::default()
							},
							..Default::default()
						})
						.with_children(|p| {
							p.spawn(TextBundle::from_section(
								value.to_string(),
								TextStyle {
									font: asset_server.load("fonts/FiraMono-Medium.ttf"),
									font_size: 13.0,
									color: Color::BLACK,
								},
							));
						});
					}
				});
			}
			});
		}
	});
}
