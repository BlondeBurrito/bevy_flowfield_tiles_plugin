//! Defines a bundle which can be spawned as/inserted into an entity which
//! movable actors can query for pathing data
//!

use crate::prelude::*;
use bevy::prelude::*;

/// The length `x` and depth `z` (or `y` in 2d) of the map
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Default, Clone)]
pub struct MapDimensions(u32, u32);

impl MapDimensions {
	/// Create a new instance of [MapDimensions]. In 2d the dimensions should be measured by the number of sprites that fit into the `x` (length) and `y` (depth) axes. For 3d the recommendation is for a `unit` of space to be 1 meter, thereby the world is `x` (length) meters by `z` (depth) meters
	pub fn new(length: u32, depth: u32) -> Self {
		let length_rem = length % SECTOR_RESOLUTION as u32;
		let depth_rem = depth % SECTOR_RESOLUTION as u32;
		if length_rem > 0 || depth_rem > 0 {
			panic!(
				"Map dimensions `({}, {})` cannot support sectors, dimensions must be exact factors of {}",
				length, depth, SECTOR_RESOLUTION
			);
		}
		MapDimensions(length, depth)
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
pub struct FlowFieldTilesBundle {
	/// [CostField]s of all sectors
	sector_cost_fields: SectorCostFields,
	/// Portals for all sectors
	sector_portals: SectorPortals,
	/// Graph describing how to get from one sector to another
	portal_graph: PortalGraph,
	/// Size of the world
	map_dimensions: MapDimensions,
	/// Cache of overarching portal-portal routes
	route_cache: RouteCache,
	/// Cache of [FlowField]s that can be queried in a steering pipeline
	flow_field_cache: FlowFieldCache,
}

impl FlowFieldTilesBundle {
	/// Create a new instance of [FlowFieldTilesBundle] based on map dimensions
	pub fn new(map_length: u32, map_depth: u32) -> Self {
		let map_dimensions = MapDimensions::new(map_length, map_depth);
		let cost_fields = SectorCostFields::new(map_length, map_depth);
		let mut portals = SectorPortals::new(map_length, map_depth);
		// update default portals for cost fields
		for sector_id in cost_fields.get().keys() {
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
		let route_cache = RouteCache::default();
		let cache = FlowFieldCache::default();
		FlowFieldTilesBundle {
			sector_cost_fields: cost_fields,
			sector_portals: portals,
			portal_graph: graph,
			map_dimensions,
			route_cache,
			flow_field_cache: cache,
		}
	}
	/// Create a new instance of [FlowFieldTilesBundle] based on map dimensions where the [SectorCostFields] are derived from disk
	#[cfg(feature = "ron")]
	pub fn new_from_disk(map_length: u32, map_depth: u32, path: &str) -> Self {
		let map_dimensions = MapDimensions::new(map_length, map_depth);
		let cost_fields = SectorCostFields::from_file(path.to_string());
		let mut portals = SectorPortals::new(map_length, map_depth);
		// update default portals for cost fields
		for sector_id in cost_fields.get().keys() {
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
		let route_cache = RouteCache::default();
		let cache = FlowFieldCache::default();
		FlowFieldTilesBundle {
			sector_cost_fields: cost_fields,
			sector_portals: portals,
			portal_graph: graph,
			map_dimensions,
			route_cache,
			flow_field_cache: cache,
		}
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn valid_map_dimensions() {
		let _map_dimsions = MapDimensions::new(10, 10);
	}
	#[test]
	#[should_panic]
	fn invalid_map_dimensions() {
		MapDimensions::new(99, 3);
	}
	#[test]
	fn new_bundle() {
		let _ = FlowFieldTilesBundle::new(30, 30);
	}
}
