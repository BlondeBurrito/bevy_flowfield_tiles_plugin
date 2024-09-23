//! Try building each field
//!

use std::collections::BTreeMap;

use bevy_flowfield_tiles_plugin::prelude::*;

#[test]
/// Try building a set of FlowFields
fn field_on_field() {
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let map_dimensions = MapDimensions::new(30, 30, 10, 0.5);
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
	let route = Route::new(path);
	// build integration layer
	let mut int_builder = IntegrationBuilder::new(route, &sector_cost_fields);
	int_builder.expand_field_portals(&sector_portals, &sector_cost_fields, &map_dimensions);
	int_builder.calculate_los();
	int_builder.build_integrated_cost(&sector_cost_fields);
	// create flow fields
	let sector_int_fields = int_builder.get_integration_fields();
	let mut sector_flow_fields = BTreeMap::new();
	for (i, (sector_id, goals, int_field)) in sector_int_fields.iter().enumerate() {
		let mut flow_field = FlowField::default();
		if *sector_id == target_sector {
			flow_field.calculate(goals, None, int_field);
			sector_flow_fields.insert(*sector_id, flow_field);
		} else if let Some(dir_prev_sector) =
			Ordinal::sector_to_sector_direction(sector_int_fields[i - 1].0, *sector_id)
		{
			let prev_int_field = &sector_int_fields[i - 1].2;
			flow_field.calculate(goals, Some((dir_prev_sector, prev_int_field)), int_field);
			sector_flow_fields.insert(*sector_id, flow_field);
		};
	}
}
