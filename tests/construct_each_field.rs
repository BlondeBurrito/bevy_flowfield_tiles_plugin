//! Try building each field
//!

use std::collections::{BTreeMap, HashMap};

use bevy_flowfield_tiles_plugin::prelude::*;

#[test]
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
	let mut path = portal_graph
		.find_best_path(
			(source_sector, source_field_cell),
			(target_sector, target_field_cell),
			&sector_portals,
			&sector_cost_fields,
		)
		.unwrap();
	// original order is from actor to goal, int fields need to be processed the other way around
	path.reverse();
	// change target cell from portal to the real goal
	path[0].1 = target_field_cell;
	let mut sector_order = Vec::new();
	let mut map = HashMap::new();
	for p in path.iter() {
		if let std::collections::hash_map::Entry::Vacant(e) = map.entry(p.0) {
			e.insert(p.1);
			sector_order.push(p.0);
		}
	}
	let mut sector_goals = Vec::new();
	for (i, sector) in sector_order.iter().enumerate() {
		let (sector_id, portal_id) = map.get_key_value(sector).unwrap();
		if *sector == target_sector {
			sector_goals.push((*sector_id, vec![*portal_id]));
		} else {
			let neighbour_sector_id = sector_order[i - 1];
			let g = sector_portals
				.get()
				.get(sector_id)
				.unwrap()
				.expand_portal_into_goals(
					&sector_cost_fields,
					sector_id,
					portal_id,
					&neighbour_sector_id,
					&map_dimensions,
				);
			sector_goals.push((*sector_id, g));
		}
	}
	// prep int fields
	let mut sector_int_fields = Vec::new();
	for (sector_id, goals) in sector_goals.iter() {
		let mut int_field = IntegrationField::new(goals);
		let cost_field = sector_cost_fields.get_scaled().get(sector_id).unwrap();
		int_field.calculate_field(goals, cost_field);
		sector_int_fields.push((*sector_id, goals.clone(), int_field));
	}
	// create flow fields
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
