//! Calculates an [IntegrationField] from a [CostField] and displays the cell
//! cost values in a UI grid
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
	let mut intfield = IntegrationField::init(&costfield, &route_step);
	intfield.build(&costfield);
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
						Text::new((value & INT_FILTER_BITS_COST).to_string()),
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
