//! Defines a bundle which can be spawned as/inserted into an entity which
//! movable actors can query for pathing data
//!

use std::{
	collections::VecDeque,
	sync::{Arc, RwLock},
};

use bevy::{prelude::*, tasks::Task};

use crate::v2::flowfields::{
	dimensions::Dimensions,
	portal::Portals,
	sectors::{
		SectorID,
		sector_cost::{CostFieldUpdateItem, SectorCostFields},
	},
};

/// Defines all required data for generating [FlowField] Tiles
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component)]
pub struct FlowFieldTiles {
	/// Size of the world
	pub dimensions: Dimensions,
	/// [CostField]s of all sectors
	pub sector_cost_fields: Arc<RwLock<SectorCostFields>>,
	/// Portals and graph describing sector-to-sector connectivity
	pub portals: Arc<RwLock<Portals>>,
	/// A list of updates to be applied to [SectorCostFields]
	pub costfield_update_queue: VecDeque<CostFieldUpdateItem>,
	/// Stores [bevy::tasks::AsyncComputeTaskPool] [Task] when a [CostField] is
	/// being updated
	#[cfg_attr(feature = "serde", serde(skip))]
	pub costfield_update_task: Option<Task<SectorID>>,
	/// Stores [bevy::tasks::AsyncComputeTaskPool] [Task] when a [Portals] are
	/// being updated
	#[cfg_attr(feature = "serde", serde(skip))]
	pub portal_update_task: Option<Task<SectorID>>,
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
	pub fn get_sector_cost_fields(&self) -> &Arc<RwLock<SectorCostFields>> {
		&self.sector_cost_fields
	}
	/// Get a mutable reference to the [SectorCostFields]
	pub fn get_sector_cost_fields_mut(&mut self) -> &mut Arc<RwLock<SectorCostFields>> {
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
		let costfields = Arc::new(RwLock::new(SectorCostFields::new(&dimensions)));
		let c = costfields.read().unwrap();
		let portals = Arc::new(RwLock::new(Portals::new(&*c)));
		// unlock now that portals are built
		drop(c);
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
			sector_cost_fields: costfields,
			portals,
			costfield_update_queue: VecDeque::new(),
			costfield_update_task: None,
			portal_update_task: None,
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
		let costfields = Arc::new(RwLock::new(SectorCostFields::from_ron(
			file_path.into(),
			&dimensions,
		)));
		let c = costfields.read().unwrap();
		let portals = Arc::new(RwLock::new(Portals::new(&*c)));
		// unlock now that portals are built
		drop(c);
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
			sector_cost_fields: costfields,
			portals,
			costfield_update_queue: VecDeque::new(),
			costfield_update_task: None,
			portal_update_task: None,
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
		let costfields = Arc::new(RwLock::new(SectorCostFields::from_heightmap(
			&dimensions,
			file_path.into(),
		)));
		let c = costfields.read().unwrap();
		let portals = Arc::new(RwLock::new(Portals::new(&*c)));
		// unlock now that portals are built
		drop(c);
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
			sector_cost_fields: costfields,
			portals,
			costfield_update_queue: VecDeque::new(),
			costfield_update_task: None,
			portal_update_task: None,
			// sector_portals: portals,
			// portal_graph: graph,
			// route_cache,
			// flow_field_cache: cache,
		}
	}
	/// Add a [CostFieldUpdateItem] to the queue
	pub fn add_costfield_update(&mut self, position: Vec2, cost: u8) {
		if let Some((sector, cell)) = self.dimensions.get_sector_and_field_cell_from_xy(position) {
			let item = CostFieldUpdateItem::new(&sector, &cell, cost);
			self.costfield_update_queue.push_back(item);
		}
	}
}
