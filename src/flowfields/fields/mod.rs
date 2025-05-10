//! The kinds of fields used by the algorithm
//!

pub mod cost_field;
pub mod flow_field;
pub mod integration_field;

use std::{collections::BTreeMap, time::Duration};

use crate::prelude::*;
use bevy::prelude::*;

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

impl std::fmt::Display for FieldCell {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Column: {}, Row: {}", self.0 .0, self.0 .1)
	}
}

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
	/// Using the Bresenham line algorithm get a list of [FieldCell] that lie along a line between two points. Note that the list will contain the source (`self`) and `target` [FieldCell]
	pub fn get_cells_between_points(&self, target: &FieldCell) -> Vec<FieldCell> {
		let source_col = self.get_column() as i32;
		let source_row = self.get_row() as i32;
		let target_col = target.get_column() as i32;
		let target_row = target.get_row() as i32;

		// optimise for orthognal line (horizontal or vertical)
		if source_col == target_col {
			let mut fields = Vec::new();
			if source_row < target_row {
				for row in source_row..=target_row {
					fields.push(FieldCell::new(source_col as usize, row as usize));
				}
				fields
			} else {
				for row in target_row..=source_row {
					fields.push(FieldCell::new(source_col as usize, row as usize));
				}
				fields.reverse(); //TODO would vecdeq be good for adding at index 0, no need to reverse
				fields
			}
		} else if source_row == target_row {
			let mut fields = Vec::new();
			if source_col < target_col {
				for col in source_col..=target_col {
					fields.push(FieldCell::new(col as usize, source_row as usize));
				}
				fields
			} else {
				for col in target_col..=source_col {
					fields.push(FieldCell::new(col as usize, source_row as usize));
				}
				fields.reverse();
				fields
			}
		} else if (target_row - source_row).abs() < (target_col - source_col).abs() {
			if source_col > target_col {
				let mut fields =
					walk_bresenham_shallow(target_col, target_row, source_col, source_row);
				// ensure list points in the direction of source to target
				fields.reverse();
				fields
			} else {
				walk_bresenham_shallow(source_col, source_row, target_col, target_row)
			}
		} else if source_row > target_row {
			let mut fields = walk_bresenham_steep(target_col, target_row, source_col, source_row);
			fields.reverse();
			fields
		} else {
			walk_bresenham_steep(source_col, source_row, target_col, target_row)
		}
	}
}
/// When finding a shallow raster representation of a line we step through the x-dimension and increment y based on an error bound which indicates which cells lie on the line
fn walk_bresenham_shallow(col_0: i32, row_0: i32, col_1: i32, row_1: i32) -> Vec<FieldCell> {
	let mut cells = Vec::new();

	let delta_col = col_1 - col_0;
	let mut delta_row = row_1 - row_0;

	let mut row_increment = 1;
	if delta_row < 0 {
		row_increment = -1;
		delta_row *= -1;
	}
	let mut difference = 2 * delta_row - delta_col;
	let mut row = row_0;

	for col in col_0..=col_1 {
		cells.push(FieldCell::new(col as usize, row as usize));
		if difference > 0 {
			row += row_increment;
			difference += 2 * (delta_row - delta_col);
		} else {
			difference += 2 * delta_row;
		}
	}
	cells
}
/// When finding a steep raster representation of a line we step through the y-dimension and increment x based on an error bound which indicates which cells lie on the line
fn walk_bresenham_steep(col_0: i32, row_0: i32, col_1: i32, row_1: i32) -> Vec<FieldCell> {
	let mut cells = Vec::new();

	let mut delta_col = col_1 - col_0;
	let delta_row = row_1 - row_0;

	let mut col_increment = 1;
	if delta_col < 0 {
		col_increment = -1;
		delta_col *= -1;
	}
	let mut difference = 2 * delta_col - delta_row;
	let mut col = col_0;

	for row in row_0..=row_1 {
		cells.push(FieldCell::new(col as usize, row as usize));
		if difference > 0 {
			col += col_increment;
			difference += 2 * (delta_col - delta_row);
		} else {
			difference += 2 * delta_col;
		}
	}
	cells
}

/// Describes the properties of a route
#[derive(Clone, Copy, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RouteMetadata {
	/// Starting sector of the route
	source_sector: SectorID,
	/// Starting FieldCell of the route
	source_field: FieldCell,
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
			&& self.source_field == other.source_field
			&& self.target_sector == other.target_sector
			&& self.target_goal == other.target_goal
	}
}
impl Eq for RouteMetadata {}

