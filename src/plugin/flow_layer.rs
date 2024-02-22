//! Logic relating to [FlowField] generation
//!

use crate::prelude::*;
use bevy::prelude::*;

/// A request to queue up an attempt at generating a Route and a series of
/// [FlowField]s describing a path from the source to target
#[derive(Event)]
pub struct EventPathRequest {
	/// The starting sector of the request
	source_sector: SectorID,
	/// The starting field cell of the starting sector
	source_field_cell: FieldCell,
	/// The sector to try and find a path to
	target_sector: SectorID,
	/// The field cell in the target sector to find a path to
	target_goal: FieldCell,
}

impl EventPathRequest {
	pub fn new(
		source_sector: SectorID,
		source_field_cell: FieldCell,
		target_sector: SectorID,
		target_goal: FieldCell,
	) -> Self {
		EventPathRequest {
			source_sector,
			source_field_cell,
			target_sector,
			target_goal,
		}
	}
}

/// Process [EventPathRequest] and generate Routes to go into the [RouteCache] queue
#[cfg(not(tarpaulin_include))]
pub fn event_insert_route_queue(
	mut events: EventReader<EventPathRequest>,
	mut cache_q: Query<(
		&mut RouteCache,
		&PortalGraph,
		&SectorPortals,
		&SectorCostFields,
	)>,
	time: Res<Time>,
) {
	// several actors may send requests at once, instead of stepping through the events one at time
	// blitz thorugh duplicates so only a fresh request gets processed each tick - this is critical to perf
	let mut is_duplicate = true;
	while is_duplicate {
		if let Some(event) = events.read().next() {
			for (mut cache, graph, sector_portals, sector_cost_fields_scaled) in cache_q.iter_mut()
			{
				// only run if the cache doesn't contain the route already
				let rm = RouteMetadata::new(
					event.source_sector,
					event.source_field_cell,
					event.target_sector,
					event.target_goal,
					time.elapsed(),
				);
				if !cache.get().contains_key(&rm) {
					is_duplicate = false;
					if let Some(mut path) = graph.find_best_path(
						(event.source_sector, event.source_field_cell),
						(event.target_sector, event.target_goal),
						sector_portals,
						sector_cost_fields_scaled,
					) {
						debug!("Portal path found");
						if !path.is_empty() {
							filter_path(&mut path, event.target_goal);
						}
						cache.add_to_queue(
							event.source_sector,
							event.source_field_cell,
							event.target_sector,
							event.target_goal,
							time.elapsed(),
							path,
						);
					} else {
						// a portal based route could not be found or the actor
						// is within the same sector as the goal
						debug!(
						"No portal path found, either local sector movement or just doesn't exist"
					);
						if let Some(cost_field) = sector_cost_fields_scaled
							.get_scaled()
							.get(&event.target_sector)
						{
							let vis = cost_field.can_internal_portal_pair_see_each_other(
								event.source_field_cell,
								event.target_goal,
							);
							// if the two cells are reachable from within the same sector
							// then there is a local route
							if vis.0 {
								cache.add_to_queue(
									event.source_sector,
									event.source_field_cell,
									event.target_sector,
									event.target_goal,
									time.elapsed(),
									vec![(event.target_sector, event.target_goal)],
								);
							}
						}
					}
				}
			}
		} else {
			is_duplicate = false;
		}
	}
}

/// Generated portal-portal routes contain two elements for each sector, one
/// for an actors entry and when for an actors exit, we only need to know
/// about the elements which an actor would use to exit the sector so we filter
/// the route and trim it down
pub fn filter_path(path: &mut Vec<(SectorID, FieldCell)>, target_goal: FieldCell) {
	let mut path_based_on_portal_exits = Vec::new();
	// target sector and entry portal where we switch the entry portal cell to the goal
	let mut end = path.pop().unwrap();
	end.1 = target_goal;
	// sector and field of leaving starting sector if source sector and target sector are different
	// otherwise it was a single element path and we already removed it
	if !path.is_empty() {
		let start = path.remove(0);
		path_based_on_portal_exits.push(start);
	}
	// all other elements in the path are in pairs for entering and leaving sectors on the way to the goal
	for p in path.iter().skip(1).step_by(2) {
		path_based_on_portal_exits.push(*p);
	}
	path_based_on_portal_exits.push(end);
	*path = path_based_on_portal_exits;
}

/// Remove items from the queue of the [RouteCache] and promote them as routes
/// which an actor can use as a high-level pathfinding route while publishing a
/// new item into the [FlowFieldCache] queue
#[cfg(not(tarpaulin_include))]
pub fn process_route_queue(mut cache_q: Query<(&mut RouteCache, &mut FlowFieldCache)>) {
	for (mut r_cache, mut f_cache) in &mut cache_q {
		while let Some((metadata, path_to_goal)) = r_cache.get_queue_mut().pop_first() {
			let mut path_from_goal = path_to_goal.clone();
			path_from_goal.reverse();
			// store a route from actor to goal so that can actor can use it for high-level pathfinding while the more accurate flowfield representation gets built in the background
			r_cache.insert_route_with_metadata(metadata, path_to_goal);
			// add the route from goal to actor into the flowfield cache queue
			f_cache.add_to_queue(metadata, path_from_goal);
		}
	}
}

