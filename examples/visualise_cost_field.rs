//! Displays the cells of a [CostField]
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
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/cost_field_impassable.ron";
	let cost_field = CostField::from_ron(path);
	// create a UI grid
	cmds.spawn(Camera2d);
	cmds.spawn((Node {
		// background canvas
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			flex_direction: FlexDirection::Column,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			..Default::default()
		},
		BackgroundColor(Color::NONE),)
	)
	.with_children(|p| {
		// a centred box to contain the field values
		p.spawn((Node {
				width: Val::Px(500.0),
				height: Val::Px(500.0),
				flex_direction: FlexDirection::Row,
				..Default::default()
			},
			BackgroundColor(Color::WHITE),)
		)
		.with_children(|p| {
			// create each column from the field
			for array in cost_field.get().iter() {
				p.spawn(Node {
						width: Val::Percent(10.0),
						height: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						..Default::default()
				})
				.with_children(|p| {
					// create each row value of the column
					for value in array.iter() {
						p.spawn(Node {
								width: Val::Percent(100.0),
								height: Val::Percent(10.0),
								justify_content: JustifyContent::Center,
								align_items: AlignItems::Center,
								..Default::default()
						})
						.with_children(|p| {
							p.spawn((
								Text::new(value.to_string()),
								TextFont {
									font: asset_server.load("fonts/FiraMono-Medium.ttf"),
									font_size: 15.0,
									..default()
								},
								TextColor(Color::BLACK),
							));
						});
					}
				});
			}
		});
	});
}
