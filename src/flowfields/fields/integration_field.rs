//! The IntegrationField contains a 2D array of 16-bit values and it uses a [CostField] to
//! produce a cumulative cost of reaching the goal/target. Every Sector has a [IntegrationField] associated with it.
//!
//! When a new route needs to be processed the field is reset to `u16::MAX` and the grid cell containing the goal is set to `0`. A series of passes are performed from the goal as an expanding wavefront calculating the field values:
//!
//! 1. The valid ordinal neighbours of the goal are determined (North, East, South, West, when not against a boundary)
//! 2. For each ordinal grid cell lookup their `CostField` value
//! 3. Add their cost to the `IntegrationField`s cost of the current cell (at the beginning this is the goal so + `0`)
//! 4. Propagate to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their integration cost, and repeat until the entire field is done
//!
//! This produces a nice diamond-like pattern as the wave expands (the underlying `CostField` are set to `1` here):
//!
//! ```text
//!  ___________________________________________________________
//! |     |     |     |     |     |     |     |     |     |     |
//! |  8  |  7  |  6  |  5  |  4  |  5  |  6  |  7  |  8  |  9  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  7  |  6  |  5  |  4  |  3  |  4  |  5  |  6  |  7  |  8  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  6  |  5  |  4  |  3  |  2  |  3  |  4  |  5  |  6  |  7  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  5  |  4  |  3  |  2  |  1  |  2  |  3  |  4  |  5  |  6  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  4  |  3  |  2  |  1  |  0  |  1  |  2  |  3  |  4  |  5  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  5  |  4  |  3  |  2  |  1  |  2  |  3  |  4  |  5  |  6  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  6  |  5  |  4  |  3  |  2  |  3  |  4  |  5  |  6  |  7  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  7  |  6  |  5  |  4  |  3  |  4  |  5  |  6  |  7  |  8  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  8  |  7  |  6  |  5  |  4  |  5  |  6  |  7  |  8  |  9  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  9  |  8  |  7  |  6  |  5  |  6  |  7  |  8  |  9  | 10  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! ```
//!
//! When it comes to `CostField` containing impassable markers, `255` as black boxes, they are ignored so the wave flows around those areas and when your `CostField` is using a range of values to indicate different areas to traverse, such as a steep hill, then you have various intermediate values similar to a terrain gradient.
//!
//! So this encourages the pathing algorithm around obstacles and expensive regions.
//!

