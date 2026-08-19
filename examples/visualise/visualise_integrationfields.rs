//! Calculates the [IntegrationField]s from a set of [CostField]s and displays
//! the cell values in a UI grid.
//!
//! For sectors which an actor does not need to traverse they are not generated
//! and left blank
//!

use std::collections::BTreeMap;

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
	// calculate the fields
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields.ron";
	let flowfield_tiles = FlowFieldTiles::from_ron((0.0, 0.0), (30.0, 30.0), 1.0, 0.5, &path);
	let costfields = flowfield_tiles.get_sector_cost_fields();
	let read_costfields = costfields.read().unwrap();
	// access portals to find high-level path
	let portals = flowfield_tiles.get_portals();
	let read_portals = portals.read().unwrap();

	let source_sector = SectorID::new(2, 0);
	let source_cell = FieldCell::new(7, 3);
	let goal_sector = SectorID::new(0, 2);
	let goal_cell = FieldCell::new(0, 7);

	let mut intfields: BTreeMap<SectorID, IntegrationField> = BTreeMap::new();
	if let Some(route) = read_portals.find_path(
		&source_sector,
		&source_cell,
		&goal_sector,
		&goal_cell,
		&*read_costfields,
	) {
		for step in route.iter() {
			// create integrationfields for each step
			let sector = step.get_sector();
			let costfield = &*read_costfields.get_scaled_costs().get(sector).unwrap();
			let mut intfield = IntegrationField::init(&costfield, &step);
			intfield.build(costfield);
			intfields.insert(*sector, intfield);
		}
	} else {
		panic!("Failed to find route");
	}
	drop(read_portals);

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
		let sectors = read_costfields.get_scaled_costs().keys();
		for sector in sectors {
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
				// if the sector is used by an integration field create visual
				if let Some(intfield) = intfields.get(sector) {
					for value in intfield.get() {
						p.spawn((
							Node {
								width: Val::Percent(10.0),
								height: Val::Percent(10.0),
								justify_content: JustifyContent::Center,
								align_items: AlignItems::Center,
								..Default::default()
							},
							BackgroundColor(get_colour(*value)),
						))
						.with_children(|p| {
							p.spawn((
								Text::new((value & INT_FILTER_BITS_COST).to_string()),
								TextFont {
									font: FontSource::Monospace,
									font_size: FontSize::Px(10.0),
									..default()
								},
								TextColor(Color::BLACK),
							));
						});
					}
				} else {
					// the sector is not generated as part of the route so create
					// some empty squares
					for _ in 0..(FIELD_RESOLUTION * FIELD_RESOLUTION) {
						p.spawn(Node {
							width: Val::Percent(10.0),
							height: Val::Percent(10.0),
							justify_content: JustifyContent::Center,
							align_items: AlignItems::Center,
							..Default::default()
						});
					}
				}
			});
		}
	});
	drop(read_costfields);
}
/// Get the colour of a UI node
fn get_colour(value: u32) -> Color {
	if value & INT_BITS_LOS == INT_BITS_LOS {
		return Color::WHITE;
	}
	let cost = value & INT_FILTER_BITS_COST;
	match cost {
		0 => Color::WHITE,
		1 => Color::srgb(1.0, 0.95, 0.68),
		65535 => Color::srgb(0.5, 0.5, 0.5),
		_ => Color::srgb(
			1.0,
			0.95 * 0.9_f32.powf(cost as f32),
			0.68 * 0.9_f32.powf(cost as f32),
		),
	}
}