impl Ord for RouteMetadata {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(
			self.source_sector,
			self.source_field,
			self.target_sector,
			self.target_goal,
		)
			.cmp(&(
				other.source_sector,
				other.source_field,
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
	/// Create a new [RouteMetadata]
	pub fn new(
		source_sector: SectorID,
		source_field: FieldCell,
		target_sector: SectorID,
		target_goal: FieldCell,
		time_generated: Duration,
	) -> Self {
		RouteMetadata {
			source_sector,
			source_field,
			target_sector,
			target_goal,
			time_generated,
		}
	}
	/// Get the source sector
	pub fn get_source_sector(&self) -> SectorID {
		self.source_sector
	}
	/// Get the source FieldCell
	pub fn get_source_field_cell(&self) -> FieldCell {
		self.source_field
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

/// List of sector-portal (or just the end goal) route describing the sector path an actor should take to move to a destination sector
#[derive(Default, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Route(Vec<(SectorID, FieldCell)>);

impl Route {
	/// Get the sector to sector path including portals/goals
	pub fn get(&self) -> &Vec<(SectorID, FieldCell)> {
		&self.0
	}
	/// Get a mutable reference to the sector to sector path including portals/goals
	pub fn get_mut(&mut self) -> &mut Vec<(SectorID, FieldCell)> {
		&mut self.0
	}
	/// Create a new instance of [Route] with the given `path`
	pub fn new(path: Vec<(SectorID, FieldCell)>) -> Self {
		Route(path)
	}
}

/// Each key makes use of custom Ord and Eq implementations based on comparing `(source_id, target_id, goal_id)` so that RouteMetaData can be used to refer to the high-level route an actor has asked for. The value is a sector-portal (or just the end goal) route. An actor can use this as a fallback if the `field_cache` doesn't yet contain the granular [FlowField] routes or for when [CostField]s have been changed and so [FlowField]s in the cache need to be regenerated
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RouteCache {
	/// A queue of high-level routes which get processed into the `routes` field
	route_queue: BTreeMap<RouteMetadata, Route>,
	/// High-level routes describing the path from an actor to an end goal
	routes: BTreeMap<RouteMetadata, Route>,
}

impl RouteCache {
	/// Get a refernce to the map of queued routes
	pub fn get_queue(&self) -> &BTreeMap<RouteMetadata, Route> {
		&self.route_queue
	}
	/// Get a mutable reference to the map of queued routes
	pub fn get_queue_mut(&mut self) -> &mut BTreeMap<RouteMetadata, Route> {
		&mut self.route_queue
	}
	/// Get the map of routes
	pub fn get_routes(&self) -> &BTreeMap<RouteMetadata, Route> {
		&self.routes
	}
	/// Get a mutable reference to the map of routes
	pub fn get_mut(&mut self) -> &mut BTreeMap<RouteMetadata, Route> {
		&mut self.routes
	}
	/// Get a high-level sector to sector route. Returns [None] if it doesn't exist
	pub fn get_route(
		&self,
		source_sector: SectorID,
		source_field: FieldCell,
		target_sector: SectorID,
		goal_id: FieldCell,
	) -> Option<&Route> {
		let route_data = RouteMetadata {
			source_sector,
			source_field,
			target_sector,
			target_goal: goal_id,
			time_generated: Duration::default(),
		};
		let route = self.routes.get(&route_data);
		route
	}
	/// Get a high-level sector to sector route. Returns [None] if it doesn't exist
	pub fn get_route_with_metadata(
		&self,
		source_sector: SectorID,
		source_field: FieldCell,
		target_sector: SectorID,
		goal_id: FieldCell,
	) -> Option<(&RouteMetadata, &Route)> {
		let route_data = RouteMetadata {
			source_sector,
			source_field,
			target_sector,
			target_goal: goal_id,
			time_generated: Duration::default(),
		};
		let route = self.routes.get_key_value(&route_data);
		route
	}
	/// Insert a high-level route of sector-portal paths (or just the end goal if local sector pathing) into the `route_cache`
	pub fn add_to_queue(&mut self, route_data: RouteMetadata, route: Route) {
		self.route_queue.insert(route_data, route);
	}
	/// Insert a high-level route of sector-portal paths (or just the end goal if local sector pathing) into the `route_cache`
	pub fn insert_route(
		&mut self,
		source_sector: SectorID,
		source_field: FieldCell,
		target_sector: SectorID,
		goal_id: FieldCell,
		elapsed_duration: Duration,
		route: Route,
	) {
		let route_data = RouteMetadata {
			source_sector,
			source_field,
			target_sector,
			target_goal: goal_id,
			time_generated: elapsed_duration,
		};
		self.routes.insert(route_data, route);
	}
	/// Insert a high-level route of sector-portal paths (or just the end goal if local sector pathing) into the `route_cache` with an already created [RouteMetadata] structure
	pub fn insert_route_with_metadata(&mut self, route_metadata: RouteMetadata, route: Route) {
		self.routes.insert(route_metadata, route);
	}
	/// Remove a high-level  route of sector-portal paths (or just the end goal if local sector pathing) from the `route_cache`
	pub fn remove_route(&mut self, route_metadata: RouteMetadata) {
		self.routes.remove(&route_metadata);
	}
	/// Remove a high-level route that has been queued (or just the end goal if
	/// local sector pathing)
	pub fn remove_queued_route(&mut self, route_metadata: RouteMetadata) {
		self.route_queue.remove(&route_metadata);
	}
}
/// Describes the properties of a [FlowField]
#[derive(Clone, Copy, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FlowFieldMetadata {
	/// The sector of the corresponding [FlowField]
	sector_id: SectorID,
	/// Goal ID if this is the field of the terminus sector
	goal_id: Option<FieldCell>,
	/// Portal ID if this field is used in trnasit to another sector
	portal_id: Option<FieldCell>,
	//? If a game is running for 136 years bad things will start happening here
	/// Marks the field based on time elapsed since app start, used to enable automatic cleardown of long lived fields that are probably not needed anymore
	time_generated: Duration,
}
// we don't want to compare `time_generated` so manually impl PartialEq
impl PartialEq for FlowFieldMetadata {
	fn eq(&self, other: &Self) -> bool {
		self.sector_id == other.sector_id
			&& self.goal_id == other.goal_id
			&& self.portal_id == other.portal_id
	}
}
impl Eq for FlowFieldMetadata {}
impl Ord for FlowFieldMetadata {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(self.sector_id, self.goal_id, self.portal_id).cmp(&(
			other.sector_id,
			other.goal_id,
			other.portal_id,
		))
	}
}
impl PartialOrd for FlowFieldMetadata {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl FlowFieldMetadata {
	/// Get the sector
	pub fn get_sector_id(&self) -> SectorID {
		self.sector_id
	}
	/// Get the goal
	pub fn get_goal_id(&self) -> Option<FieldCell> {
		self.goal_id
	}
	/// Get the portal
	pub fn get_portal_id(&self) -> Option<FieldCell> {
		self.portal_id
	}
	/// Get when the field was generated
	pub fn get_time_generated(&self) -> Duration {
		self.time_generated
	}
}

/// Each generated [FlowField] is placed into this cache so that multiple actors can read from the same dataset.
///
/// Each entry is given an ID of `(sector_id, goal_id)` and actors can poll the
/// cache to retrieve the field once it's built and inserted. Note that
/// `goal_id` can refer to the true end-goal or it can refer to a portal
/// position when a path spans multiple sectors
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FlowFieldCache {
	/// Routes describing the sector path and [IntegrationField]s where the
	/// integration and flow fields can be incrementally built
	queue: BTreeMap<RouteMetadata, IntegrationBuilder>,
	/// Created FlowFields that actors can use to pathfind
	flows: BTreeMap<FlowFieldMetadata, FlowField>,
}

impl FlowFieldCache {
	/// Get the map of [FlowField]s
	pub fn get(&self) -> &BTreeMap<FlowFieldMetadata, FlowField> {
		&self.flows
	}
	/// Get a mutable reference to the map of [FlowField]s
	pub fn get_mut(&mut self) -> &mut BTreeMap<FlowFieldMetadata, FlowField> {
		&mut self.flows
	}
	/// Get a mutable reference to the queue map
	pub fn get_queue_mut(&mut self) -> &mut BTreeMap<RouteMetadata, IntegrationBuilder> {
		&mut self.queue
	}
	/// Insert a route into the queue to be built
	pub fn add_to_queue(
		&mut self,
		metadata: RouteMetadata,
		path: Route,
		cost_fields: &SectorCostFields,
	) {
		let int_builder = IntegrationBuilder::new(path, cost_fields);
		self.queue.insert(metadata, int_builder);
	}
	/// Get a [FlowField] based on the `sector_id` and `goal_id`. Returns
	/// [None] if the cache doesn't contain a record
	pub fn get_field(
		&self,
		current_sector_id: SectorID,
		goal_sector_id: SectorID,
		goal_id: FieldCell,
	) -> Option<&FlowField> {
		if current_sector_id == goal_sector_id {
			let flow_meta = FlowFieldMetadata {
				sector_id: current_sector_id,
				goal_id: Some(goal_id),
				portal_id: None,
				time_generated: Duration::default(),
			};
			self.flows.get(&flow_meta)
		} else {
			let flow_meta = FlowFieldMetadata {
				sector_id: current_sector_id,
				goal_id: None,
				portal_id: Some(goal_id),
				time_generated: Duration::default(),
			};
			self.flows.get(&flow_meta)
		}
	}
	/// Insert a [FlowField] into the cache with a sector-goal ID
	pub fn insert_field(
		&mut self,
		sector_id: SectorID,
		goal_id: Option<FieldCell>,
		portal_id: Option<FieldCell>,
		elapsed_duration: Duration,
		field: FlowField,
	) {
		let flow_meta = FlowFieldMetadata {
			sector_id,
			goal_id,
			portal_id,
			time_generated: elapsed_duration,
		};
		self.flows.insert(flow_meta, field);
	}
	/// Remove a [FlowField] from the cache (when it needs regenerating from a
	/// [CostField] update)
	pub fn remove_field(&mut self, flow_meta: FlowFieldMetadata) {
		self.flows.remove(&flow_meta);
	}
	/// Remove a [RouteMetadata] from the cache integration queue (when it
	/// needs regenerating from a [CostField] update)
	pub fn remove_queue_item(&mut self, route_meta: RouteMetadata) {
		self.queue.remove(&route_meta);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn field_cell_line_horizontal() {
		let source = FieldCell::new(3, 4);
		let target = FieldCell::new(7, 4);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(3, 4),
			FieldCell::new(4, 4),
			FieldCell::new(5, 4),
			FieldCell::new(6, 4),
			FieldCell::new(7, 4),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_horizontal_reverse() {
		let source = FieldCell::new(7, 4);
		let target = FieldCell::new(3, 4);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(7, 4),
			FieldCell::new(6, 4),
			FieldCell::new(5, 4),
			FieldCell::new(4, 4),
			FieldCell::new(3, 4),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_vertical() {
		let source = FieldCell::new(3, 4);
		let target = FieldCell::new(3, 7);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(3, 4),
			FieldCell::new(3, 5),
			FieldCell::new(3, 6),
			FieldCell::new(3, 7),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_vertical_reverse() {
		let source = FieldCell::new(3, 7);
		let target = FieldCell::new(3, 4);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(3, 7),
			FieldCell::new(3, 6),
			FieldCell::new(3, 5),
			FieldCell::new(3, 4),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_vertical_steep() {
		let source = FieldCell::new(3, 0);
		let target = FieldCell::new(4, 9);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(3, 0),
			FieldCell::new(3, 1),
			FieldCell::new(3, 2),
			FieldCell::new(3, 3),
			FieldCell::new(3, 4),
			FieldCell::new(4, 5),
			FieldCell::new(4, 6),
			FieldCell::new(4, 7),
			FieldCell::new(4, 8),
			FieldCell::new(4, 9),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_pos_gradient() {
		let source = FieldCell::new(3, 4);
		let target = FieldCell::new(7, 6);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(3, 4),
			FieldCell::new(4, 4),
			FieldCell::new(5, 5),
			FieldCell::new(6, 5),
			FieldCell::new(7, 6),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_pos_gradient_reverse() {
		let source = FieldCell::new(7, 6);
		let target = FieldCell::new(3, 4);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(7, 6),
			FieldCell::new(6, 5),
			FieldCell::new(5, 5),
			FieldCell::new(4, 4),
			FieldCell::new(3, 4),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_neg_gradient() {
		let source = FieldCell::new(3, 4);
		let target = FieldCell::new(7, 2);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(3, 4),
			FieldCell::new(4, 4),
			FieldCell::new(5, 3),
			FieldCell::new(6, 3),
			FieldCell::new(7, 2),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_neg_gradient_reverse() {
		let source = FieldCell::new(7, 2);
		let target = FieldCell::new(3, 4);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![
			FieldCell::new(7, 2),
			FieldCell::new(6, 3),
			FieldCell::new(5, 3),
			FieldCell::new(4, 4),
			FieldCell::new(3, 4),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn field_cell_line_zero() {
		let source = FieldCell::new(3, 4);
		let target = FieldCell::new(3, 4);
		let result = source.get_cells_between_points(&target);
		let actual: Vec<FieldCell> = vec![FieldCell::new(3, 4)];
		assert_eq!(actual, result);
	}
}
