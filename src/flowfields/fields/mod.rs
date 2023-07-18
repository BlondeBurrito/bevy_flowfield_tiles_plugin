//! The kinds of fields used by the algorithm
//!

pub mod cost_field;
pub mod flow_field;
pub mod integration_field;

use std::collections::BTreeMap;

use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::Duration;

/// Defines required access to field arrays
pub trait Field<T> {
	/// Get a reference to the field array
	fn get_field(&self) -> &[[T; FIELD_RESOLUTION]; FIELD_RESOLUTION];
	/// Retrieve a grid cell value
	fn get_grid_value(&self, column: usize, row: usize) -> T;
	/// Set a grid cell to a value
	fn set_grid_value(&mut self, value: T, column: usize, row: usize);
}
/// Describes the properties of a route
#[derive(Clone, Copy, Debug)]
pub struct RouteMetadata {
	/// Starting sector of the route
	source_sector: (u32, u32),
	/// Sector to find a route to
	target_sector: (u32, u32),
	/// Field/grid cell of the goal in the target sector
	target_goal: (usize, usize),
	//? If a game is running for 136 years bad things will start happening here
	/// Marks the route based on time elapsed since app start, used to enable automatic cleardown of long lived routes that are probably not needed anymore
	time_generated: Duration,
}
// we don't want to compare `time_generated` so manually impl PartialEq
impl PartialEq for RouteMetadata {
	fn eq(&self, other: &Self) -> bool {
		self.source_sector == other.source_sector
			&& self.target_sector == other.target_sector
			&& self.target_goal == other.target_goal
	}
}
impl Eq for RouteMetadata {}

impl Ord for RouteMetadata {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(self.source_sector, self.target_sector, self.target_goal).cmp(&(
			other.source_sector,
			other.target_sector,
			other.target_goal,
		))
	}
}

impl PartialOrd for RouteMetadata {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl RouteMetadata {
	/// Get the source sector
	pub fn get_source_sector(&self) -> (u32, u32) {
		self.source_sector
	}
	/// Get the target sector
	pub fn get_target_sector(&self) -> (u32, u32) {
		self.target_sector
	}
	/// Get the goal
	pub fn get_target_goal(&self) -> (usize, usize) {
		self.target_goal
	}
	/// Get when the route was generated
	pub fn get_time_generated(&self) -> Duration {
		self.time_generated
	}
}
/// Each key makes use of custom Ord and Eq implementations based on comparing `(sector_id, sector_id, goal_id)` so that RouteMetaData can be used to refer to the high-level route an actor has asked for. The value is a list of `(sector_id, goal_id)` referring to the sector-portal (or just the end goal) route. An actor can use this as a fallback if the `field_cache` doesn't yet contain the granular [FlowField] routes or for when [CostField]s have been changed and so [FlowField]s in the cache need to be regenerated
#[allow(clippy::type_complexity)]
#[derive(Component, Default)]
pub struct RouteCache(BTreeMap<RouteMetadata, Vec<((u32, u32), (usize, usize))>>);

impl RouteCache {
	/// Get the map of routes
	#[allow(clippy::type_complexity)]
	pub fn get(&self) -> &BTreeMap<RouteMetadata, Vec<((u32, u32), (usize, usize))>> {
		&self.0
	}
	/// Get a mutable reference to the map of routes
	#[allow(clippy::type_complexity)]
	pub fn get_mut(&mut self) -> &mut BTreeMap<RouteMetadata, Vec<((u32, u32), (usize, usize))>> {
		&mut self.0
	}
	/// Get a high-level sector to sector route. Returns [None] if it doesn't exist
	#[allow(clippy::type_complexity)]
	pub fn get_route(
		&self,
		source_sector: (u32, u32),
		target_sector: (u32, u32),
		goal_id: (usize, usize),
	) -> Option<&Vec<((u32, u32), (usize, usize))>> {
		let route_data = RouteMetadata {
			source_sector,
			target_sector,
			target_goal: goal_id,
			time_generated: Duration::default(),
		};
		let route = self.0.get(&route_data);
		trace!("Route: {:?}", route);
		route
	}
	/// Insert a high-level route of sector-portal paths (or just the end goal if local sector pathing) into the `route_cache`
	pub fn insert_route(
		&mut self,
		source_sector: (u32, u32),
		target_sector: (u32, u32),
		goal_id: (usize, usize),
		elapsed_duration: Duration,
		route: Vec<((u32, u32), (usize, usize))>,
	) {
		let route_data = RouteMetadata {
			source_sector,
			target_sector,
			target_goal: goal_id,
			time_generated: elapsed_duration,
		};
		self.0.insert(route_data, route);
	}
	/// Remove a high-level  route of sector-portal paths (or just the end goal if local sector pathing) from the `route_cache`
	pub fn remove_route(&mut self, route_metadata: RouteMetadata) {
		self.0.remove(&route_metadata);
	}
}
/// Describes the properties of a [FlowField]
#[derive(Clone, Copy)]
pub struct FlowFieldMetadata {
	/// The sector of the corresponding [FlowField]
	sector_id: (u32, u32),
	/// Portal goal or true target goal of the sector
	goal_id: (usize, usize),
	//? If a game is running for 136 years bad things will start happening here
	/// Marks the field based on time elapsed since app start, used to enable automatic cleardown of long lived fields that are probably not needed anymore
	time_generated: Duration,
}
// we don't want to compare `time_generated` so manually impl PartialEq
impl PartialEq for FlowFieldMetadata {
	fn eq(&self, other: &Self) -> bool {
		self.sector_id == other.sector_id && self.goal_id == other.goal_id
	}
}

impl Eq for FlowFieldMetadata {}

impl FlowFieldMetadata {
	/// Get the sector
	pub fn get_sector_id(&self) -> (u32, u32) {
		self.sector_id
	}
	/// Get the goal
	pub fn get_goal_id(&self) -> (usize, usize) {
		self.goal_id
	}
	/// Get when the field was generated
	pub fn get_time_generated(&self) -> Duration {
		self.time_generated
	}
}

impl Ord for FlowFieldMetadata {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(self.sector_id, self.goal_id).cmp(&(other.sector_id, other.goal_id))
	}
}

