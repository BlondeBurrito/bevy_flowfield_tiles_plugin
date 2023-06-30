//! Defines the Bevy [Plugin] for FlowfieldTiles
//!

use bevy::prelude::*;

use super::{
	portal::portal_graph::PortalGraph,
	sectors::{SectorCostFields, SectorPortals},
	MapDimensions,
};

pub struct FlowFieldTilesPlugin;

impl Plugin for FlowFieldTilesPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<EventUpdateCostfieldsCell>()
			.add_event::<EventRebuildSectorPortals>()
			.add_event::<EventUpdatePortalGraph>()
			.add_systems(Update, (process_costfields_updates,))
			.add_systems(Update, (rebuild_portals, update_portal_graph).chain());
	}
}
/// Used to update a sectors [super::cost_fields::CostFields]
pub struct EventUpdateCostfieldsCell {
	cell: (usize, usize),
	sector: (u32, u32),
	cell_value: u8,
}

impl EventUpdateCostfieldsCell {
	pub fn new(cell: (usize, usize), sector: (u32, u32), cell_value: u8) -> Self {
		EventUpdateCostfieldsCell {
			cell,
			sector,
			cell_value,
		}
	}
	pub fn get_cell(&self) -> (usize, usize) {
		self.cell
	}
	pub fn get_sector(&self) -> (u32, u32) {
		self.sector
	}
	pub fn get_cost_value(&self) -> u8 {
		self.cell_value
	}
}
/// Read [EventUpdateCostfieldsCell] and update the values within [super::cost_fields::CostFields]
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
/// Emitted when a [super::cost_fields::CostFields] has been updated so the [super::portal::portals::Portals] of the sector
/// and its neighbours can be rebuilt
pub struct EventRebuildSectorPortals {
	sector_id: (u32, u32),
}

impl EventRebuildSectorPortals {
	pub fn new(sector_id: (u32, u32)) -> Self {
		EventRebuildSectorPortals { sector_id }
	}
	pub fn get_sector_id(&self) -> (u32, u32) {
		self.sector_id
	}
}
/// Process events indicating that a [super::cost_fields::CostFields] has changed and as such update
/// the [super::portal::portals::Portals] associated with the sector of the [super::cost_fields::CostFields] and its
/// neighbours need to be regenerated
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
		// queue an update the the portal graph
		event_update_graph.send(EventUpdatePortalGraph::new(sector_id));
	}
}

/// Emitted when [super::portal::portals::Portals] has been updated so the
/// [super::portal::portal_graph::PortalGraph] of the sector and its neighbours can be rebuilt
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

/// Process events indicating that a [super::portal::portals::Portals] has been changed and as such update
/// the navigation graph
pub fn update_portal_graph(
	mut event_graph: EventReader<EventRebuildSectorPortals>,
	mut portal_q: Query<(&mut PortalGraph, &SectorPortals, &SectorCostFields, &MapDimensions)>,
) {
	for event in event_graph.iter() {
		let sector_id = event.get_sector_id();
		for (mut portal_graph, sector_portals, sector_cost_fields, dimensions) in portal_q.iter_mut() {
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
