//! Calculates an [IntegrationField] as far the the Line-of-Sight calculation
//! layer and displays which cells are LOS, impassable and wavefront blocked
//! (unreachable cells due to blocking are labeled '?')
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
	// calculate the field
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/costfield_impassable.ron";
	let costfield = CostField::from_ron(path);
	let route_step = RouteStep::new(&SectorID::new(0, 0), 44, None);
	let intfield = IntegrationField::init(&costfield, &route_step);
	// create a UI grid
	cmds.spawn(Camera2d);
	cmds.spawn((
		Node {
			// background canvas
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			flex_direction: FlexDirection::Column,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			..Default::default()
		},
		BackgroundColor(Color::NONE),
	))
	.with_children(|p| {
		// a centred box to contain the field values
		p.spawn((
			Node {
				width: Val::Px(500.0),
				height: Val::Px(500.0),
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				..Default::default()
			},
			BackgroundColor(Color::WHITE),
		))
		.with_children(|p| {
			for value in intfield.get().iter() {
				p.spawn(Node {
					width: Val::Percent(10.0),
					height: Val::Percent(10.0),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					..Default::default()
				})
				.with_children(|p| {
					p.spawn((
						Text::new(convert_integration_flags(*value)),
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
	});
}
/// Using the integration flags derive a character symbol to represent the value
fn convert_integration_flags(value: u32) -> String {
	let flags = value & INT_FILTER_BITS_FLAGS;
	if flags & INT_BITS_GOAL == INT_BITS_GOAL {
		String::from("G")
	} else if flags & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE {
		String::from("X")
	} else if flags & INT_BITS_CORNER == INT_BITS_CORNER {
		String::from("C")
	} else if flags & INT_BITS_LOS == INT_BITS_LOS {
		String::from("LOS")
	} else if flags & INT_BITS_WAVE_BLOCKED == INT_BITS_WAVE_BLOCKED {
		String::from("WB")
	} else {
		String::from("?")
	}
}