/// Inspect the [FlowFieldCache] queue and if the [IntegrationField]s of the
/// first entry haven't been created then calculate them
#[cfg(not(tarpaulin_include))]
pub fn create_queued_integration_fields(
	mut cache_q: Query<(
		&mut FlowFieldCache,
		&SectorPortals,
		&SectorCostFields,
		&MapDimensions,
	)>,
) {
	for (mut f_cache, sector_portals, sector_cost_fields_scaled, map_dimensions) in &mut cache_q {
		if let Some(mut entry) = f_cache.get_queue_mut().first_entry() {
			if entry.get().is_pending() {
				let sectors_expanded_goals = exapnd_sector_portals(
					entry.get().get_path(),
					sector_portals,
					sector_cost_fields_scaled,
					map_dimensions,
				);
				// build the integration fields
				let sector_int_fields =
					build_integration_fields(&sectors_expanded_goals, sector_cost_fields_scaled);
				entry.get_mut().add_integration_fields(sector_int_fields);
			}
		}
	}
}

/// Portals may represent multiple [FieldCell]s along a boundary, expand them
/// to provide multiple goal [FieldCell]s for crossing from one secgtor to another
#[cfg(not(tarpaulin_include))]
fn exapnd_sector_portals(
	path: &[(SectorID, FieldCell)],
	sector_portals: &SectorPortals,
	sector_cost_fields_scaled: &SectorCostFields,
	map_dimensions: &MapDimensions,
) -> Vec<(SectorID, Vec<FieldCell>)> {
	let mut sectors_expanded_goals = Vec::new();
	for (i, (sector_id, goal)) in path.iter().enumerate() {
		// first element is always the end target, don't bother with portal expansion
		if i == 0 {
			sectors_expanded_goals.push((*sector_id, vec![*goal]));
		} else {
			// portals represent the boundary to another sector, a portal can be spread over
			// multple field cells, expand the portal to provide multiple goal
			// targets for moving to another sector
			let neighbour_sector_id = path[i - 1].0;
			let g = sector_portals
				.get()
				.get(sector_id)
				.unwrap()
				.expand_portal_into_goals(
					sector_cost_fields_scaled,
					sector_id,
					goal,
					&neighbour_sector_id,
					map_dimensions,
				);
			sectors_expanded_goals.push((*sector_id, g));
		}
	}
	sectors_expanded_goals
}

/// Iterate over each sector with the expanded portal goals and calculate the
/// [IntegrationField] for it
#[cfg(not(tarpaulin_include))]
fn build_integration_fields(
	sectors_expanded_goals: &[(SectorID, Vec<FieldCell>)],
	sector_cost_fields_scaled: &SectorCostFields,
) -> Vec<(SectorID, Vec<FieldCell>, IntegrationField)> {
	let mut sector_int_fields = Vec::new();
	for (sector_id, goals) in sectors_expanded_goals.iter() {
		let mut int_field = IntegrationField::new(goals);
		let cost_field = sector_cost_fields_scaled
			.get_scaled()
			.get(sector_id)
			.unwrap();
		int_field.calculate_field(goals, cost_field);
		sector_int_fields.push((*sector_id, goals.clone(), int_field));
	}
	sector_int_fields
}

/// When a queued item has had its [IntegrationField]s built generate the
/// [FlowField]s for it
#[cfg(not(tarpaulin_include))]
pub fn create_flow_fields(mut cache_q: Query<&mut FlowFieldCache>, time: Res<Time>) {
	for mut field_cache in &mut cache_q {
		if let Some(mut entry) = field_cache.get_queue_mut().first_entry() {
			// if the integration fields havbe been created then remove form queue and calculate flowfields
			if !entry.get_mut().is_pending() {
				let int = entry.remove();
				let sector_int_fields = int.get_integration_fields().as_deref().unwrap();
				let path = int.get_path();
				// build the flow fields
				for (i, (sector_id, goals, int_field)) in sector_int_fields.iter().enumerate() {
					let mut flow_field = FlowField::default();
					// first element is end target, therefore has no info about previous sector for
					// direction optimisations
					if i == 0 {
						flow_field.calculate(goals, None, int_field);
						field_cache.insert_field(*sector_id, path[i].1, time.elapsed(), flow_field);
					} else if let Some(dir_prev_sector) =
						Ordinal::sector_to_sector_direction(sector_int_fields[i - 1].0, *sector_id)
					{
						let prev_int_field = &sector_int_fields[i - 1].2;
						flow_field.calculate(
							goals,
							Some((dir_prev_sector, prev_int_field)),
							int_field,
						);
						//TODO by using the portal goal from path[i].1 actors criss-crossing from two seperate routes means one will use the others route in a sector which may be less efficient then using thier own?
						field_cache.insert_field(*sector_id, path[i].1, time.elapsed(), flow_field);
					} else {
						error!("Route from goal to actor {:?}", path);
					};
				}
			}
		}
	}
}

