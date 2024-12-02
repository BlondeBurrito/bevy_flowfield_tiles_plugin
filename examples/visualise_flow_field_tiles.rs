//! Calculates the [FlowField]s from a set of [CostField]s and displays the cell values in a UI grid.
//!
//! For sectors which an actor does not need to traverse they are not generated or rendered
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
	let map_dimensions = MapDimensions::new(30, 30, 10, 1.0);
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let sector_cost_fields = SectorCostFields::from_ron(path, &map_dimensions);
	let mut sector_portals = SectorPortals::new(
		map_dimensions.get_length(),
		map_dimensions.get_depth(),
		map_dimensions.get_sector_resolution(),
	);
	// update default portals for cost fields
	for sector_id in sector_cost_fields.get_scaled().keys() {
		sector_portals.update_portals(*sector_id, &sector_cost_fields, &map_dimensions);
	}
	// generate the portal graph
	let portal_graph = PortalGraph::new(&sector_portals, &sector_cost_fields, &map_dimensions);
	//
	let source_sector = SectorID::new(2, 0);
	let source_field_cell = FieldCell::new(7, 3);
	let target_sector = SectorID::new(0, 2);
	let target_field_cell = FieldCell::new(0, 6);
	// path from actor to goal sectors
	let mut path = portal_graph
		.find_best_path(
			(source_sector, source_field_cell),
			(target_sector, target_field_cell),
			&sector_portals,
			&sector_cost_fields,
		)
		.unwrap();
	filter_path(&mut path, target_field_cell);
	path.reverse();
	// create integration
	let route = Route::new(path);
	let mut int_builder = IntegrationBuilder::new(route, &sector_cost_fields);
	int_builder.expand_field_portals(&sector_portals, &sector_cost_fields, &map_dimensions);
	int_builder.calculate_los();
	int_builder.build_integrated_cost(&sector_cost_fields);
	// create flow
	let int_fields = int_builder.get_integration_fields();
	let mut sector_flow_fields = BTreeMap::new();
	for (i, (sector_id, goals, int_field)) in int_fields.iter().enumerate() {
		let mut flow_field = FlowField::default();
		if *sector_id == target_sector {
			flow_field.calculate(goals, None, int_field);
			sector_flow_fields.insert(*sector_id, flow_field);
		} else if let Some(dir_prev_sector) =
			Ordinal::sector_to_sector_direction(int_fields[i - 1].0, *sector_id)
		{
			let prev_int_field = &int_fields[i - 1].2;
			flow_field.calculate(goals, Some((dir_prev_sector, prev_int_field)), int_field);
			sector_flow_fields.insert(*sector_id, flow_field);
		};
	}

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
		// a centred box to contain the fields
		p.spawn((
			Node {
				width: Val::Px(1000.0),
				height: Val::Px(1000.0),
				flex_direction: FlexDirection::Column,
				flex_wrap: FlexWrap::Wrap,
				flex_shrink: 0.0,
				..Default::default()
			},
			BackgroundColor(Color::WHITE),
		))
		.with_children(|p| {
			// create an area for each sector int field
			for i in 0..map_dimensions.get_length() / 10 {
				for j in 0..map_dimensions.get_depth() / 10 {
					// bounding node of a sector
					p.spawn(Node {
						width: Val::Percent(100.0 / (map_dimensions.get_length() / 10) as f32),
						height: Val::Percent(100.0 / (map_dimensions.get_depth() / 10) as f32),
						flex_direction: FlexDirection::Column,
						flex_wrap: FlexWrap::Wrap,
						flex_shrink: 0.0,
						..Default::default()
					})
					.with_children(|p| {
						// the array area of the sector
						let flow_field = sector_flow_fields.get(&SectorID::new(i, j));
						if let Some(field) = flow_field {
							// create each column from the field
							for array in field.get().iter() {
								p.spawn(Node {
									width: Val::Percent(10.0),
									height: Val::Percent(100.0),
									flex_direction: FlexDirection::Column,
									..Default::default()
								})
								.with_children(|p| {
									// create each row value of the column
									for value in array.iter() {
										p.spawn((
											Node {
												width: Val::Percent(100.0),
												height: Val::Percent(10.0),
												justify_content: JustifyContent::Center,
												align_items: AlignItems::Center,
												..Default::default()
											},
											BackgroundColor(Color::WHITE),
											ImageNode::new(asset_server.load(get_ord_icon(*value))),
										));
									}
								});
							}
						}
					});
				}
			}
		});
	});
}
/// Get the asset path of ordinal icons
fn get_ord_icon(value: u8) -> String {
	// temp
	if value == 64 {
		return String::from("ordinal_icons/goal.png");
	}
	//
	if has_line_of_sight(value) {
		return String::from("ordinal_icons/los.png");
	}
	let ordinal = get_ordinal_from_bits(value);
	match ordinal {
		Ordinal::North => String::from("ordinal_icons/north.png"),
		Ordinal::East => String::from("ordinal_icons/east.png"),
		Ordinal::South => String::from("ordinal_icons/south.png"),
		Ordinal::West => String::from("ordinal_icons/west.png"),
		Ordinal::NorthEast => String::from("ordinal_icons/north_east.png"),
		Ordinal::SouthEast => String::from("ordinal_icons/south_east.png"),
		Ordinal::SouthWest => String::from("ordinal_icons/south_west.png"),
		Ordinal::NorthWest => String::from("ordinal_icons/north_west.png"),
		Ordinal::Zero => String::from("ordinal_icons/impassable.png"),
	}
}
