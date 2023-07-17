//!
//!

use crate::prelude::*;
use bevy::prelude::*;

/// Used to update a sectors [CostField]
#[derive(Event)]
pub struct EventUpdateCostfieldsCell {
	cell: (usize, usize),
	sector: (u32, u32),
	cell_value: u8,
}

impl EventUpdateCostfieldsCell {
	/// Create a new instance of [EventUpdateCostfieldsCell]
	#[cfg(not(tarpaulin_include))]
	pub fn new(cell: (usize, usize), sector: (u32, u32), cell_value: u8) -> Self {
		EventUpdateCostfieldsCell {
			cell,
			sector,
			cell_value,
		}
	}
	#[cfg(not(tarpaulin_include))]
	pub fn get_cell(&self) -> (usize, usize) {
		self.cell
	}
	#[cfg(not(tarpaulin_include))]
	pub fn get_sector(&self) -> (u32, u32) {
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
	mut costfields_q: Query<&mut SectorCostFields>,
	mut event_portal_rebuild: EventWriter<EventRebuildSectorPortals>,
) {
	for event in events.iter() {
		let grid_cell = event.get_cell();
		let sector_id = event.get_sector();
		let cost = event.get_cost_value();
		for mut costfields in costfields_q.iter_mut() {
			for (sector, field) in costfields.get_mut().iter_mut() {
				if *sector == sector_id {
					field.set_grid_value(cost, grid_cell.0, grid_cell.1);
					event_portal_rebuild.send(EventRebuildSectorPortals::new(sector_id));
				}
			}
		}
	}
}
/// Emitted when a [CostField] has been updated so the [Portals] of the sector
/// and its neighbours can be rebuilt
#[derive(Event)]
pub struct EventRebuildSectorPortals {
	sector_id: (u32, u32),
}

impl EventRebuildSectorPortals {
	#[cfg(not(tarpaulin_include))]
	pub fn new(sector_id: (u32, u32)) -> Self {
		EventRebuildSectorPortals { sector_id }
	}
	#[cfg(not(tarpaulin_include))]
	pub fn get_sector_id(&self) -> (u32, u32) {
		self.sector_id
	}
}
/// Process events indicating that a [CostField] has changed and as such update
/// the [Portals] associated with the sector of the [CostField] and its
/// neighbours need to be regenerated
#[cfg(not(tarpaulin_include))]
pub fn rebuild_portals(
	mut event_portal_rebuild: EventReader<EventRebuildSectorPortals>,
	mut portal_q: Query<(&mut SectorPortals, &SectorCostFields, &MapDimensions)>,
	mut event_update_graph: EventWriter<EventUpdatePortalGraph>,
) {
	for event in event_portal_rebuild.iter() {
		let sector_id = event.get_sector_id();
		// update the portals of the sector and around it
		for (mut sector_portals, sector_cost_fields, dimensions) in portal_q.iter_mut() {
			sector_portals.update_portals(
				sector_id,
				sector_cost_fields,
				dimensions.get_column(),
				dimensions.get_row(),
			);
		}
		// queue an update to the portal graph
		event_update_graph.send(EventUpdatePortalGraph::new(sector_id));
	}
}

/// Emitted when [Portals] has been updated so the
/// [PortalGraph] of the sector and its neighbours can be rebuilt
#[derive(Event)]
pub struct EventUpdatePortalGraph {
	sector_id: (u32, u32),
}

impl EventUpdatePortalGraph {
	pub fn new(sector_id: (u32, u32)) -> Self {
		EventUpdatePortalGraph { sector_id }
	}
	pub fn get_sector_id(&self) -> (u32, u32) {
		self.sector_id
	}
}

/// Process events indicating that a [Portals] has been changed and as such update
/// the navigation graph
#[cfg(not(tarpaulin_include))]
pub fn update_portal_graph(
	mut event_graph: EventReader<EventRebuildSectorPortals>,
	mut portal_q: Query<(
		&mut PortalGraph,
		&SectorPortals,
		&SectorCostFields,
		&MapDimensions,
	)>,
) {
	for event in event_graph.iter() {
		let sector_id = event.get_sector_id();
		for (mut portal_graph, sector_portals, sector_cost_fields, dimensions) in
			portal_q.iter_mut()
		{
			portal_graph.update_graph(
				sector_id,
				sector_portals,
				sector_cost_fields,
				dimensions.get_column(),
				dimensions.get_row(),
			);
		}
	}
}