/// Purge any routes older than 15 minutes
#[cfg(not(tarpaulin_include))]
pub fn cleanup_old_routes(mut q_route_cache: Query<&mut RouteCache>, time: Res<Time>) {
	for mut cache in q_route_cache.iter_mut() {
		let mut routes_to_purge = Vec::new();
		for data in cache.get_mut().keys() {
			let elapsed = time.elapsed();
			let diff = elapsed.saturating_sub(data.get_time_generated());
			if diff.as_secs() > 900 {
				routes_to_purge.push(*data);
			}
		}
		for purge in routes_to_purge.iter() {
			cache.remove_route(*purge);
		}
	}
}
/// Purge any [FlowField]s older than 15 minutes
#[cfg(not(tarpaulin_include))]
pub fn cleanup_old_flowfields(mut q_flow_cache: Query<&mut FlowFieldCache>, time: Res<Time>) {
	for mut cache in q_flow_cache.iter_mut() {
		let mut routes_to_purge = Vec::new();
		for data in cache.get_mut().keys() {
			let elapsed = time.elapsed();
			let diff = elapsed.saturating_sub(data.get_time_generated());
			if diff.as_secs() > 900 {
				routes_to_purge.push(*data);
			}
		}
		for purge in routes_to_purge.iter() {
			cache.remove_field(*purge);
		}
	}
}
#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn filter_graph_route() {
		// path in 3x3 sector grid, moving from top right to bottom left
		let mut path: Vec<(SectorID, FieldCell)> = vec![
			(SectorID::new(2, 0), FieldCell::new(0, 4)), // start sector and exit
			(SectorID::new(1, 0), FieldCell::new(9, 4)), // entry portal of next sector
			(SectorID::new(1, 0), FieldCell::new(3, 9)), // exit portal of next sector
			(SectorID::new(1, 1), FieldCell::new(3, 0)), // entry portal of next sector
			(SectorID::new(1, 1), FieldCell::new(5, 9)), // exit portal of next sector
			(SectorID::new(1, 2), FieldCell::new(5, 0)), // entry portal of next sector
			(SectorID::new(1, 2), FieldCell::new(0, 3)), // exit portal of next sector
			(SectorID::new(0, 2), FieldCell::new(9, 3)) // goal sector and entry portal
		];
		let target_goal = FieldCell::new(4, 4);

		filter_path(&mut path, target_goal);
		let actual = vec![
			(SectorID::new(2, 0), FieldCell::new(0, 4)),
			(SectorID::new(1, 0), FieldCell::new(3, 9)),
			(SectorID::new(1, 1), FieldCell::new(5, 9)),
			(SectorID::new(1, 2), FieldCell::new(0, 3)),
			(SectorID::new(0, 2), FieldCell::new(4, 4)) // gets switch to target_goal
		];

		assert_eq!(actual, path);
	}

	#[test]
	fn filter_graph_route_back_on_itself() {
		// path in 3x3 sector grid, moving from top right to top right
		// i.e impassable values mean that the actor must leave its starting sector and
		// re-enter it from a different portal
		let mut path: Vec<(SectorID, FieldCell)> = vec![
			(SectorID::new(2, 0), FieldCell::new(8, 9)), // start sector and exit
			(SectorID::new(2, 1), FieldCell::new(8, 0)), // entry portal of next sector
			(SectorID::new(2, 1), FieldCell::new(6, 0)), // exit back towards start sector
			(SectorID::new(2, 0), FieldCell::new(6, 9)), // entry back into start sector
			(SectorID::new(2, 0), FieldCell::new(4, 9)), // leave starting sector again
			(SectorID::new(2, 1), FieldCell::new(4, 0)), // entry of neighbour again
			(SectorID::new(2, 1), FieldCell::new(2, 0)), // exit back towrards start again
			(SectorID::new(2, 0), FieldCell::new(2, 9)), // last entry into original sector
		];
		let target_goal = FieldCell::new(2, 1);

		filter_path(&mut path, target_goal);
		let actual = vec![
			(SectorID::new(2, 0), FieldCell::new(8, 9)),
			(SectorID::new(2, 1), FieldCell::new(6, 0)),
			(SectorID::new(2, 0), FieldCell::new(4, 9)),
			(SectorID::new(2, 1), FieldCell::new(2, 0)),
			(SectorID::new(2, 0), FieldCell::new(2, 1)), // gets switch to target_goal
		];

		assert_eq!(actual, path);
	}

}
