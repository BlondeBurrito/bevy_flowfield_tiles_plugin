//! The CostField contains a 2D array of 8-bit values. The values correspond to the cost of that
//! grid in the array. A value of 1 is the default, a value of 255 is a special case that idicates
//! that the grid cell is strictly forbidden from being used in a pathing calculation (effectively
//! saying there is a wall or cliff/impassable terrain there). Any other value indicates a harder
//! cost of movement which could be from a slope or marshland or others.
//!
//! Every [Sector] has a [CostsField] associated with it. An example cost field may look:
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

use super::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CostField([[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for CostField {
	fn default() -> Self {
		CostField([[1; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl CostField {
	pub fn get_grid_value(&self, column: usize, row: usize) -> u8 {
		if column >= self.0.len() || row >= self.0[0].len() {
			panic!("Cannot get a CostField grid value, index out of bounds. Asked for column {}, row {}, grid column length is {}, grid row length is {}", column, row, self.0.len(), self.0[0].len())
		}
		self.0[column][row]
	}
	pub fn set_grid_value(&mut self, value: u8, column: usize, row: usize) {
		if column >= self.0.len() || row >= self.0[0].len() {
			panic!("Cannot set a CostField grid value, index out of bounds. Asked for column {}, row {}, grid column length is {}, grid row length is {}", column, row, self.0.len(), self.0[0].len())
		}
		self.0[column][row] = value;
	}
	/// Tests whether two portals can see each other within a sector (one might be boxed in by impassable cost field values), additionally returns the number of steps taken to find a route between the two - this can be used as an edge weight
	pub fn can_internal_portal_pair_see_each_other(&self, source: (usize, usize), target: (usize, usize)) -> (bool, i32) {
		// instance of corner portals overlapping from cramped world
		if source == target {
			return (true, 0)
		}
		let queue = vec![source];
		// as nodes are visted we add them here to prevent the exploration from getting stuck in an infinite loop
		let visited = HashSet::new();
		let is_routable = process_neighbours(target, queue, visited, &self, 0);

		fn process_neighbours(
			target: (usize, usize),
			queue: Vec<(usize, usize)>,
			mut visited: HashSet<(usize, usize)>,
			cost_field: &CostField,
			mut steps_taken: i32,
		) -> (bool, i32) {
			let mut next_neighbours = Vec::new();
			// iterate over the queue calculating neighbour int costs
			steps_taken += 1;
			for cell in queue.iter() {
				visited.insert(*cell);
				let neighbours = Ordinal::get_cell_neighbours(*cell);
				// iterate over the neighbours to try and find the target
				for n in neighbours.iter() {
					if *n == target {
						return (true, steps_taken)
					}
					let cell_cost = cost_field.get_grid_value(n.0, n.1);
					// ignore impassable cells
					if cell_cost != 255 && !visited.contains(&(n.0, n.1)) {
						// keep exploring
						next_neighbours.push((n.0, n.1));
					}
				}
			}
			if next_neighbours.len() != 0 {
				process_neighbours(target, next_neighbours, visited, cost_field, steps_taken)
			} else {
				(false, steps_taken)
			}
		}
		is_routable
	}
	/// From a `ron` file generate the [CostField]
	#[cfg(feature = "ron")]
	pub fn from_file(path: String) -> Self {
		let file = std::fs::File::open(&path).expect("Failed opening CostField file");
		let field: CostField = match ron::de::from_reader(file) {
			Ok(field) => field,
			Err(e) => panic!("Failed deserializing CostField: {}", e),
		};
		field
	}
}

// /// A [CostField] grid is made up of a ([SECTOR_RESOLUTION]x[SECTOR_RESOLUTION]) array
// fn get_inidices_of_neighbouring_grid_cells(
// 	grid_cell: (u32, u32),
// 	map_x_dimension: u32,
// 	map_z_dimension: u32,
// ) -> Vec<(u32, u32)> {
// 	let sector_x_column_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
// 	let sector_z_row_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;

// 	if sector_id.0 == 0 && sector_id.1 == 0 {
// 		//top left sector only has 2 valid neighbours
// 		vec![(1, 0), (0, 1)]
// 	} else if sector_id.0 == sector_x_column_limit && sector_id.1 == 0 {
// 		// top right sector has only two valid neighbours
// 		vec![(sector_x_column_limit, 1), (sector_x_column_limit - 1, 0)]
// 	} else if sector_id.0 == sector_x_column_limit && sector_id.1 == sector_z_row_limit {
// 		// bottom right sector only has two valid neighbours
// 		vec![
// 			(sector_x_column_limit, sector_z_row_limit - 1),
// 			(sector_x_column_limit - 1, sector_z_row_limit),
// 		]
// 	} else if sector_id.0 == 0 && sector_id.1 == sector_z_row_limit {
// 		// bottom left sector only has two valid neighbours
// 		vec![(0, sector_z_row_limit - 1), (1, sector_z_row_limit)]
// 	} else if sector_id.0 > 0 && sector_id.0 < sector_x_column_limit && sector_id.1 == 0 {
// 		// northern row minus the corners sectors have three valid neighbours
// 		vec![(sector_id.0 + 1, 0), (sector_id.0, 1), (sector_id.0 - 1, 0)]
// 	} else if sector_id.0 == sector_x_column_limit
// 		&& sector_id.1 > 0
// 		&& sector_id.1 < sector_z_row_limit
// 	{
// 		// eastern column minus the corners have three sectors of valid neighbours
// 		vec![
// 			(sector_x_column_limit, sector_id.1 - 1),
// 			(sector_x_column_limit, sector_id.1 + 1),
// 			(sector_x_column_limit - 1, sector_id.1),
// 		]
// 	} else if sector_id.0 > 0
// 		&& sector_id.0 < sector_x_column_limit
// 		&& sector_id.1 == sector_z_row_limit
// 	{
// 		// southern row minus corners have three sectors of valid neighbours
// 		vec![
// 			(sector_id.0, sector_z_row_limit - 1),
// 			(sector_id.0 + 1, sector_z_row_limit),
// 			(sector_id.0 - 1, sector_z_row_limit),
// 		]
// 	} else if sector_id.0 == 0 && sector_id.1 > 0 && sector_id.1 < sector_z_row_limit {
// 		// western column minus corners have three sectors of valid neighbours
// 		vec![(0, sector_id.1 - 1), (1, sector_id.1), (0, sector_id.1 + 1)]
// 	} else if sector_id.0 > 0 && sector_id.0 < sector_x_column_limit && sector_id.1 > 0 && sector_id.1 < sector_z_row_limit {
// 		// all other sectors not along an edge of the map have four valid sectors for portals
// 		vec![
// 			(sector_id.0, sector_id.1 - 1),
// 			(sector_id.0 + 1, sector_id.1),
// 			(sector_id.0, sector_id.1 + 1),
// 			(sector_id.0 - 1, sector_id.1),
// 		]
// 	} else {
// 		// special case of no neighbours
// 		warn!("Sector ID {:?} does not fit within map dimensions, there are only `{}x{}` sectors", sector_id, map_x_dimension / SECTOR_RESOLUTION as u32, map_z_dimension / SECTOR_RESOLUTION as u32);
// 		vec![]
// 	}
// }

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn get_cost_field_value() {
		let mut cost_field = CostField::default();
		cost_field.set_grid_value(255, 9, 9);
		let result = cost_field.get_grid_value(9, 9);
		let actual: u8 = 255;
		assert_eq!(actual, result);
	}
	#[test]
	#[cfg(feature = "ron")]
	fn cost_field_file() {
		let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/cost_field.ron";
		let _cost_field = CostField::from_file(path);
		assert!(true)
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
		cost_field.set_grid_value(255, 5, 9);
		cost_field.set_grid_value(255, 5, 8);
		cost_field.set_grid_value(255, 5, 7);
		cost_field.set_grid_value(255, 6, 7);
		let source = (0, 4);
		let target = (6, 9);

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
		cost_field.set_grid_value(255, 5, 9);
		cost_field.set_grid_value(255, 5, 8);
		cost_field.set_grid_value(255, 5, 7);
		cost_field.set_grid_value(255, 6, 7);
		cost_field.set_grid_value(255, 7, 7);
		cost_field.set_grid_value(255, 7, 8);
		cost_field.set_grid_value(255, 7, 9);
		let source = (0, 4);
		let target = (6, 9);

		let result = cost_field.can_internal_portal_pair_see_each_other(source, target);

		let actual = (false, 14);
		assert_eq!(actual.0, result.0)
	}
}
