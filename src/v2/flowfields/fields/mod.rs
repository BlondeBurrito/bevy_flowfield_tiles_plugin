//! The kinds of fields used by the algorithm
//!

pub mod bresenham;
pub mod cost_field;
pub mod flow_field;
pub mod integration_field;

use bevy::prelude::*;

use crate::v2::flowfields::utilities::{FIELD_RESOLUTION, Ordinal};

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
	column: usize,
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
	/// Convert the column-row representation into a 1-dimensional index that fits into field array size
	pub fn as_1d_index(&self) -> usize {
		self.get_row() * FIELD_RESOLUTION + self.get_column()
	}
	/// From a flat index use field dimensions to calculate [FieldCell]
	pub fn from_index(index: usize) -> Self {
		let row = index / FIELD_RESOLUTION;
		let col = index % FIELD_RESOLUTION;
		FieldCell::new(col, row)
	}
	/// Try and get a [FieldCell] a number of `steps` away in a particular [Ordinal] direction
	pub fn get_in_ordinal_direction(&self, ordinal: &Ordinal, steps: usize) -> Option<FieldCell> {
		let (column, row) = match ordinal {
			Ordinal::North => {
				let this_row = self.row;
				if let Some(n) = this_row.checked_sub(steps) {
					(self.column, n)
				} else {
					return None;
				}
			}
			Ordinal::East => {
				let this_col = self.column;
				let n = this_col + steps;
				if n < FIELD_RESOLUTION {
					(n, self.row)
				} else {
					return None;
				}
			}
			Ordinal::South => {
				let this_row = self.row;
				let n = this_row + steps;
				if n < FIELD_RESOLUTION {
					(self.column, n)
				} else {
					return None;
				}
			}
			Ordinal::West => {
				let this_col = self.column;
				if let Some(n) = this_col.checked_sub(steps) {
					(n, self.row)
				} else {
					return None;
				}
			}
			Ordinal::NorthEast => {
				let this_row = self.row;
				if let Some(n_row) = this_row.checked_sub(steps) {
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
			Ordinal::SouthEast => {
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
			Ordinal::SouthWest => {
				let this_row = self.row;
				let n_row = this_row + steps;
				if n_row < FIELD_RESOLUTION {
					let this_col = self.column;
					if let Some(n_col) = this_col.checked_sub(steps) {
						(n_col, n_row)
					} else {
						return None;
					}
				} else {
					return None;
				}
			}
			Ordinal::NorthWest => {
				let this_row = self.row;
				if let Some(n_row) = this_row.checked_sub(steps) {
					let this_col = self.column;
					if let Some(n_col) = this_col.checked_sub(steps) {
						(n_col, n_row)
					} else {
						return None;
					}
				} else {
					return None;
				}
			}
			_ => panic!("{} should never be used for FieldCell stepping", ordinal),
		};
		Some(FieldCell::new(column, row))
	}
	// /// In a given [Ordinal] find the first [FieldCell] in the adjacent sector
	// pub fn get_sector_entry_cell(&self, ordinal: &Ordinal) -> FieldCell {
	// 	match ordinal {
	// 		Ordinal::North => FieldCell::new(self.get_column(), FIELD_RESOLUTION - 1),
	// 		Ordinal::East => FieldCell::new(0, self.get_row()),
	// 		Ordinal::South => FieldCell::new(self.get_column(), 0),
	// 		Ordinal::West => FieldCell::new(FIELD_RESOLUTION - 1, self.get_row()),
	// 		Ordinal::NorthEast => {
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
	// 		Ordinal::SouthEast => FieldCell::new(0, 0),
	// 		Ordinal::SouthWest => FieldCell::new(FIELD_RESOLUTION - 1, 0),
	// 		Ordinal::NorthWest => FieldCell::new(FIELD_RESOLUTION - 1, FIELD_RESOLUTION - 1),
	// 		_ => panic!("{} should never be used for finding entry cell", ordinal),
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
	fn ordinal_north_valid() {
		let cell = FieldCell::new(0, 1);
		let actual = FieldCell::new(0, 0);
		let result = cell.get_in_ordinal_direction(&Ordinal::North, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn ordinal_north_invalid() {
		let cell = FieldCell::new(0, 0);
		let result = cell.get_in_ordinal_direction(&Ordinal::North, 1);
		assert!(result.is_none())
	}

	#[test]
	fn ordinal_east_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(2, 1);
		let result = cell.get_in_ordinal_direction(&Ordinal::East, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn ordinal_east_invalid() {
		let cell = FieldCell::new(9, 0);
		let result = cell.get_in_ordinal_direction(&Ordinal::East, 1);
		assert!(result.is_none())
	}

	#[test]
	fn ordinal_south_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(1, 2);
		let result = cell.get_in_ordinal_direction(&Ordinal::South, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn ordinal_south_invalid() {
		let cell = FieldCell::new(0, 9);
		let result = cell.get_in_ordinal_direction(&Ordinal::South, 1);
		assert!(result.is_none())
	}

	#[test]
	fn ordinal_west_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(0, 1);
		let result = cell.get_in_ordinal_direction(&Ordinal::West, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn ordinal_west_invalid() {
		let cell = FieldCell::new(0, 0);
		let result = cell.get_in_ordinal_direction(&Ordinal::West, 1);
		assert!(result.is_none())
	}

	#[test]
	fn ordinal_northeast_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(2, 0);
		let result = cell.get_in_ordinal_direction(&Ordinal::NorthEast, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn ordinal_northeast_invalid() {
		let cell = FieldCell::new(0, 0);
		let result = cell.get_in_ordinal_direction(&Ordinal::NorthEast, 1);
		assert!(result.is_none())
	}

	#[test]
	fn ordinal_southeast_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(2, 2);
		let result = cell.get_in_ordinal_direction(&Ordinal::SouthEast, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn ordinal_southeast_invalid() {
		let cell = FieldCell::new(9, 9);
		let result = cell.get_in_ordinal_direction(&Ordinal::SouthEast, 1);
		assert!(result.is_none())
	}

	#[test]
	fn ordinal_southwest_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(0, 2);
		let result = cell.get_in_ordinal_direction(&Ordinal::SouthWest, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn ordinal_southwest_invalid() {
		let cell = FieldCell::new(0, 9);
		let result = cell.get_in_ordinal_direction(&Ordinal::SouthWest, 1);
		assert!(result.is_none())
	}

	#[test]
	fn ordinal_northwest_valid() {
		let cell = FieldCell::new(1, 1);
		let actual = FieldCell::new(0, 0);
		let result = cell.get_in_ordinal_direction(&Ordinal::NorthWest, 1);
		assert_eq!(actual, result.unwrap());
	}

	#[test]
	fn ordinal_northwest_invalid() {
		let cell = FieldCell::new(0, 0);
		let result = cell.get_in_ordinal_direction(&Ordinal::NorthWest, 1);
		assert!(result.is_none())
	}

	// #[test]
	// fn sector_entry_north() {
	// 	let cell = FieldCell::new(4, 4);
	// 	let actual = FieldCell::new(4, 9);
	// 	let result = cell.get_sector_entry_cell(&Ordinal::North);
	// 	assert_eq!(actual, result);
	// }

	// #[test]
	// fn sector_entry_east() {
	// 	let cell = FieldCell::new(4, 4);
	// 	let actual = FieldCell::new(0, 4);
	// 	let result = cell.get_sector_entry_cell(&Ordinal::East);
	// 	assert_eq!(actual, result);
	// }

	// #[test]
	// fn sector_entry_south() {
	// 	let cell = FieldCell::new(4, 4);
	// 	let actual = FieldCell::new(4, 0);
	// 	let result = cell.get_sector_entry_cell(&Ordinal::South);
	// 	assert_eq!(actual, result);
	// }

	// #[test]
	// fn sector_entry_west() {
	// 	let cell = FieldCell::new(4, 4);
	// 	let actual = FieldCell::new(9, 4);
	// 	let result = cell.get_sector_entry_cell(&Ordinal::West);
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
}
