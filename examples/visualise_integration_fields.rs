//! Calculates an [IntegrationFields] from a [CostFields] and displays the cell values in a UI grid
//!

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::flowfields::{
	cost_fields::CostFields, integration_fields::IntegrationFields,
};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_systems(Startup, (setup,))
		.run();
}

fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// calculate the field
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/cost_fields_impassable.ron";
	let cost_fields = CostFields::from_file(path);
	let mut int_field = IntegrationFields::default();
	let source = (4, 4);
	int_field.reset(source);
	int_field.calculate_fields(source, &cost_fields);
	// create a UI grid
	cmds.spawn(Camera2dBundle::default());
	cmds.spawn(NodeBundle {
		// background canvas
		style: Style {
			size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
			flex_direction: FlexDirection::Column,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			..Default::default()
		},
		background_color: BackgroundColor(Color::NONE),
		..Default::default()
	})
	.with_children(|p| {
		// a centred box to contain the field values
		p.spawn(NodeBundle {
			style: Style {
				size: Size::new(Val::Px(500.0), Val::Px(500.0)),
				flex_direction: FlexDirection::Row,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..Default::default()
			},
			background_color: BackgroundColor(Color::WHITE),
			..Default::default()
		})
		.with_children(|p| {
			// create each column from the field
			for array in int_field.get_fields().iter() {
				p.spawn(NodeBundle {
					style: Style {
						size: Size::new(Val::Percent(10.0), Val::Percent(100.0)),
						flex_direction: FlexDirection::Column,
						justify_content: JustifyContent::Center,
						align_items: AlignItems::Center,
						..Default::default()
					},
					..Default::default()
				})
				.with_children(|p| {
					// create each row value of the column
					for value in array.iter() {
						p.spawn(NodeBundle {
							style: Style {
								size: Size::new(Val::Percent(100.0), Val::Percent(10.0)),
								flex_direction: FlexDirection::Column,
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
									font_size: 15.0,
									color: Color::BLACK,
								},
							));
						});
					}
				});
			}
		});
	});
}
