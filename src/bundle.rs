//! Defines the [FlowFieldTiles] component which can be spawned as/inserted
//! into an entity which movable actors can query for pathing data
//!

use std::{
	collections::VecDeque,
	sync::{Arc, RwLock},
};

use bevy::{
	prelude::*,
	tasks::{AsyncComputeTaskPool, Task},
};

use crate::flowfields::{
	dimensions::Dimensions,
	fields::{FieldCell, flow_field::FlowField},
	flowfield_cache::FlowFieldCache,
	portal::Portals,
	route::RouteStep,
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
	/// [crate::flowfields::fields::cost_field::CostField]s of all sectors
	pub sector_cost_fields: Arc<RwLock<SectorCostFields>>,
	/// Portals and graph describing sector-to-sector connectivity
	pub portals: Arc<RwLock<Portals>>,
	/// A list of updates to be applied to [SectorCostFields]
	pub costfield_update_queue: VecDeque<CostFieldUpdateItem>,
	/// Stores [bevy::tasks::AsyncComputeTaskPool] [Task] when a
	///  [crate::flowfields::fields::cost_field::CostFieldCostField] is
	/// being updated
	#[cfg_attr(feature = "serde", serde(skip))]
	pub costfield_update_task: Option<Task<SectorID>>,
	/// Stores [bevy::tasks::AsyncComputeTaskPool] [Task] when a [Portals] are
	/// being updated
	#[cfg_attr(feature = "serde", serde(skip))]
	pub portal_update_task: Option<Task<SectorID>>,
	/// Store groups of routes that will be used to create [FlowField]
	pub flow_queue: Arc<RwLock<VecDeque<Vec<RouteStep>>>>,
	/// Stores [bevy::tasks::AsyncComputeTaskPool] [Task] when a group of
	/// [FlowField]s are being built
	#[cfg_attr(feature = "serde", serde(skip))]
	pub flow_gen_task: Option<Task<Vec<(RouteStep, FlowField)>>>,
	/// Cache of [FlowField]s that can be queried in a steering pipeline
	pub flowfield_cache: FlowFieldCache,
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
	/// Get a reference to [Portals]
	pub fn get_portals(&self) -> &Arc<RwLock<Portals>> {
		&self.portals
	}
	/// Create a new instance of [FlowFieldTiles] based on world dimensions
	pub fn new(
		origin: (f32, f32),
		size: (f32, f32),
		world_unit_size: f32,
		actor_radius: f32,
	) -> Self {
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let costfields = Arc::new(RwLock::new(SectorCostFields::new(&dimensions)));
		let c = costfields.read().unwrap();
		let portals = Arc::new(RwLock::new(Portals::new(&c)));
		// unlock now that portals are built
		drop(c);

		FlowFieldTiles {
			dimensions,
			sector_cost_fields: costfields,
			portals,
			costfield_update_queue: VecDeque::new(),
			costfield_update_task: None,
			portal_update_task: None,
			flow_queue: Arc::new(RwLock::new(VecDeque::new())),
			flow_gen_task: None,
			flowfield_cache: FlowFieldCache::default(),
		}
	}
	/// Create a new instance of [FlowFieldTiles] with a starting `cost` across
	/// all [crate::flowfields::fields::cost_field::CostField]s
	pub fn new_with_cost(
		origin: (f32, f32),
		size: (f32, f32),
		world_unit_size: f32,
		actor_radius: f32,
		cost: u8,
	) -> Self {
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let costfields = Arc::new(RwLock::new(SectorCostFields::new_with_cost(
			&dimensions,
			cost,
		)));
		let c = costfields.read().unwrap();
		let portals = Arc::new(RwLock::new(Portals::new(&c)));
		// unlock now that portals are built
		drop(c);

		FlowFieldTiles {
			dimensions,
			sector_cost_fields: costfields,
			portals,
			costfield_update_queue: VecDeque::new(),
			costfield_update_task: None,
			portal_update_task: None,
			flow_queue: Arc::new(RwLock::new(VecDeque::new())),
			flow_gen_task: None,
			flowfield_cache: FlowFieldCache::default(),
		}
	}
	/// Create a new instance of [FlowFieldTiles] based on map dimensions where the [SectorCostFields] are derived from a `.ron` file
	#[cfg(feature = "ron")]
	pub fn from_ron(
		origin: (f32, f32),
		size: (f32, f32),
		world_unit_size: f32,
		actor_radius: f32,
		file_path: &str,
	) -> Self {
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let costfields = Arc::new(RwLock::new(SectorCostFields::from_ron(
			file_path.into(),
			&dimensions,
		)));
		let c = costfields.read().unwrap();
		let portals = Arc::new(RwLock::new(Portals::new(&c)));
		// unlock now that portals are built
		drop(c);

		FlowFieldTiles {
			dimensions,
			sector_cost_fields: costfields,
			portals,
			costfield_update_queue: VecDeque::new(),
			costfield_update_task: None,
			portal_update_task: None,
			flow_queue: Arc::new(RwLock::new(VecDeque::new())),
			flow_gen_task: None,
			flowfield_cache: FlowFieldCache::default(),
		}
	}
	/// From a greyscale heightmap image initialise a bundle where the
	/// [crate::flowfields::fields::cost_field::CostField]s are derived from the
	/// pixel values of the image
	#[cfg(not(tarpaulin_include))]
	#[cfg(feature = "heightmap")]
	pub fn from_heightmap(
		origin: (f32, f32),
		size: (f32, f32),
		world_unit_size: f32,
		actor_radius: f32,
		file_path: &str,
	) -> Self {
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let costfields = Arc::new(RwLock::new(SectorCostFields::from_heightmap(
			&dimensions,
			file_path.into(),
		)));
		let c = costfields.read().unwrap();
		let portals = Arc::new(RwLock::new(Portals::new(&c)));
		// unlock now that portals are built
		drop(c);

		FlowFieldTiles {
			dimensions,
			sector_cost_fields: costfields,
			portals,
			costfield_update_queue: VecDeque::new(),
			costfield_update_task: None,
			portal_update_task: None,
			flow_queue: Arc::new(RwLock::new(VecDeque::new())),
			flow_gen_task: None,
			flowfield_cache: FlowFieldCache::default(),
		}
	}
	/// Add a [CostFieldUpdateItem] to the queue based on the [Dimensions]
	/// `world_unit_size` at the supplied `position`
	#[cfg(feature = "2d")]
	pub fn add_costfield_update_2d(&mut self, position: Vec2, cost: u8) {
		if let Some((sector, cell)) = self.dimensions.get_sector_and_field_cell_from_xy(position) {
			let item = CostFieldUpdateItem::new(&sector, &cell, cost);
			self.costfield_update_queue.push_back(item);
		}
	}
	/// Add a [CostFieldUpdateItem] to the queue based on the [Dimensions]
	/// `world_unit_size` at the supplied `position`
	#[cfg(feature = "3d")]
	pub fn add_costfield_update_3d(&mut self, position: Vec3, cost: u8) {
		if let Some((sector, cell)) = self.dimensions.get_sector_and_field_cell_from_xyz(position) {
			let item = CostFieldUpdateItem::new(&sector, &cell, cost);
			self.costfield_update_queue.push_back(item);
		}
	}

	/// Request a path (if it exists). If the `from` and `to` parameters are valid
	/// coordinates in [Dimensions] space a [Task] will be returned. Polling this
	/// task will return a high-level list of [RouteStep] describing the portal-to
	/// -portal route of the path if it exists. Each [RouteStep] can be used with
	/// the `read_flowfield()` method to obtain a [FlowField] for the step
	#[cfg(feature = "2d")]
	pub fn get_route_2d(&self, from: Vec2, to: Vec2) -> Option<Task<Option<Vec<RouteStep>>>> {
		let (source_sector, source_cell) =
			self.dimensions.get_sector_and_field_cell_from_xy(from)?;
		let (goal_sector, goal_cell) = self.dimensions.get_sector_and_field_cell_from_xy(to)?;
		//
		self.get_route(source_sector, source_cell, goal_sector, goal_cell)
	}
	/// Request a path (if it exists). If the `from` and `to` parameters are valid
	/// coordinates in [Dimensions] space a [Task] will be returned. Polling this
	/// task will return a high-level list of [RouteStep] describing the portal-to
	/// -portal route of the path if it exists. Each [RouteStep] can be used with
	/// the `read_flowfield()` method to obtain a [FlowField] for the step
	#[cfg(feature = "3d")]
	pub fn get_route_3d(&self, from: Vec3, to: Vec3) -> Option<Task<Option<Vec<RouteStep>>>> {
		let (source_sector, source_cell) =
			self.dimensions.get_sector_and_field_cell_from_xyz(from)?;
		let (goal_sector, goal_cell) = self.dimensions.get_sector_and_field_cell_from_xyz(to)?;
		self.get_route(source_sector, source_cell, goal_sector, goal_cell)
	}
	/// From a source and goal attempt to retrieve a series of [RouteStep]
	/// describing the path
	fn get_route(
		&self,
		source_sector: SectorID,
		source_cell: FieldCell,
		goal_sector: SectorID,
		goal_cell: FieldCell,
	) -> Option<Task<Option<Vec<RouteStep>>>> {
		let costfields = self.sector_cost_fields.clone();
		let portals = self.portals.clone();
		let queue = self.flow_queue.clone();
		//
		let thread_pool = AsyncComputeTaskPool::get();
		let task = thread_pool.spawn(async move {
			let read_costfields = costfields.read().unwrap();
			let read_portals = portals.read().unwrap();
			let path = read_portals.find_path(
				&source_sector,
				&source_cell,
				&goal_sector,
				&goal_cell,
				&read_costfields,
			);

			if let Some(p) = path {
				// push route into a queue for flowfield generation
				let mut write_queue = queue.write().unwrap();
				write_queue.push_back(p.clone());
				Some(p)
			} else {
				None
			}
		});
		Some(task)
	}

	/// Retrieve a [FlowField] based on a [RouteStep]
	///
	/// This will return `None` if the [FlowField] has not been generated yet
	pub fn read_flowfield(&self, route_step: &RouteStep) -> Option<&FlowField> {
		self.flowfield_cache.get(route_step)
	}
	/// Get a reference to the [FlowFieldCache]
	pub fn flowfield_cache(&self) -> &FlowFieldCache {
		&self.flowfield_cache
	}
}
