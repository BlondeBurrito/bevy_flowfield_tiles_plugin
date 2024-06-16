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
	/// Get a reference to the [SectorCostFields]
	pub fn get_sector_cost_fields(&self) -> &SectorCostFields {
		&self.sector_cost_fields
	}
	/// Get a reference to the [SectorPortals]
	pub fn get_sector_portals(&self) -> &SectorPortals {
		&self.sector_portals
	}
	/// Get a reference to the [PortalGraph]
	pub fn get_portal_graph(&self) -> &PortalGraph {
		&self.portal_graph
	}
	/// Get a reference to the [MapDimensions]
	pub fn get_map_dimensions(&self) -> &MapDimensions {
		&self.map_dimensions
	}
	/// Get a reference to the [RouteCache]
	pub fn get_route_cache(&self) -> &RouteCache {
		&self.route_cache
	}
	/// Get a mutable reference to the [RouteCache]
	pub fn get_route_cache_mut(&mut self) -> &mut RouteCache {
		&mut self.route_cache
	}
	/// Get a reference to the [FlowFieldCache]
	pub fn get_flowfield_cache(&self) -> &FlowFieldCache {
		&self.flow_field_cache
	}
	/// Get a mutable reference to the [FlowFieldCache]
	pub fn get_flowfield_cache_mut(&mut self) -> &mut FlowFieldCache {
		&mut self.flow_field_cache
	}
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
	/// From a greyscale heightmap image initialise a bundle where the
	/// [CostField]s are derived from the pixel values of the image
	#[cfg(not(tarpaulin_include))]
	#[cfg(feature = "heightmap")]
	pub fn from_heightmap(
		map_length: u32,
		map_depth: u32,
		sector_resolution: u32,
		actor_size: f32,
		file_path: &str,
	) -> Self {
		let map_dimensions =
			MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
		let cost_fields = SectorCostFields::from_heightmap(&map_dimensions, file_path.to_string());
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
	/// From a list of 2d meshes and their translation initialise a bundle. The vertex points of the meshes must be within the `map_length` and `map_depth` of the world
	#[cfg(not(tarpaulin_include))]
	#[cfg(feature = "2d")]
	pub fn from_bevy_2d_meshes(
		meshes: Vec<(&Mesh, Vec2)>,
		map_length: u32,
		map_depth: u32,
		sector_resolution: u32,
		actor_size: f32,
	) -> Self {
		let map_dimensions =
			MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
		let cost_fields = SectorCostFields::from_bevy_2d_meshes(&map_dimensions, &meshes);
		let mut portals = SectorPortals::new(
			map_dimensions.get_length(),
			map_dimensions.get_depth(),
			sector_resolution,
		);
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
		let path = env!("CARGO_MANIFEST_DIR").to_string()
			+ "/assets/sector_cost_fields_continuous_layout.ron";
		let _ = FlowFieldTilesBundle::from_ron(30, 30, 10, 0.5, &path);
	}
}
