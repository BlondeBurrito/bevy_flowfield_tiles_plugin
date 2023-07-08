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
#[cfg(not(tarpaulin_include))]
pub fn handle_path_requests(
	mut events: EventReader<EventPathRequest>,
	mut cache_q: Query<(
		&mut RouteCache,
		&PortalGraph,
		&SectorPortals,
		&SectorCostFields,
		&MapDimensions,
	)>,
) {
	for event in events.iter() {
		for (mut cache, graph, sector_portals, sector_cost_fields, map_dimensions) in
			cache_q.iter_mut()
		{
			// only run if the cache doesn't contain the route already
			if !cache.get().contains_key(&(
				event.source_sector,
				event.target_sector,
				event.target_goal,
			)) {
				if let Some(node_route) = graph.find_best_path(
					(event.source_sector, event.source_field_cell),
					(event.target_sector, event.target_goal),
					sector_portals,
					sector_cost_fields,
				) {
					debug!("Portal path found");
					let mut path = graph
						.convert_index_path_to_sector_portal_cells(node_route.1, &sector_portals);
					if path.len() > 0 {
						// // original order is from actor to goal, int fields need to be processed the other way around
						// path.reverse();
						// change target cell from portal to the real goal for the destination
						let len = path.len();
						path[len - 1].1 = event.target_goal;
						// filter out the entry portals of sectors, we only care about the end of each sector and the end goal itself
						let mut sector_order = Vec::new();
						let mut map = HashMap::new();
						for p in path.iter() {
							if !map.contains_key(&p.0) {
								map.insert(p.0, p.1);
								sector_order.push(p.0);
							}
						}
						// reassemble to only include 1 element for each sector
						let mut sector_goals = Vec::new();
						for sector in sector_order.iter() {
							let (sector_id, portal_id) = map.get_key_value(sector).unwrap();
							sector_goals.push((*sector_id, *portal_id));
						}
						path = sector_goals;
					}
					cache.insert_route(
						event.source_sector,
						event.target_sector,
						event.target_goal,
						path,
					);
				} else {
					// a portal based route could not be found or the actor
					// is within the same sector as the goal, for the latter
					// we store a single element route
					debug!(
						"No portal path found, either local sector movement or just doesn't exist"
					);
					cache.insert_route(
						event.source_sector,
						event.target_sector,
						event.target_goal,
						vec![(event.target_sector, event.target_goal)],
					);
				}
			}
		}
	}
}
#[cfg(not(tarpaulin_include))]
pub fn generate_flow_fields(
	mut cache_q: Query<
		(
			&mut FlowFieldCache,
			&RouteCache,
			&PortalGraph,
			&SectorPortals,
			&SectorCostFields,
			&MapDimensions,
		),
		Changed<RouteCache>,
	>,
) {
	for (
		mut field_cache,
		route_cache,
		portal_graph,
		sector_portals,
		sector_cost_fields,
		map_dimensions,
	) in cache_q.iter_mut()
	{
		for (key, portal_path) in route_cache.get().iter() {
			// original order is from actor to goal, int fields need to be processed the other way around
			let mut path = portal_path.clone();
			path.reverse();
			let mut sectors_expanded_goals = Vec::new();
			for (i, (sector_id, goal)) in path.iter().enumerate() {
				// only run if a FlowField hasn't been generated
				if !field_cache.get().contains_key(&(*sector_id, *goal)) {
					// first element is always the end target, don't bother with portal expansion
					if i == 0 {
						sectors_expanded_goals.push((*sector_id, vec![*goal]));
					} else {
						// portals represent the boundary to another sector, a portal can be spread over
						// multple grid cells, expand the portal to provide multiple goal
						// targets for moving to another sector
						let neighbour_sector_id = path[i - 1].0;
						let g = sector_portals
							.get()
							.get(&sector_id)
							.unwrap()
							.expand_portal_into_goals(
								&sector_cost_fields,
								&sector_id,
								goal,
								&neighbour_sector_id,
								map_dimensions.get_column(),
								map_dimensions.get_row(),
							);
						sectors_expanded_goals.push((*sector_id, g));
					}
				}
			}
			// build the integration fields
			let mut sector_int_fields = Vec::new();
			for (sector_id, goals) in sectors_expanded_goals.iter() {
				let mut int_field = IntegrationField::new(goals);
				let cost_field = sector_cost_fields.get().get(sector_id).unwrap();
				int_field.calculate_field(goals, cost_field);
				sector_int_fields.push((*sector_id, goals.clone(), int_field));
			}
			// build the flow fields
			for (i, (sector_id, goals, int_field)) in sector_int_fields.iter().enumerate() {
				let mut flow_field = FlowField::default();
				// first element is end target, therefore has no info about previous sector for
				// direction optimisations
				if i == 0 {
					flow_field.calculate(goals, None, int_field);
					field_cache.insert_field(*sector_id, path[i].1, flow_field);
				} else {
					let dir_prev_sector =
						Ordinal::sector_to_sector_direction(sector_int_fields[i - 1].0, *sector_id);
					let prev_int_field = &sector_int_fields[i - 1].2;
					flow_field.calculate(goals, Some((dir_prev_sector, prev_int_field)), int_field);
					field_cache.insert_field(*sector_id, path[i].1, flow_field);
				}
			}
		}
	}
}
