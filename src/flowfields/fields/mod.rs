//! The kinds of fields used by the algorithm
//!

pub mod cost_field;
pub mod flow_field;
pub mod integration_field;

use std::collections::BTreeMap;

use bevy::prelude::*;
use crate::prelude::*;

/// Defines required access to field arrays
pub trait Field<T> {
	/// Get a reference to the field array
	fn get_field(&self) -> &[[T; FIELD_RESOLUTION]; FIELD_RESOLUTION];
	/// Retrieve a grid cell value
	fn get_grid_value(&self, column: usize, row: usize) -> T;
	/// Set a grid cell to a value
	fn set_grid_value(&mut self, value: T, column: usize, row: usize);
}

// pub struct FlowFieldRecord {
// 	field: FlowField,
// 	generation
// }


/// Each generated [FlowField] is placed into this cache so that multiple actors can read from the same dataset. Each entry is given an ID of `(sector_id, portal_id)` and actors can poll the cache to retrieve the field once it's built and inserted
#[derive(Component, Default)]
pub struct FlowFieldCache(BTreeMap<((u32, u32), (usize, usize)), FlowField>);

impl FlowFieldCache {
	/// Get a [FlowField] based on the `sector_id` and `portal_id`. Returns [None] if the cache doesn't contain a record
	pub fn get_field(&self, sector_id: (u32, u32), portal_id: (usize, usize)) -> Option<&FlowField> {
		self.0.get(&(sector_id, portal_id))
	}
	/// Insert a [FlowField] into the cache with a sector-portal ID
	pub fn insert_field(&mut self, sector_id: (u32, u32), portal_id: (usize, usize), field: FlowField) {
		self.0.insert((sector_id, portal_id), field);
	}
	/// Remove a [FlowField] from the cache (when it needs regenerating from a [CostField] update)
	pub fn remove_field(&mut self, sector_id: (u32, u32), portal_id: (usize, usize)) {
		self.0.remove(&(sector_id, portal_id));
	}
}