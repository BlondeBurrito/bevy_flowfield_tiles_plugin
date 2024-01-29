//! Defines a bundle which can be spawned as/inserted into an entity which
//! movable actors can query for pathing data
//!

use crate::prelude::*;
use bevy::prelude::*;

//TODO #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
/// Defines all required components for generating [FlowField] Tiles
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
	pub fn new(map_length: u32, map_depth: u32, sector_resolution: u32, actor_size: f32) -> Self {
		let map_dimensions =
			MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
		let cost_fields = SectorCostFields::new(&map_dimensions);
		let mut portals = SectorPortals::new(map_length, map_depth, sector_resolution);
		// update default portals for cost fields
		for sector_id in cost_fields.get_scaled().keys() {
			portals.update_portals(*sector_id, &cost_fields, &map_dimensions);
		}
		let graph = PortalGraph::new(&portals, &cost_fields, &map_dimensions);
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
	/// Create a new instance of [FlowFieldTilesBundle] based on map dimensions where the [SectorCostFields] are derived from a `.ron` file
	#[cfg(feature = "ron")]
	pub fn from_ron(
		map_length: u32,
		map_depth: u32,
		sector_resolution: u32,
		actor_size: f32,
		path: &str,
	) -> Self {
		let map_dimensions =
			MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
		let cost_fields = SectorCostFields::from_ron(path.to_string(), &map_dimensions);
		if ((map_length * map_depth) / (sector_resolution * sector_resolution)) as usize
			!= cost_fields.get_baseline().len()
		{
			panic!("Map size ({}, {}) with resolution {} produces ({}x{}) sectors. Ron file only produces {} sectors", map_length, map_depth, sector_resolution, map_length/sector_resolution, map_depth/sector_resolution, cost_fields.get_baseline().len());
		}
		let mut portals = SectorPortals::new(map_length, map_depth, sector_resolution);
		// update default portals for cost fields
		for sector_id in cost_fields.get_scaled().keys() {
			portals.update_portals(*sector_id, &cost_fields, &map_dimensions);
		}
		let graph = PortalGraph::new(&portals, &cost_fields, &map_dimensions);
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
	/// Create a new instance of [FlowFieldTilesBundle] from a directory containing CSV [CostField] files
	#[cfg(not(tarpaulin_include))]
	#[cfg(feature = "csv")]
	pub fn from_csv(
		map_length: u32,
		map_depth: u32,
		sector_resolution: u32,
		actor_size: f32,
		directory: &str,
	) -> Self {
		let map_dimensions =
			MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
		let cost_fields = SectorCostFields::from_csv_dir(&map_dimensions, directory.to_string());
		let mut portals = SectorPortals::new(map_length, map_depth, sector_resolution);
		// update default portals for cost fields
		for sector_id in cost_fields.get_scaled().keys() {
			portals.update_portals(*sector_id, &cost_fields, &map_dimensions);
		}
		let graph = PortalGraph::new(&portals, &cost_fields, &map_dimensions);
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
		let _map_dimsions = MapDimensions::new(10, 10, 10, 0.5);
	}
	#[test]
	#[should_panic]
	fn invalid_map_dimensions() {
		MapDimensions::new(99, 3, 10, 1.0);
	}
	#[test]
	fn new_bundle() {
		let _ = FlowFieldTilesBundle::new(30, 30, 10, 0.5);
	}
	#[test]
	fn new_bundle_from_ron() {
		let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields_continuous_layout.ron";
		let _ = FlowFieldTilesBundle::from_ron(30, 30, 10, 0.5, &path);
	}
}
