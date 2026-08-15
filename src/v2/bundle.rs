//! Defines a bundle which can be spawned as/inserted into an entity which
//! movable actors can query for pathing data
//!

use bevy::prelude::*;

use crate::v2::flowfields::{
	dimensions::Dimensions, portal::Portals, sectors::sector_cost::SectorCostFields,
};

/// Defines all required data for generating [FlowField] Tiles
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component)]
pub struct FlowFieldTiles {
	/// Size of the world
	pub dimensions: Dimensions,
	/// [CostField]s of all sectors
	pub sector_cost_fields: SectorCostFields,
	/// Portals and graph describing sector-to-sector connectivity
	pub portals: Portals,
	// /// Portals for all sectors
	// pub sector_portals: SectorPortals,
	// /// Graph describing how to get from one sector to another
	// pub portal_graph: PortalGraph,
	// /// Cache of overarching portal-portal routes
	// pub route_cache: RouteCache,
	// /// Cache of [FlowField]s that can be queried in a steering pipeline
	// pub flow_field_cache: FlowFieldCache,
}

impl FlowFieldTiles {
	/// Get a reference to the [Dimensions]
	pub fn get_dimensions(&self) -> &Dimensions {
		&self.dimensions
	}
	/// Get a reference to the [SectorCostFields]
	pub fn get_sector_cost_fields(&self) -> &SectorCostFields {
		&self.sector_cost_fields
	}
	/// Get a mutable reference to the [SectorCostFields]
	pub fn get_sector_cost_fields_mut(&mut self) -> &mut SectorCostFields {
		&mut self.sector_cost_fields
	}
	// /// Get a reference to the [SectorPortals]
	// pub fn get_sector_portals(&self) -> &SectorPortals {
	// 	&self.sector_portals
	// }
	// /// Get a reference to the [PortalGraph]
	// pub fn get_portal_graph(&self) -> &PortalGraph {
	// 	&self.portal_graph
	// }
	// /// Get a reference to the [RouteCache]
	// pub fn get_route_cache(&self) -> &RouteCache {
	// 	&self.route_cache
	// }
	// /// Get a mutable reference to the [RouteCache]
	// pub fn get_route_cache_mut(&mut self) -> &mut RouteCache {
	// 	&mut self.route_cache
	// }
	// /// Get a reference to the [FlowFieldCache]
	// pub fn get_flowfield_cache(&self) -> &FlowFieldCache {
	// 	&self.flow_field_cache
	// }
	// /// Get a mutable reference to the [FlowFieldCache]
	// pub fn get_flowfield_cache_mut(&mut self) -> &mut FlowFieldCache {
	// 	&mut self.flow_field_cache
	// }
	/// Create a new instance of [FlowFieldTiles] based on world dimensions
	pub fn new(
		origin: (f32, f32),
		size: (f32, f32),
		world_unit_size: f32,
		actor_size: f32,
	) -> Self {
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let cost_fields = SectorCostFields::new(&dimensions);
		let portals = Portals::new(&cost_fields);
		// let mut portals = SectorPortals::new(map_length, map_depth, sector_resolution);
		// // update default portals for cost fields
		// for sector_id in cost_fields.get_scaled().keys() {
		// 	portals.update_portals(*sector_id, &cost_fields, &map_dimensions);
		// }
		// let graph = PortalGraph::new(&portals, &cost_fields, &map_dimensions);
		// let route_cache = RouteCache::default();
		// let cache = FlowFieldCache::default();
		FlowFieldTiles {
			dimensions,
			sector_cost_fields: cost_fields,
			portals,
			// sector_portals: portals,
			// portal_graph: graph,
			// route_cache,
			// flow_field_cache: cache,
		}
	}
	/// Create a new instance of [FlowFieldTilesBundle] based on map dimensions where the [SectorCostFields] are derived from a `.ron` file
	#[cfg(feature = "ron")]
	pub fn from_ron(
		origin: (f32, f32),
		size: (f32, f32),
		world_unit_size: f32,
		actor_size: f32,
		file_path: &str,
	) -> Self {
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let cost_fields = SectorCostFields::from_ron(file_path.into(), &dimensions);
		let portals = Portals::new(&cost_fields);

		// let map_dimensions =
		// 	MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
		// let cost_fields = SectorCostFields::from_ron(path.to_string(), &map_dimensions);
		// if ((map_length * map_depth) / (sector_resolution * sector_resolution)) as usize
		// 	!= cost_fields.get_baseline().len()
		// {
		// 	panic!("Map size ({}, {}) with resolution {} produces ({}x{}) sectors. Ron file only produces {} sectors", map_length, map_depth, sector_resolution, map_length/sector_resolution, map_depth/sector_resolution, cost_fields.get_baseline().len());
		// }
		// let mut portals = SectorPortals::new(map_length, map_depth, sector_resolution);
		// // update default portals for cost fields
		// for sector_id in cost_fields.get_scaled().keys() {
		// 	portals.update_portals(*sector_id, &cost_fields, &map_dimensions);
		// }
		// let graph = PortalGraph::new(&portals, &cost_fields, &map_dimensions);
		// let route_cache = RouteCache::default();
		// let cache = FlowFieldCache::default();
		FlowFieldTiles {
			dimensions,
			sector_cost_fields: cost_fields,
			portals,
			// sector_portals: portals,
			// portal_graph: graph,
			// route_cache,
			// flow_field_cache: cache,
		}
	}
	/// From a greyscale heightmap image initialise a bundle where the
	/// [CostField]s are derived from the pixel values of the image
	#[cfg(not(tarpaulin_include))]
	#[cfg(feature = "heightmap")]
	pub fn from_heightmap(
		origin: (f32, f32),
		size: (f32, f32),
		world_unit_size: f32,
		actor_size: f32,
		file_path: &str,
	) -> Self {
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let cost_fields = SectorCostFields::from_heightmap(&dimensions, file_path.into());
		let portals = Portals::new(&cost_fields);

		// let map_dimensions =
		// 	MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
		// let cost_fields = SectorCostFields::from_heightmap(&map_dimensions, file_path.to_string());
		// let mut portals = SectorPortals::new(map_length, map_depth, sector_resolution);
		// // update default portals for cost fields
		// for sector_id in cost_fields.get_scaled().keys() {
		// 	portals.update_portals(*sector_id, &cost_fields, &map_dimensions);
		// }
		// let graph = PortalGraph::new(&portals, &cost_fields, &map_dimensions);
		// let route_cache = RouteCache::default();
		// let cache = FlowFieldCache::default();
		FlowFieldTiles {
			dimensions,
			sector_cost_fields: cost_fields,
			portals,
			// sector_portals: portals,
			// portal_graph: graph,
			// route_cache,
			// flow_field_cache: cache,
		}
	}
}
