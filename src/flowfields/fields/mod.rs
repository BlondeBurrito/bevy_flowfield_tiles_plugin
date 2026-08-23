//! The kinds of fields used by the algorithm
//!

pub mod bresenham;
pub mod cost_field;
pub mod flow_field;
pub mod integration_field;

use bevy::prelude::*;

use crate::flowfields::{
	fields::bresenham::{walk_bresenham_shallow, walk_bresenham_steep},
	utilities::{CompassDir, FIELD_RESOLUTION},
};

/// Defines required access to field arrays
pub trait Field<T> {
	/// Get a reference to the field array
	fn get(&self) -> &[T; FIELD_RESOLUTION * FIELD_RESOLUTION];
	/// Retrieve a field cell value
	fn get_field_cell_value(&self, field_cell: FieldCell) -> T;
	/// Set a field cell to a value
	fn set_field_cell_value(&mut self, value: T, field_cell: FieldCell);
}

/// ID of a cell within a field
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Reflect)]
pub struct FieldCell {
	/// Column
	column: usize,
	/// Row
	row: usize,
}

impl std::fmt::Display for FieldCell {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Column: {}, Row: {}", self.column, self.row)
	}
}

impl FieldCell {
	/// Create a new instance of [FieldCell]
	pub fn new(column: usize, row: usize) -> Self {
		if column > FIELD_RESOLUTION - 1 || row > FIELD_RESOLUTION - 1 {
			panic!(
				"FieldCell must be within {} bounds, supplied col {}, row {}",
				FIELD_RESOLUTION, column, row
			)
		}
		FieldCell { column, row }
	}
	/// Get the sector `(column, row)` tuple
	pub fn get_column_row(&self) -> (usize, usize) {
		(self.column, self.row)
	}
	/// Get the sector column
	pub fn get_column(&self) -> usize {
		self.column
	}
	/// Get the sector row
	pub fn get_row(&self) -> usize {
		self.row
	}
	/// Convert the column-row representation into a 1-dimensional index that fits
	/// into field array size
	pub fn as_1d_index(&self) -> usize {
		self.get_row() * FIELD_RESOLUTION + self.get_column()
	}
	/// From a flat index use field dimensions to calculate [FieldCell]
	pub fn from_index(index: usize) -> Self {
		let row = index / FIELD_RESOLUTION;
		let col = index % FIELD_RESOLUTION;
		FieldCell::new(col, row)
	}
	/// Try and get a [FieldCell] a number of `steps` away in a particular
	/// [CompassDir] direction
	pub fn get_in_compass_direction(
		&self,
		compass_dir: &CompassDir,
		steps: usize,
	) -> Option<FieldCell> {
		let (column, row) = match compass_dir {
			CompassDir::North => {
				let this_row = self.row;
				let n = this_row.checked_sub(steps)?;
				(self.column, n)
			}
			CompassDir::East => {
				let this_col = self.column;
				let n = this_col + steps;
				if n < FIELD_RESOLUTION {
					(n, self.row)
				} else {
					return None;
				}
			}
			CompassDir::South => {
				let this_row = self.row;
				let n = this_row + steps;
				if n < FIELD_RESOLUTION {
					(self.column, n)
				} else {
					return None;
				}
			}
			CompassDir::West => {
				let this_col = self.column;
				let n = this_col.checked_sub(steps)?;
				(n, self.row)
			}
			CompassDir::NorthEast => {
				let this_row = self.row;
				let n_row = this_row.checked_sub(steps)?;
				let this_col = self.column;
				let n_col = this_col + steps;
				if n_col < FIELD_RESOLUTION {
					(n_col, n_row)
				} else {
					return None;
				}
			}
			CompassDir::SouthEast => {
				let this_row = self.row;
				let n_row = this_row + steps;
				if n_row < FIELD_RESOLUTION {
					let this_col = self.column;
					let n_col = this_col + steps;
					if n_col < FIELD_RESOLUTION {
						(n_col, n_row)
					} else {
						return None;
					}
				} else {
					return None;
				}
			}
			CompassDir::SouthWest => {
				let this_row = self.row;
				let n_row = this_row + steps;
				if n_row < FIELD_RESOLUTION {
					let this_col = self.column;
					let n_col = this_col.checked_sub(steps)?;
					(n_col, n_row)
				} else {
					return None;
				}
			}
			CompassDir::NorthWest => {
				let this_row = self.row;
				let n_row = this_row.checked_sub(steps)?;
				let this_col = self.column;
				let n_col = this_col.checked_sub(steps)?;
				(n_col, n_row)
			}
			_ => panic!(
				"{} should never be used for FieldCell stepping",
				compass_dir
			),
		};
		Some(FieldCell::new(column, row))
	}
	// /// In a given [CompassDir] find the first [FieldCell] in the adjacent sector
	// pub fn get_sector_entry_cell(&self, compass_dir: &CompassDir) -> FieldCell {
	// 	match compass_dir {
	// 		CompassDir::North => FieldCell::new(self.get_column(), FIELD_RESOLUTION - 1),
	// 		CompassDir::East => FieldCell::new(0, self.get_row()),
	// 		CompassDir::South => FieldCell::new(self.get_column(), 0),
	// 		CompassDir::West => FieldCell::new(FIELD_RESOLUTION - 1, self.get_row()),
	// 		CompassDir::NorthEast => {
	// 			if self.column + self.row == FIELD_RESOLUTION - 1 {
	// 				//TODO wrong
	// 				FieldCell::new(0, FIELD_RESOLUTION - 1)
	// 			} else if self.column > self.row {
	// 				//TODO WRONG
	// 				FieldCell::new(self.column + self.row + 1, FIELD_RESOLUTION - 1)
	// 			}
	// 			// problem, where this used (scaling) the diagonal can end up in the wrong sector
	// 			// go up and right can result in entering any of 3 sectors
	// 			// in sector_cost this means scaling can actually go over multiple sectors

