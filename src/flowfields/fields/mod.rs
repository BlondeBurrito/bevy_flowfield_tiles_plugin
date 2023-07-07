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

#[derive(Eq)]
pub struct RouteMetadata {
	source_sector: (u32, u32),
	source_grid_cell: (usize, usize),
	target_sector: (u32, u32),
	target_goal: (usize, usize),
	//? If a game is running for 136 years bad things will start happening here
	/// Marks the route based on time elapsed since app start, used to enable automatic cleardown of long lived routes that are probably not needed anymore
	time_generated: Duration,
}
// we don't want to compare `time_generated` so manually impl PartialEq
impl PartialEq for RouteMetadata {
	fn eq(&self, other: &Self) -> bool {
		if self.source_sector == other.source_sector
			&& self.source_grid_cell == other.source_grid_cell
			&& self.target_sector == other.target_sector
			&& self.target_goal == other.target_goal
		{
			true
		} else {
			false
		}
	}
}
/// Each entry is given an ID of `(sector_id, sector_id, goal_id)` referring to the high-level route an actor has asked for. The value is a list of `(sector_id, goal_id)` referring to the sector-portal (or just the end goal) route. An actor can use this as a fallback if the `field_cache` doesn't yet contain the granular [FlowField] routes or for when [CostField]s have been changed and so [FlowField]s in the cache need to be regenerated
#[derive(Component, Default)]
pub struct RouteCache(
	BTreeMap<((u32, u32), (u32, u32), (usize, usize)), Vec<((u32, u32), (usize, usize))>>,
);

impl RouteCache {
	pub fn get(
		&self,
	) -> &BTreeMap<((u32, u32), (u32, u32), (usize, usize)), Vec<((u32, u32), (usize, usize))>> {
		&self.0
	}
	/// Get a high-level sector to sector route. Returns [None] if it doesn't exist
	pub fn get_route(
		&self,
		source_sector: (u32, u32),
		target_sector: (u32, u32),
		goal_id: (usize, usize),
	) -> Option<&Vec<((u32, u32), (usize, usize))>> {
		self.0.get(&(source_sector, target_sector, goal_id))
	}
	/// Insert a high-level route of sector-portal paths (or just the end goal if local sector pathing) into the `route_cache`
	pub fn insert_route(
		&mut self,
		source_sector: (u32, u32),
		target_sector: (u32, u32),
		goal_id: (usize, usize),
		route: Vec<((u32, u32), (usize, usize))>,
	) {
		self.0
			.insert((source_sector, target_sector, goal_id), route);
	}
	/// Remove a high-level  route of sector-portal paths (or just the end goal if local sector pathing) from the `route_cache`
	pub fn remove_route(
		&mut self,
		source_sector: (u32, u32),
		target_sector: (u32, u32),
		goal_id: (usize, usize),
	) {
		self.0.remove(&(source_sector, target_sector, goal_id));
	}
}
#[derive(Eq)]
pub struct FlowFieldMetadata {
	sector_id: (u32, u32),
	goal_id: (usize, usize),
	//? If a game is running for 136 years bad things will start happening here
	/// Marks the field based on time elapsed since app start, used to enable automatic cleardown of long lived fields that are probably not needed anymore
	time_generated: Duration,
}
// we don't want to compare `time_generated` so manually impl PartialEq
impl PartialEq for FlowFieldMetadata {
	fn eq(&self, other: &Self) -> bool {
		if self.sector_id == other.sector_id && self.goal_id == other.goal_id {
			true
		} else {
			false
		}
	}
}
//? means of invalidating fields in cache that are very old?
/// Each generated [FlowField] is placed into this cache so that multiple actors can read from the same dataset.
///
/// Each entry is given an ID of `(sector_id, goal_id)` and actors can poll the cache to retrieve the field once it's built and inserted. Note that `goal_id` can refer to the true end-goal or it can refer to a portal position when a path spans multiple sectors
#[derive(Component, Default)]
pub struct FlowFieldCache(BTreeMap<((u32, u32), (usize, usize)), FlowField>);

impl FlowFieldCache {
	pub fn get(&self) -> &BTreeMap<((u32, u32), (usize, usize)), FlowField> {
		&self.0
	}
	/// Get a [FlowField] based on the `sector_id` and `goal_id`. Returns [None] if the cache doesn't contain a record
	pub fn get_field(&self, sector_id: (u32, u32), goal_id: (usize, usize)) -> Option<&FlowField> {
		self.0.get(&(sector_id, goal_id))
	}
	/// Insert a [FlowField] into the cache with a sector-goal ID
	pub fn insert_field(
		&mut self,
		sector_id: (u32, u32),
		goal_id: (usize, usize),
		field: FlowField,
	) {
		self.0.insert((sector_id, goal_id), field);
	}
	/// Remove a [FlowField] from the cache (when it needs regenerating from a [CostField] update)
	pub fn remove_field(&mut self, sector_id: (u32, u32), goal_id: (usize, usize)) {
		self.0.remove(&(sector_id, goal_id));
	}
}
