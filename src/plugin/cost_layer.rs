//! Logic for handling changes to a [CostField] which in turn updates Portals,
//! [PortalGraph] and cleaning of cached routes which may of been made invalid
//! by the cost change
//!

use crate::prelude::*;
use bevy::prelude::*;

/// Used to update a sectors [CostField]
#[derive(Event)]
pub struct EventUpdateCostfieldsCell {
	/// FieldCell to update
	cell: FieldCell,
	/// The sector the field cell resides in
	sector: SectorID,
	/// The value the field cell should be assigned
	cell_value: u8,
}

impl EventUpdateCostfieldsCell {
	/// Create a new instance of [EventUpdateCostfieldsCell]
	#[cfg(not(tarpaulin_include))]
	pub fn new(cell: FieldCell, sector: SectorID, cell_value: u8) -> Self {
		EventUpdateCostfieldsCell {
			cell,
			sector,
			cell_value,
		}
	}
	#[cfg(not(tarpaulin_include))]
	pub fn get_cell(&self) -> FieldCell {
		self.cell
	}
	#[cfg(not(tarpaulin_include))]
	pub fn get_sector(&self) -> SectorID {
		self.sector
	}
	#[cfg(not(tarpaulin_include))]
	pub fn get_cost_value(&self) -> u8 {
		self.cell_value
	}
}

/// Read [EventUpdateCostfieldsCell] and update the values within [CostField]
#[cfg(not(tarpaulin_include))]
pub fn process_costfields_updates(
	mut events: EventReader<EventUpdateCostfieldsCell>,
	mut query: Query<(
		&mut PortalGraph,
		&mut SectorPortals,
		&mut SectorCostFields,
		&mut SectorCostFieldsScaled,
		&MapDimensions,
	)>,
	mut event_cache_clean: EventWriter<EventCleanCaches>,
) {
	// coalesce events to avoid processing duplicates
	let mut coalesced_sectors = Vec::new();
	for event in events.iter() {
		let field_cell = event.get_cell();
		let sector_id = event.get_sector();
		let cost = event.get_cost_value();
		for (
			_portal_graph,
			mut sector_portals,
			mut sector_cost_fields,
			mut sector_cost_fields_scaled,
			dimensions,
		) in query.iter_mut()
		{
			if let Some(field) = sector_cost_fields.get_mut().get_mut(&sector_id) {
				field.set_field_cell_value(cost, field_cell);
				sector_cost_fields_scaled.update(
					&sector_cost_fields,
					&sector_id,
					field,
					dimensions.get_actor_scale(),
				);
				// update the portals of the sector and around it
				sector_portals.update_portals(
					sector_id,
					sector_cost_fields_scaled.as_ref(),
					dimensions,
				);
			}
			// for (sector, field) in sector_cost_fields.get_mut().iter_mut() {
			// 	if *sector == sector_id {
			// 		field.set_field_cell_value(cost, field_cell);
			// 		sector_cost_fields_scaled.update(
			// 		&mut sector_cost_fields,
			// 		&sector_id,
			// 		field,
			// 		dimensions.get_actor_scale(),
			// 	);
			// 	// update the portals of the sector and around it
			// 	sector_portals.update_portals(
			// 		sector_id,
			// 		sector_cost_fields_scaled.as_ref(),
			// 		dimensions,
			// 	);
			// }}
		}
		if !coalesced_sectors.contains(&sector_id) {
			coalesced_sectors.push(sector_id);
		}
	}
	for sector_id in coalesced_sectors.iter() {
		debug!("Rebuilding fields of {:?}", sector_id.get());
		for (
			mut portal_graph,
			sector_portals,
			_sector_cost_fields,
			sector_cost_fields_scaled,
			dimensions,
		) in query.iter_mut()
		{
			// update the graph
			portal_graph.update_graph(
				*sector_id,
				sector_portals.as_ref(),
				sector_cost_fields_scaled.as_ref(),
				dimensions,
			);
		}
		event_cache_clean.send(EventCleanCaches(*sector_id));
	}
}

/// For the given sector any route or [FlowField] making use of it needs to have the cached entry removed and a new request made to regenerate the route
#[derive(Event)]
pub struct EventCleanCaches(SectorID);

//TODO in order to regenerate the routes the source field cell needs to be known - a cost field change may make that field cell invalid though... disable regen for now, steering pipeline/character controler will have to poll the route cache and request a new one....
/// Lookup any cached data records making use of sectors that have had their [CostField] adjusted and remove them from the cache
#[cfg(not(tarpaulin_include))]
pub fn clean_cache(
	mut events: EventReader<EventCleanCaches>,
	mut q_flow: Query<&mut FlowFieldCache>,
	mut q_route: Query<&mut RouteCache>,
	// mut event_path_request: EventWriter<EventPathRequest>,
) {
	let mut sectors = Vec::new();
	for event in events.iter() {
		sectors.push(event.0);
	}
	if !sectors.is_empty() {
		let mut to_purge = Vec::new();
		for mut flow_cache in q_flow.iter_mut() {
			let map = flow_cache.get_mut();
			for id in sectors.iter() {
				for metadata in map.keys() {
					if *id == metadata.get_sector_id() {
						to_purge.push(*metadata);
					}
				}
			}
			for purge_me in to_purge.iter() {
				flow_cache.remove_field(*purge_me);
			}
		}
		let mut to_purge = Vec::new();
		for mut route_cache in q_route.iter_mut() {
			let map = route_cache.get_mut();
			for id in sectors.iter() {
				'next: for (metadata, route) in map.iter() {
					if *id == metadata.get_source_sector() {
						to_purge.push(*metadata);
						continue 'next;
					}
					if *id == metadata.get_target_sector() {
						to_purge.push(*metadata);
						continue 'next;
					}
					for (route_sector, _) in route.iter() {
						if *id == *route_sector {
							to_purge.push(*metadata);
							continue 'next;
						}
					}
				}
			}
			for purge_me in to_purge.iter() {
				route_cache.remove_route(*purge_me);
			}
		}
		// // send events to regenerate routes
		// for metadata in to_purge.iter() {
		// 	//TODO someway of getting the orignal source_field_cell instead of (5,5) assumption
		// 	event_path_request.send(EventPathRequest::new(
		// 		metadata.get_source_sector(),
		// 		(5, 5),
		// 		metadata.get_target_sector(),
		// 		metadata.get_target_goal(),
		// 	))
		// }
	}
}
