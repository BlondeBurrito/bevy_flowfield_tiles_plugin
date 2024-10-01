//! The CostField contains a 2D array of 8-bit values. The values correspond to the cost of that
//! cell in the array. A value of 1 is the default, a value of 255 is a special case that idicates
//! that the field cell is strictly forbidden from being used in a pathing calculation (effectively
//! saying there is a wall or cliff/impassable terrain there). Any other value indicates a harder
//! cost of movement which could be from a slope or marshland or others.
//!
//! Every Sector has a [CostField] associated with it. An example cost field may look:
//!
//! ```text
//!  ___________________________________________________________
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  |  1  | 255 | 255 | 255 | 255 | 255 |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  |  1  |  1  |  1  | 255 | 255 |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  | 255 |  1  |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  | 255 |  1  |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  | 255 | 255 |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  | 255 | 255 | 255 |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! ```
//!

use crate::prelude::*;
use bevy::reflect::Reflect;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Reflect)]
pub struct CostField([[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for CostField {
	fn default() -> Self {
		CostField([[1; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl Field<u8> for CostField {
	/// Get a reference to the field array
	fn get(&self) -> &[[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION] {
		&self.0
	}
	/// Retrieve a field cell value
	///
	/// NB: This will panic if out of bounds
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u8 {
		self.0[field_cell.get_column()][field_cell.get_row()]
	}
	/// Set a field cell to a value
	///
	/// NB: This will panic if out of bounds
	fn set_field_cell_value(&mut self, value: u8, field_cell: FieldCell) {
		self.0[field_cell.get_column()][field_cell.get_row()] = value;
	}
}
impl CostField {
	/// Create a new [CostField] with all cell values initialised with `cost`
	pub fn new_with_cost(cost: u8) -> Self {
		CostField([[cost; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
	/// Tests whether two cells can see each other within a sector (one might be boxed in by impassable cost field values)
	pub fn is_cell_pair_reachable(&self, source: FieldCell, target: FieldCell) -> bool {
		// instance of corner cells overlapping
		if source == target {
			return true;
		}
		let queue = vec![source];
		// as nodes are visted we add them here to prevent the exploration from getting stuck in an infinite loop, once a cell is visited it gets marked as true
		let mut propagation = [[false; FIELD_RESOLUTION]; FIELD_RESOLUTION];
		propagation[source.get_column()][source.get_row()] = true;
		process_neighbours_visibility(self, &target, queue, &mut propagation)
	}
	/// Counts the shorterst number of steps to another cell in the same sector if it's reachable
	pub fn get_distance_between_cells(
		&self,
		source: &FieldCell,
		target: &FieldCell,
	) -> Option<i32> {
		// instance of corner portals overlapping from cramped world
		if source == target {
			return Some(1);
		}
		let queue = vec![(*source, 0)];
		// as nodes are visted we add their cumulative cost here to prevent the exploration from getting stuck in an infinite loop, valid cells should be les than 65535
		let mut propagation = [[65535; FIELD_RESOLUTION]; FIELD_RESOLUTION];
		propagation[source.get_column()][source.get_row()] = 0;
		process_neighbours_distance(self, target, queue, &mut propagation)
	}
	/// From a `ron` file generate the [CostField]
	#[cfg(feature = "ron")]
	pub fn from_ron(path: String) -> Self {
		let file = std::fs::File::open(path).expect("Failed opening CostField file");
		let field: CostField = match ron::de::from_reader(file) {
			Ok(field) => field,
			Err(e) => panic!("Failed deserializing CostField: {}", e),
		};
		field
	}
}

/// Recursively process the cells to see if there's a path
fn process_neighbours_visibility(
	cost_field: &CostField,
	target: &FieldCell,
	queue: Vec<FieldCell>,
	propagation: &mut [[bool; FIELD_RESOLUTION]; FIELD_RESOLUTION],
) -> bool {
	let mut next_queue = vec![];
	// iterate over the queue to explore neighbours
	for cell in queue.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
		// iterate over the neighbours to try and find the target
		for n in neighbours.iter() {
			if *n == *target {
				return true;
			}
			let cell_cost = cost_field.get_field_cell_value(*n);
			// ignore impassable cells
			if cell_cost != 255 {
				let (column, row) = n.get_column_row();
				let has_existing_propagation = propagation[column][row];
				if !has_existing_propagation {
					propagation[column][row] = true;
					// keep exploring
					next_queue.push(*n);
				}
			}
		}
	}
	if !next_queue.is_empty() {
		process_neighbours_visibility(cost_field, target, next_queue, propagation)
	} else {
		false
	}
}
/// Recursively process the cells to see if there's a path and a weighting for the distance between the cell pair
fn process_neighbours_distance(
	cost_field: &CostField,
	target: &FieldCell,
	queue: Vec<(FieldCell, i32)>,
	propagation: &mut [[i32; FIELD_RESOLUTION]; FIELD_RESOLUTION],
) -> Option<i32> {
	let mut next_queue = vec![];
	for (cell, prev_cost) in queue.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
		for n in neighbours {
			let n_cost = cost_field.get_field_cell_value(n);
			// ignore impassable
			if n_cost != 255 {
				// let cumulative_cost = n_cost as i32 + prev_cost;
				let cumulative_cost = 1_i32 + prev_cost;
				let (column, row) = n.get_column_row();
				let existing_propagation_cost = propagation[column][row];
				if cumulative_cost < existing_propagation_cost {
					propagation[column][row] = cumulative_cost;
					next_queue.push((n, cumulative_cost));
				}
			}
		}
	}
	if !next_queue.is_empty() {
		process_neighbours_distance(cost_field, target, next_queue, propagation)
	} else {
		let target_cumulative = propagation[target.get_column()][target.get_row()];
		if target_cumulative != 65535 {
			Some(target_cumulative)
		} else {
			None
		}
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn get_cost_field_value() {
		let mut cost_field = CostField::default();
		let field_cell = FieldCell::new(9, 9);
		cost_field.set_field_cell_value(255, field_cell);
		let result = cost_field.get_field_cell_value(field_cell);
		let actual: u8 = 255;
		assert_eq!(actual, result);
	}
	#[test]
	#[cfg(feature = "ron")]
	fn cost_field_file() {
		let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/cost_field.ron";
		let _cost_field = CostField::from_ron(path);
	}
	#[test]
	fn internal_portal_visibility_true() {
		//  _____________________________
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |P_|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|x_|x_|__|__|__|
		// |__|__|__|__|__|x_|__|__|__|__|
		// |__|__|__|__|__|x_|P_|__|__|__|
		let mut cost_field = CostField::default();
		cost_field.set_field_cell_value(255, FieldCell::new(5, 9));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 8));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 7));
		let source = FieldCell::new(0, 4);
		let target = FieldCell::new(6, 9);

		let result = cost_field.is_cell_pair_reachable(source, target);
		assert!(result)
	}
	#[test]
	fn internal_portal_visibility_false() {
		//  _____________________________
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |P_|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|x_|x_|x_|__|__|
		// |__|__|__|__|__|x_|__|x_|__|__|
		// |__|__|__|__|__|x_|P_|x_|__|__|
		let mut cost_field = CostField::default();
		cost_field.set_field_cell_value(255, FieldCell::new(5, 9));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 8));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 8));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 9));
		let source = FieldCell::new(0, 4);
		let target = FieldCell::new(6, 9);

		let result = cost_field.is_cell_pair_reachable(source, target);

		let actual = false;
		assert_eq!(actual, result)
	}
	#[test]
	fn internal_cell_distance_some() {
		//  _____________________________
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |P_|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|x_|x_|__|__|__|
		// |__|__|__|__|__|x_|__|__|__|__|
		// |__|__|__|__|__|x_|P_|__|__|__|
		let mut cost_field = CostField::default();
		cost_field.set_field_cell_value(255, FieldCell::new(5, 9));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 8));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 7));
		let source = FieldCell::new(0, 4);
		let target = FieldCell::new(6, 9);

		let result = cost_field.get_distance_between_cells(&source, &target);
		assert_eq!(Some(13), result)
	}
	#[test]
	fn internal_cell_distance_none() {
		//  _____________________________
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |P_|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|__|__|__|__|__|
		// |__|__|__|__|__|x_|x_|x_|__|__|
		// |__|__|__|__|__|x_|__|x_|__|__|
		// |__|__|__|__|__|x_|P_|x_|__|__|
		let mut cost_field = CostField::default();
		cost_field.set_field_cell_value(255, FieldCell::new(5, 9));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 8));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 8));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 9));
		let source = FieldCell::new(0, 4);
		let target = FieldCell::new(6, 9);

		let result = cost_field.get_distance_between_cells(&source, &target);
		assert!(result.is_none())
	}
}
