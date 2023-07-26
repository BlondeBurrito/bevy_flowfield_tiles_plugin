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

use std::collections::HashSet;

use crate::prelude::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
pub struct CostField([[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for CostField {
	fn default() -> Self {
		CostField([[1; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl Field<u8> for CostField {
	/// Get a reference to the field array
	fn get_field(&self) -> &[[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION] {
		&self.0
	}
	/// Retrieve a field cell value
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u8 {
		if field_cell.get_column() >= self.0.len() || field_cell.get_row() >= self.0[0].len() {
			panic!("Cannot get a CostField value, index out of bounds. Asked for column {}, row {}, field column length is {}, field row length is {}", field_cell.get_column(), field_cell.get_row(), self.0.len(), self.0[0].len())
		}
		self.0[field_cell.get_column()][field_cell.get_row()]
	}
	/// Set a field cell to a value
	fn set_field_cell_value(&mut self, value: u8, field_cell: FieldCell) {
		if field_cell.get_column() >= self.0.len() || field_cell.get_row() >= self.0[0].len() {
			panic!("Cannot set a CostField value, index out of bounds. Asked for column {}, row {}, field column length is {}, field row length is {}", field_cell.get_column(), field_cell.get_row(), self.0.len(), self.0[0].len())
		}
		self.0[field_cell.get_column()][field_cell.get_row()] = value;
	}
}
impl CostField {
	/// Tests whether two portals can see each other within a sector (one might be boxed in by impassable cost field values), additionally returns the number of steps taken to find a route between the two - this can be used as an edge weight
	pub fn can_internal_portal_pair_see_each_other(
		&self,
		source: FieldCell,
		target: FieldCell,
	) -> (bool, i32) {
		// instance of corner portals overlapping from cramped world
		if source == target {
			return (true, 0);
		}
		let queue = vec![source];
		// as nodes are visted we add them here to prevent the exploration from getting stuck in an infinite loop
		let visited = HashSet::new();
		let is_routable = process_neighbours(target, queue, visited, self, 0);
		/// Recursively process the cells to see if there's a path
		fn process_neighbours(
			target: FieldCell,
			queue: Vec<FieldCell>,
			mut visited: HashSet<FieldCell>,
			cost_field: &CostField,
			mut steps_taken: i32,
		) -> (bool, i32) {
			let mut next_neighbours = Vec::new();
			// iterate over the queue calculating neighbour int costs
			steps_taken += 1;
			for cell in queue.iter() {
				visited.insert(*cell);
				let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
				// iterate over the neighbours to try and find the target
				for n in neighbours.iter() {
					if *n == target {
						return (true, steps_taken);
					}
					let cell_cost = cost_field.get_field_cell_value(*n);
					// ignore impassable cells
					if cell_cost != 255 && !visited.contains(n) {
						// keep exploring
						next_neighbours.push(*n);
					}
				}
			}
			if !next_neighbours.is_empty() {
				process_neighbours(target, next_neighbours, visited, cost_field, steps_taken)
			} else {
				(false, steps_taken)
			}
		}
		is_routable
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

		let result = cost_field.can_internal_portal_pair_see_each_other(source, target);

		let actual = (true, 13);
		assert_eq!(actual, result)
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

		let result = cost_field.can_internal_portal_pair_see_each_other(source, target);

		let actual = (false, 14);
		assert_eq!(actual.0, result.0)
	}
}