impl PartialOrd for FlowFieldMetadata {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

//TODO? means of invalidating fields in cache that are very old?
/// Each generated [FlowField] is placed into this cache so that multiple actors can read from the same dataset.
///
/// Each entry is given an ID of `(sector_id, goal_id)` and actors can poll the cache to retrieve the field once it's built and inserted. Note that `goal_id` can refer to the true end-goal or it can refer to a portal position when a path spans multiple sectors
#[derive(Component, Default)]
pub struct FlowFieldCache(BTreeMap<FlowFieldMetadata, FlowField>);

impl FlowFieldCache {
	/// Get the map of [FlowField]s
	pub fn get(&self) -> &BTreeMap<FlowFieldMetadata, FlowField> {
		&self.0
	}
	/// Get a mutable reference to the map of [FlowField]s
	pub fn get_mut(&mut self) -> &mut BTreeMap<FlowFieldMetadata, FlowField> {
		&mut self.0
	}
	/// Get a [FlowField] based on the `sector_id` and `goal_id`. Returns [None] if the cache doesn't contain a record
	pub fn get_field(&self, sector_id: (u32, u32), goal_id: (usize, usize)) -> Option<&FlowField> {
		let flow_meta = FlowFieldMetadata {
			sector_id,
			goal_id,
			time_generated: Duration::default(),
		};
		self.0.get(&flow_meta)
	}
	/// Insert a [FlowField] into the cache with a sector-goal ID
	pub fn insert_field(
		&mut self,
		sector_id: (u32, u32),
		goal_id: (usize, usize),
		elapsed_duration: Duration,
		field: FlowField,
	) {
		let flow_meta = FlowFieldMetadata {
			sector_id,
			goal_id,
			time_generated: elapsed_duration,
		};
		self.0.insert(flow_meta, field);
	}
	/// Remove a [FlowField] from the cache (when it needs regenerating from a [CostField] update)
	pub fn remove_field(&mut self, flow_meta: FlowFieldMetadata) {
		self.0.remove(&flow_meta);
	}
}