	// 			if self.row == 0 {
	// 				FieldCell::new(0, FIELD_RESOLUTION - 1)
	// 			} else {
	// 				FieldCell::new(0, FIELD_RESOLUTION - 1 - self.get_row())
	// 			}
	// 		}
	// 		CompassDir::SouthEast => FieldCell::new(0, 0),
	// 		CompassDir::SouthWest => FieldCell::new(FIELD_RESOLUTION - 1, 0),
	// 		CompassDir::NorthWest => FieldCell::new(FIELD_RESOLUTION - 1, FIELD_RESOLUTION - 1),
	// 		_ => panic!("{} should never be used for finding entry cell", compass_dir),
	// 	}
	// }
	/// Get the adjacent neighbours of a [FieldCell]
	pub fn get_orthogonal_neighbours(&self) -> Vec<FieldCell> {
		let mut neighbours = vec![];

		// north
		if self.row > 0 {
			neighbours.push(FieldCell::new(self.column, self.row - 1));
		}
		// east
		if self.column < FIELD_RESOLUTION - 1 {
			neighbours.push(FieldCell::new(self.column + 1, self.row));
		}
		// south
		if self.row < FIELD_RESOLUTION - 1 {
			neighbours.push(FieldCell::new(self.column, self.row + 1));
		}
		// west
		if self.column > 0 {
			neighbours.push(FieldCell::new(self.column - 1, self.row));
		}
		neighbours
	}
	/// Get the [CompassDir] from `self` to `rhs`
	pub fn dir_from_this_to_rhs(&self, rhs: &FieldCell) -> CompassDir {
		if rhs.row < self.row {
			// NW, N or NE
			if rhs.column > self.column {
				CompassDir::NorthEast
			} else if rhs.column < self.column {
				CompassDir::NorthWest
			} else {
				CompassDir::North
			}
		} else if rhs.row > self.row {
			// SW, S or SE
			if rhs.column > self.column {
				CompassDir::SouthEast
			} else if rhs.column < self.column {
				CompassDir::SouthWest
			} else {
				CompassDir::South
			}
		} else {
			// E or W, or `rhs` is `self`
			if rhs.column > self.column {
				CompassDir::East
			} else if rhs.column < self.column {
				CompassDir::West
			} else {
				CompassDir::Zero
			}
		}
	}
	/// Using the Bresenham line algorithm get a list of [FieldCell] that lie
	/// along a line between two points. Note that the list will contain the
	/// source (`self`) and `target` [FieldCell]
	pub fn get_cells_between_points(&self, target: &FieldCell) -> Vec<FieldCell> {
		let source_col = self.get_column() as i32;
		let source_row = self.get_row() as i32;
		let target_col = target.get_column() as i32;
		let target_row = target.get_row() as i32;

		// optimise for orthogonal line (horizontal or vertical)
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn to_index() {
		let cell = FieldCell::new(1, 1);
		let actual = 11;
		let result = cell.as_1d_index();
		assert_eq!(actual, result);
	}

	#[test]
	fn from_index() {
		let index = 12;
		let actual = FieldCell::new(2, 1);
		let result = FieldCell::from_index(index);
		assert_eq!(actual, result);
	}

	#[test]
	fn compass_dir_north_valid() {
		let cell = FieldCell::new(0, 1);
		let actual = FieldCell::new(0, 0);
		let result = cell.get_in_compass_direction(&CompassDir::North, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn compass_dir_north_invalid() {
		let cell = FieldCell::new(0, 0);
		let result = cell.get_in_compass_direction(&CompassDir::North, 1);
		assert!(result.is_none())
	}

	#[test]
	fn compass_dir_east_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(2, 1);
		let result = cell.get_in_compass_direction(&CompassDir::East, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn compass_dir_east_invalid() {
		let cell = FieldCell::new(9, 0);
		let result = cell.get_in_compass_direction(&CompassDir::East, 1);
		assert!(result.is_none())
	}

	#[test]
	fn compass_dir_south_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(1, 2);
		let result = cell.get_in_compass_direction(&CompassDir::South, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn compass_dir_south_invalid() {
		let cell = FieldCell::new(0, 9);
		let result = cell.get_in_compass_direction(&CompassDir::South, 1);
		assert!(result.is_none())
	}

	#[test]
	fn compass_dir_west_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(0, 1);
		let result = cell.get_in_compass_direction(&CompassDir::West, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn compass_dir_west_invalid() {
		let cell = FieldCell::new(0, 0);
		let result = cell.get_in_compass_direction(&CompassDir::West, 1);
		assert!(result.is_none())
	}

	#[test]
	fn compass_dir_northeast_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(2, 0);
		let result = cell.get_in_compass_direction(&CompassDir::NorthEast, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn compass_dir_northeast_invalid() {
		let cell = FieldCell::new(0, 0);
		let result = cell.get_in_compass_direction(&CompassDir::NorthEast, 1);
		assert!(result.is_none())
	}

	#[test]
	fn compass_dir_southeast_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(2, 2);
		let result = cell.get_in_compass_direction(&CompassDir::SouthEast, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn compass_dir_southeast_invalid() {
		let cell = FieldCell::new(9, 9);
		let result = cell.get_in_compass_direction(&CompassDir::SouthEast, 1);
		assert!(result.is_none())
	}

	#[test]
	fn compass_dir_southwest_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(0, 2);
		let result = cell.get_in_compass_direction(&CompassDir::SouthWest, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn compass_dir_southwest_invalid() {
		let cell = FieldCell::new(0, 9);
		let result = cell.get_in_compass_direction(&CompassDir::SouthWest, 1);
		assert!(result.is_none())
	}

	#[test]
	fn compass_dir_northwest_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(0, 0);
		let result = cell.get_in_compass_direction(&CompassDir::NorthWest, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn compass_dir_northwest_invalid() {
		let cell = FieldCell::new(0, 0);
		let result = cell.get_in_compass_direction(&CompassDir::NorthWest, 1);
		assert!(result.is_none())
	}

	// #[test]
	// fn sector_entry_north() {
	// 	let cell = FieldCell::new(4, 4);
	// 	let actual = FieldCell::new(4, 9);
	// 	let result = cell.get_sector_entry_cell(&CompassDir::North);
	// 	assert_eq!(actual, result);
	// }

	// #[test]
	// fn sector_entry_east() {
	// 	let cell = FieldCell::new(4, 4);
	// 	let actual = FieldCell::new(0, 4);
	// 	let result = cell.get_sector_entry_cell(&CompassDir::East);
	// 	assert_eq!(actual, result);
	// }

	// #[test]
	// fn sector_entry_south() {
	// 	let cell = FieldCell::new(4, 4);
	// 	let actual = FieldCell::new(4, 0);
	// 	let result = cell.get_sector_entry_cell(&CompassDir::South);
	// 	assert_eq!(actual, result);
	// }

	// #[test]
	// fn sector_entry_west() {
	// 	let cell = FieldCell::new(4, 4);
	// 	let actual = FieldCell::new(9, 4);
	// 	let result = cell.get_sector_entry_cell(&CompassDir::West);
	// 	assert_eq!(actual, result);
	// }

	#[test]
	fn neighbours2() {
		let cell = FieldCell::new(9, 0);
		let actual = vec![FieldCell::new(9, 1), FieldCell::new(8, 0)];
		let result = cell.get_orthogonal_neighbours();
		assert_eq!(actual, result);
	}

	#[test]
	fn neighbours3() {
		let cell = FieldCell::new(0, 4);
		let actual = vec![
			FieldCell::new(0, 3),
			FieldCell::new(1, 4),
			FieldCell::new(0, 5),
		];
		let result = cell.get_orthogonal_neighbours();
		assert_eq!(actual, result);
	}

	#[test]
	fn neighbours4() {
		let cell = FieldCell::new(4, 4);
		let actual = vec![
			FieldCell::new(4, 3),
			FieldCell::new(5, 4),
			FieldCell::new(4, 5),
			FieldCell::new(3, 4),
		];
		let result = cell.get_orthogonal_neighbours();
		assert_eq!(actual, result);
	}

	#[test]
	fn dir_from_to_north() {
		let this = FieldCell::new(3, 4);
		let other = FieldCell::new(3, 3);
		let result = this.dir_from_this_to_rhs(&other);
		let actual = CompassDir::North;
		assert_eq!(actual, result);
	}
	#[test]
	fn dir_from_to_north_east() {
		let this = FieldCell::new(3, 4);
		let other = FieldCell::new(4, 3);
		let result = this.dir_from_this_to_rhs(&other);
		let actual = CompassDir::NorthEast;
		assert_eq!(actual, result);
	}
	#[test]
	fn dir_from_to_east() {
		let this = FieldCell::new(3, 4);
		let other = FieldCell::new(4, 4);
		let result = this.dir_from_this_to_rhs(&other);
		let actual = CompassDir::East;
		assert_eq!(actual, result);
	}
	#[test]
	fn dir_from_to_south_east() {
		let this = FieldCell::new(3, 4);
		let other = FieldCell::new(4, 5);
		let result = this.dir_from_this_to_rhs(&other);
		let actual = CompassDir::SouthEast;
		assert_eq!(actual, result);
	}
	#[test]
	fn dir_from_to_south() {
		let this = FieldCell::new(3, 4);
		let other = FieldCell::new(3, 5);
		let result = this.dir_from_this_to_rhs(&other);
		let actual = CompassDir::South;
		assert_eq!(actual, result);
	}
	#[test]
	fn dir_from_to_south_west() {
		let this = FieldCell::new(3, 4);
		let other = FieldCell::new(2, 5);
		let result = this.dir_from_this_to_rhs(&other);
		let actual = CompassDir::SouthWest;
		assert_eq!(actual, result);
	}
	#[test]
	fn dir_from_to_north_west() {
		let this = FieldCell::new(3, 4);
		let other = FieldCell::new(2, 3);
		let result = this.dir_from_this_to_rhs(&other);
		let actual = CompassDir::NorthWest;
		assert_eq!(actual, result);
	}
	#[test]
	fn dir_from_to_zero() {
		let this = FieldCell::new(3, 4);
		let other = FieldCell::new(3, 4);
		let result = this.dir_from_this_to_rhs(&other);
		let actual = CompassDir::Zero;
		assert_eq!(actual, result);
	}
}
