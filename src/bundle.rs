//!
//!

use crate::prelude::*;
use bevy::prelude::*;

/// The length `x` and depth `z` of the map
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Default)]
pub struct MapDimensions(u32, u32);

impl MapDimensions {
	pub fn new(x_length: u32, z_depth: u32) -> Self {
		//TODO some kind of check to ensure map isn;t too small, must be 3x3? sectors at least
		let x_sector_count = (x_length / SECTOR_RESOLUTION as u32).checked_sub(1);
		let z_sector_count = (z_depth / SECTOR_RESOLUTION as u32).checked_sub(1);
		if x_sector_count.is_none() || z_sector_count.is_none() {
			panic!(
				"Map dimensions `({}, {})` cannot support sectors, try larger values",
				x_length, z_depth
			);
		}
		MapDimensions(x_length, z_depth)
	}
	pub fn get_column(&self) -> u32 {
		self.0
	}
	pub fn get_row(&self) -> u32 {
		self.1
	}
}
//TODO #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Bundle)]
pub struct FlowfieldTilesBundle {
	sector_cost_fields: SectorCostFields,
	sector_portals: SectorPortals,
	portal_graph: PortalGraph,
	map_dimensions: MapDimensions,
	flow_field_cache: FlowFieldCache,
}

impl FlowfieldTilesBundle {
	pub fn new(map_length: u32, map_depth: u32) -> Self {
		let map_dimensions = MapDimensions::new(map_length, map_depth);
		let cost_fields = SectorCostFields::new(map_length, map_depth);
		let mut portals = SectorPortals::new(map_length, map_depth);
		// update default portals for cost fields
		for (sector_id, _v) in cost_fields.get() {
			portals.update_portals(
				*sector_id,
				&cost_fields,
				map_dimensions.get_column(),
				map_dimensions.get_row(),
			);
		}
		let graph = PortalGraph::new(
			&portals,
			&cost_fields,
			map_dimensions.get_column(),
			map_dimensions.get_row(),
		);
		let cache = FlowFieldCache::default();
		FlowfieldTilesBundle {
			sector_cost_fields: cost_fields,
			sector_portals: portals,
			portal_graph: graph,
			map_dimensions,
			flow_field_cache: cache,
		}
	}
}
