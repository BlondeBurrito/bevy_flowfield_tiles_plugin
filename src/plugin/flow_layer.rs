//!
//!

use std::collections::HashMap;

use crate::prelude::*;
use bevy::prelude::*;

pub struct EventPathRequest {
	source_sector: (u32, u32),
	source_field_cell: (usize, usize),
	target_sector: (u32, u32),
	target_goal: (usize, usize),
}

impl EventPathRequest {
	pub fn new(
		source_sector: (u32, u32),
		source_field_cell: (usize, usize),
		target_sector: (u32, u32),
		target_goal: (usize, usize),
	) -> Self {
		EventPathRequest {
			source_sector,
			source_field_cell,
			target_sector,
			target_goal,
		}
	}
}

pub fn handle_path_requests(
	mut events: EventReader<EventPathRequest>,
	mut cache_q: Query<(
		&mut FlowFieldCache,
		&PortalGraph,
		&SectorPortals,
		&SectorCostFields,
		&MapDimensions,
	)>,
) {
	for event in events.iter() {
		for (mut cache, graph, sector_portals, sector_cost_fields, map_dimensions) in cache_q.iter_mut() {
			if let Some(node_route) = graph.find_best_path(
				(event.source_sector, event.source_field_cell),
				(event.target_sector, event.target_goal),
				sector_portals,
				sector_cost_fields,
			) {
				info!("hi");
				let mut path =
					graph.convert_index_path_to_sector_portal_cells(node_route.1, &sector_portals);
				// 	if path.len() > 0 {
				// 	// original order is from actor to goal, int fields need to be processed the other way around
				// 	path.reverse();
				// 	// change target cell from portal to the real goal for the destination
				// 	path[0].1 = event.target_goal;
				// 	let mut sector_order = Vec::new();
				// 	let mut map = HashMap::new();
				// 	for p in path.iter() {
				// 		if !map.contains_key(&p.0) {
				// 			map.insert(p.0, p.1);
				// 			sector_order.push(p.0);
				// 		}
				// 	}
				// 	let mut sector_goals = Vec::new();
				// 	for (i, sector) in sector_order.iter().enumerate() {
				// 		let (sector_id, portal_id) = map.get_key_value(sector).unwrap();
				// 		if *sector == event.target_sector {
				// 			sector_goals.push((*sector_id, vec![*portal_id]));
				// 		} else {
				// 			let neighbour_sector_id = sector_order[i - 1];
				// 			let g = sector_portals
				// 				.get()
				// 				.get(&sector_id)
				// 				.unwrap()
				// 				.expand_portal_into_goals(
				// 					&sector_cost_fields,
				// 					&sector_id,
				// 					portal_id,
				// 					&neighbour_sector_id,
				// 					map_dimensions.get_column(),
				// 					map_dimensions.get_row(),
				// 				);
				// 			sector_goals.push((*sector_id, g));
				// 		}
				// 	}
				// }
				// 	cache.insert_route(source_sector, target_sector, goal_id, route);
			} else {
				// a portal based route could not be found or the actor
				// is within the same sector as the goal, for the latter
				// we store a single element route
				cache.insert_route(event.source_sector, event.target_sector, event.target_goal, vec![(event.target_sector, event.target_goal)]);
			}
		}
	}
}
