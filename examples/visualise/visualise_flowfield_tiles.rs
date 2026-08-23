//! Calculates the [FlowField]s from a set of [CostField]s and displays the
//! flow vectors as graphical arrows.
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
fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
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

	let mut intfields: BTreeMap<RouteStep, IntegrationField> = BTreeMap::new();
	if let Some(route) = read_portals.find_path(
		&source_sector,
		&source_cell,
		&goal_sector,
		&goal_cell,
		&read_costfields,
	) {
		for step in route.iter() {
			// create integrationfields for each step
			let sector = step.get_sector();
			let costfield = &read_costfields.get_scaled_costs().get(sector).unwrap();
			let mut intfield = IntegrationField::init(costfield, step);
			intfield.build(costfield);
			intfields.insert(*step, intfield);
		}
	} else {
		panic!("Failed to find route");
	}
	drop(read_portals);

	let mut flowfields: BTreeMap<SectorID, FlowField> = BTreeMap::new();
	for (step, intfield) in intfields.iter() {
		let mut flowfield = FlowField::new(step, intfield);
		flowfield.build(intfield);
		flowfields.insert(*step.get_sector(), flowfield);
	}

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
				if let Some(flowfield) = flowfields.get(sector) {
					for value in flowfield.get() {
						p.spawn((
							Node {
								width: Val::Percent(10.0),
								height: Val::Percent(10.0),
								justify_content: JustifyContent::Center,
								align_items: AlignItems::Center,
								..Default::default()
							},
							BackgroundColor(Color::WHITE),
							ImageNode::new(asset_server.load(get_compass_dir_icon(*value))),
						));
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
/// Get the asset path of compass dir icons
fn get_compass_dir_icon(value: u8) -> String {
	// println!(
	// 	"{} :: flags: {:#010b}, cost: {:#010b}",
	// 	value,
	// 	value & 240,
	// 	value & 15
	// );
	//
	if is_goal(value) {
		return String::from("compass_dir_icons/goal.png");
	}
	//
	if has_line_of_sight(value) {
		return String::from("compass_dir_icons/los.png");
	}
	//
	if is_wall(value) {
		return String::from("compass_dir_icons/impassable.png");
	}
	let compass_dir = get_compass_dir_from_bits(value);
	match compass_dir {
		CompassDir::North => String::from("compass_dir_icons/north.png"),
		CompassDir::East => String::from("compass_dir_icons/east.png"),
		CompassDir::South => String::from("compass_dir_icons/south.png"),
		CompassDir::West => String::from("compass_dir_icons/west.png"),
		CompassDir::NorthEast => String::from("compass_dir_icons/north_east.png"),
		CompassDir::SouthEast => String::from("compass_dir_icons/south_east.png"),
		CompassDir::SouthWest => String::from("compass_dir_icons/south_west.png"),
		CompassDir::NorthWest => String::from("compass_dir_icons/north_west.png"),
		CompassDir::Zero => String::from("compass_dir_icons/impassable.png"),
	}
}