use crate::prelude::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct IntegrationField([[u16; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for IntegrationField {
	fn default() -> Self {
		IntegrationField([[u16::MAX; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl Field<u16> for IntegrationField {
	/// Get a reference to the field array
	fn get_field(&self) -> &[[u16; FIELD_RESOLUTION]; FIELD_RESOLUTION] {
		&self.0
	}
	/// Retrieve a grid cell value
	fn get_grid_value(&self, column: usize, row: usize) -> u16 {
		if column >= self.0.len() || row >= self.0[0].len() {
			panic!("Cannot get a IntegrationField grid value, index out of bounds. Asked for column {}, row {}, grid column length is {}, grid row length is {}", column, row, self.0.len(), self.0[0].len())
		}
		self.0[column][row]
	}
	/// Set a grid cell to a value
	fn set_grid_value(&mut self, value: u16, column: usize, row: usize) {
		if column >= self.0.len() || row >= self.0[0].len() {
			panic!("Cannot set a IntegrationField grid value, index out of bounds. Asked for column {}, row {}, grid column length is {}, grid row length is {}", column, row, self.0.len(), self.0[0].len())
		}
		self.0[column][row] = value;
	}
}
impl IntegrationField {
	/// Creates a new [IntegrationField] where all cells are set to `u16::MAX` apart from the `goals` which is set to `0`
	pub fn new(goals: &Vec<(usize, usize)>) -> Self {
		let mut field = IntegrationField([[u16::MAX; FIELD_RESOLUTION]; FIELD_RESOLUTION]);
		for goal in goals {
			field.set_grid_value(0, goal.0, goal.1);
		}
		field
	}
	/// Reset all the cells of the [IntegrationField] to `u16::MAX` apart from the `goals` which are the starting points of calculating the field which is set to `0`
	pub fn reset(&mut self, goals: &Vec<(usize, usize)>) {
		for i in 0..FIELD_RESOLUTION {
			for j in 0..FIELD_RESOLUTION {
				self.set_grid_value(u16::MAX, i, j);
			}
		}
		for goal in goals {
			self.set_grid_value(0, goal.0, goal.1);
		}
	}
	/// From a list of `goals` (the actual end target goal or portal grid cells
	/// to the next sector towards the goal sector) grid cells iterate over
	/// successive neighbouring cells and calculate the field values from the
	/// `cost_field`
	pub fn calculate_field(&mut self, goals: &Vec<(usize, usize)>, cost_field: &CostField) {
		// further positions to process, tuple element 0 is the position, element 1 is the integration cost from the previous cell needed to help calculate element 0s cost
		let mut queue: Vec<((usize, usize), u16)> = Vec::new();
		for goal in goals.iter() {
			queue.push(((*goal), self.get_grid_value(goal.0, goal.1)));
		}
		process_neighbours(self, queue, cost_field);

		fn process_neighbours(
			int_field: &mut IntegrationField,
			queue: Vec<((usize, usize), u16)>,
			cost_field: &CostField,
		) {
			let mut next_neighbours = Vec::new();
			// iterate over the queue calculating neighbour int costs
			for (cell, prev_int_cost) in queue.iter() {
				let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
				// iterate over the neighbours calculating int costs
				for n in neighbours.iter() {
					let cell_cost = cost_field.get_grid_value(n.0, n.1);
					// ignore impassable cells
					if cell_cost != 255 {
						// don't overwrite an int cell with a better cost
						let int_cost = cell_cost as u16 + prev_int_cost;
						if int_cost < int_field.get_grid_value(n.0, n.1) {
							int_field.set_grid_value(int_cost, n.0, n.1);
							next_neighbours.push(((n.0, n.1), int_cost));
						}
					}
				}
			}
			if next_neighbours.len() != 0 {
				process_neighbours(int_field, next_neighbours, cost_field);
			}
		}
	}
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	/// Calculate integration field from a uniform cost field with a source near the centre
	#[test]
	fn basic_field() {
		let cost_field = CostField::default();
		let mut integration_field = IntegrationField::default();
		let source = vec![(4, 4)];
		integration_field.reset(&source);
		integration_field.calculate_field(&source, &cost_field);
		let result = integration_field.get_field();

		let actual: [[u16; FIELD_RESOLUTION]; FIELD_RESOLUTION] = [
			[8,7,6,5,4,5,6,7,8,9], [7,6,5,4,3,4,5,6,7,8], [6,5,4,3,2,3,4,5,6,7], [5,4,3,2,1,2,3,4,5,6], [4,3,2,1,0,1,2,3,4,5], [5,4,3,2,1,2,3,4,5,6], [6,5,4,3,2,3,4,5,6,7], [7,6,5,4,3,4,5,6,7,8], [8,7,6,5,4,5,6,7,8,9], [9,8,7,6,5,6,7,8,9,10]
		];


		assert_eq!(actual, *result);
	}
	/// Calculate integration field from a custom cost field set
	#[test]
	fn complex_field() {
		let mut cost_field = CostField::default();
		cost_field.set_grid_value(255, 5, 6);
		cost_field.set_grid_value(255, 5, 7);
		cost_field.set_grid_value(255, 6, 9);
		cost_field.set_grid_value(255, 6, 8);
		cost_field.set_grid_value(255, 6, 7);
		cost_field.set_grid_value(255, 6, 4);
		cost_field.set_grid_value(255, 7, 9);
		cost_field.set_grid_value(255, 7, 4);
		cost_field.set_grid_value(255, 8, 4);
		cost_field.set_grid_value(255, 9, 4);
		cost_field.set_grid_value(255, 1, 2);
		cost_field.set_grid_value(255, 1, 1);
		cost_field.set_grid_value(255, 2, 1);
		cost_field.set_grid_value(255, 2, 2);
		let mut integration_field = IntegrationField::default();
		let source = vec![(4, 4)];
		integration_field.reset(&source);
		integration_field.calculate_field(&source, &cost_field);
		let result = integration_field.get_field();

		let actual: [[u16; FIELD_RESOLUTION]; FIELD_RESOLUTION] = [
			[8,7,6,5,4,5,6,7,8,9], [7,65535,65535,4,3,4,5,6,7,8], [6,65535,65535,3,2,3,4,5,6,7], [5,4,3,2,1,2,3,4,5,6], [4,3,2,1,0,1,2,3,4,5], [5,4,3,2,1,2,65535,65535,5,6], [6,5,4,3,65535,3,4,65535,65535,65535], [7,6,5,4,65535,4,5,6,7,65535], [8,7,6,5,65535,5,6,7,8,9], [9,8,7,6,65535,6,7,8,9,10]
		];
		assert_eq!(actual, *result);
	}
}
