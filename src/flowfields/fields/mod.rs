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
	fn get(&self) -> &[[T; FIELD_RESOLUTION]; FIELD_RESOLUTION];
	/// Retrieve a field cell value
	fn get_field_cell_value(&self, field_cell: FieldCell) -> T;
	/// Set a field cell to a value
	fn set_field_cell_value(&mut self, value: T, field_cell: FieldCell);
}

/// ID of a cell within a field
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Reflect)]
pub struct FieldCell((usize, usize));

impl FieldCell {
	/// Create a new instance of [FieldCell]
	pub fn new(column: usize, row: usize) -> Self {
		FieldCell((column, row))
	}
	/// Get the sector `(column, row)` tuple
	pub fn get_column_row(&self) -> (usize, usize) {
		self.0
	}
	/// Get the sector column
	pub fn get_column(&self) -> usize {
		self.0 .0
	}
	/// Get the sector row
	pub fn get_row(&self) -> usize {
		self.0 .1
	}
	/// From the position of a `cell_id`, if it sits along a boundary, return the [Ordinal] of that boundary. Note that if the `cell_id` is in a field corner then it'll have two boundaries. Note that if the `cell_id` is not in fact along a boundary then this will panic
	pub fn get_boundary_ordinal_from_field_cell(&self) -> Vec<Ordinal> {
		let mut boundaries = Vec::new();
		if self.get_row() == 0 {
			boundaries.push(Ordinal::North);
		}
		if self.get_column() == FIELD_RESOLUTION - 1 {
			boundaries.push(Ordinal::East);
		}
		if self.get_row() == FIELD_RESOLUTION - 1 {
			boundaries.push(Ordinal::South);
		}
		if self.get_column() == 0 {
			boundaries.push(Ordinal::West);
		}
		if !boundaries.is_empty() {
			boundaries
		} else {
			panic!("{:?} does not sit along the boundary", self);
		}
	}
}

/// Describes the properties of a route
#[derive(Clone, Copy, Debug, Reflect)]
pub struct RouteMetadata {
	/// Starting sector of the route
	source_sector: SectorID,
	/// Sector to find a route to
	target_sector: SectorID,
	/// Field cell of the goal in the target sector
	target_goal: FieldCell,
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
	pub fn get_source_sector(&self) -> SectorID {
		self.source_sector
	}
	/// Get the target sector
	pub fn get_target_sector(&self) -> SectorID {
		self.target_sector
	}
	/// Get the goal
	pub fn get_target_goal(&self) -> FieldCell {
		self.target_goal
	}
	/// Get when the route was generated
	pub fn get_time_generated(&self) -> Duration {
		self.time_generated
	}
}
/// Each key makes use of custom Ord and Eq implementations based on comparing `(source_id, target_id, goal_id)` so that RouteMetaData can be used to refer to the high-level route an actor has asked for. The value is a list of `(sector_id, goal_id)` referring to the sector-portal (or just the end goal) route. An actor can use this as a fallback if the `field_cache` doesn't yet contain the granular [FlowField] routes or for when [CostField]s have been changed and so [FlowField]s in the cache need to be regenerated
#[derive(Component, Default, Clone)]
pub struct RouteCache(BTreeMap<RouteMetadata, Vec<(SectorID, FieldCell)>>);

impl RouteCache {
	/// Get the map of routes
	pub fn get(&self) -> &BTreeMap<RouteMetadata, Vec<(SectorID, FieldCell)>> {
		&self.0
	}
	/// Get a mutable reference to the map of routes
	pub fn get_mut(&mut self) -> &mut BTreeMap<RouteMetadata, Vec<(SectorID, FieldCell)>> {
		&mut self.0
	}
	/// Get a high-level sector to sector route. Returns [None] if it doesn't exist
	pub fn get_route(
		&self,
		source_sector: SectorID,
		target_sector: SectorID,
		goal_id: FieldCell,
	) -> Option<&Vec<(SectorID, FieldCell)>> {
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
		source_sector: SectorID,
		target_sector: SectorID,
		goal_id: FieldCell,
		elapsed_duration: Duration,
		route: Vec<(SectorID, FieldCell)>,
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
#[derive(Clone, Copy, Reflect)]
pub struct FlowFieldMetadata {
	/// The sector of the corresponding [FlowField]
	sector_id: SectorID,
	/// Portal goal or true target goal of the sector
	goal_id: FieldCell,
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
	pub fn get_sector_id(&self) -> SectorID {
		self.sector_id
	}
	/// Get the goal
	pub fn get_goal_id(&self) -> FieldCell {
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
	pub fn get_field(&self, sector_id: SectorID, goal_id: FieldCell) -> Option<&FlowField> {
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
		sector_id: SectorID,
		goal_id: FieldCell,
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
