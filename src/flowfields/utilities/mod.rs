//! Useful structures and tools used by the fields
//!

use bevy::prelude::*;

use crate::flowfields::{fields::FieldCell, sectors::SectorID};

/// Defines the dimensions of all field arrays
pub const FIELD_RESOLUTION: usize = 10;

/// Convenience way of accessing the 4 sides of a sector in [crate::prelude::Portals], the 4 sides of a field cell in [crate::prelude::IntegrationField] and the 8 directions
/// of movement in [crate::prelude::flow_field::FlowField]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, PartialEq, Clone, Copy, Reflect, Default, PartialOrd, Eq, Ord, Hash)]
pub enum CompassDir {
	#[default]
	North,
	East,
	South,
	West,
	NorthEast,
	SouthEast,
	SouthWest,
	NorthWest,
	/// Special case, used to indicate a forbidden cell in the [crate::prelude::flow_field::FlowField]
	Zero,
}

impl std::fmt::Display for CompassDir {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CompassDir::North => write!(f, "North"),
			CompassDir::East => write!(f, "East"),
			CompassDir::South => write!(f, "South"),
			CompassDir::West => write!(f, "West"),
			CompassDir::NorthEast => write!(f, "NorthEast"),
			CompassDir::SouthEast => write!(f, "SouthEast"),
			CompassDir::SouthWest => write!(f, "SouthWest"),
			CompassDir::NorthWest => write!(f, "NorthWest"),
			CompassDir::Zero => write!(f, "Zero"),
		}
	}
}

impl CompassDir {
	/// From a [FieldCell] identify the first [FieldCell] in an [CompassDir] direction that lies in a different sector.
	///
	/// The [SectorID] returned by this method is a `delta` [SectorID], i.e it's not a real sector but marks the changes in sector column and row from the sector that the input [FieldCell] resides in
	///
	/// E.g:
	/// ```txt
	/// _____________________
	/// |         |         |
	/// |     --->|x        |
	/// |   A     |    B    |
	/// |         |         |
	/// |_________|_________|
	/// ```
	///
	/// Choosing a [FieldCell] in sector `A` with direction [CompassDir::East] will identify the first [FieldCell] in sector `B` along the boundary, and "steps" of [SectorID] away it is, in this case `B` is located `SectorID(1, 0)` away from `A`
	pub fn get_sector_cell_entry(&self, field_cell: &FieldCell) -> (SectorID, FieldCell) {
		match self {
			CompassDir::North => (
				SectorID::new(0, -1),
				FieldCell::new(field_cell.get_column(), FIELD_RESOLUTION - 1),
			),
			CompassDir::East => (SectorID::new(1, 0), FieldCell::new(0, field_cell.get_row())),
			CompassDir::South => (
				SectorID::new(0, 1),
				FieldCell::new(field_cell.get_column(), 0),
			),
			CompassDir::West => (
				SectorID::new(-1, 0),
				FieldCell::new(FIELD_RESOLUTION - 1, field_cell.get_row()),
			),
			CompassDir::NorthEast => {
				// care: moving diagonally can result in 3 possible sectors,
				// to the north, north-east and east
				//
				//
				let (col, row) = field_cell.get_column_row();
				if col + row == FIELD_RESOLUTION - 1 {
					(
						SectorID::new(1, -1),
						FieldCell::new(0, FIELD_RESOLUTION - 1),
					)
				} else if col + row > FIELD_RESOLUTION - 1 {
					(
						SectorID::new(1, 0),
						FieldCell::new(0, row - (FIELD_RESOLUTION - col)),
					)
				} else {
					(
						SectorID::new(0, -1),
						FieldCell::new(col + 1 + row, FIELD_RESOLUTION - 1),
					)
				}
			}
			CompassDir::SouthEast => {
				// care: moving diagonally can result in 3 possible sectors,
				// to the south, south-east and east
				//
				//
				let (col, row) = field_cell.get_column_row();
				if col == row {
					(SectorID::new(1, 1), FieldCell::new(0, 0))
				} else if col > row {
					(
						SectorID::new(1, 0),
						FieldCell::new(0, row + (FIELD_RESOLUTION - col)),
					)
				} else {
					(
						SectorID::new(0, 1),
						FieldCell::new(col + (FIELD_RESOLUTION - row), 0),
					)
				}
			}
			CompassDir::SouthWest => {
				// care: moving diagonally can result in 3 possible sectors,
				// to the south, south-west and west
				//
				//
				let (col, row) = field_cell.get_column_row();
				if col + row == FIELD_RESOLUTION - 1 {
					(
						SectorID::new(-1, 1),
						FieldCell::new(FIELD_RESOLUTION - 1, 0),
					)
				} else if col + row > FIELD_RESOLUTION - 1 {
					(
						SectorID::new(0, 1),
						FieldCell::new(col - (FIELD_RESOLUTION - row), 0),
					)
				} else {
					(
						SectorID::new(-1, 0),
						FieldCell::new(FIELD_RESOLUTION - 1, col + 1 + row),
					)
				}
			}
			CompassDir::NorthWest => {
				// care: moving diagonally can result in 3 possible sectors,
				// to the north, north-west and west
				//
				//
				let (col, row) = field_cell.get_column_row();
				if col == row {
					(
						SectorID::new(-1, -1),
						FieldCell::new(FIELD_RESOLUTION - 1, FIELD_RESOLUTION - 1),
					)
				} else if col > row {
					(
						SectorID::new(0, -1),
						FieldCell::new(col - 1 - row, FIELD_RESOLUTION - 1),
					)
				} else {
					(
						SectorID::new(-1, 0),
						FieldCell::new(FIELD_RESOLUTION - 1, row - 1 - col),
					)
				}
			}
			// this should never be called, panic instead?
			CompassDir::Zero => (SectorID::new(0, 0), *field_cell),
		}
	}

	/// From a [FieldCell] head in an [CompassDir] a number of `steps` away from the source and determine any [SectorID] delta change and what [FieldCell] the steps arrive at
	///
	/// The [SectorID] returned by this method is a `delta` [SectorID], i.e it's not a real sector but marks the changes in sector column and row from the sector that the input [FieldCell] resides in
	pub fn step_cell_in_direction(
		&self,
		field_cell: &FieldCell,
		steps: usize,
	) -> (SectorID, FieldCell) {
		match self {
			CompassDir::North => {
				let (col, row) = field_cell.get_column_row();
				if steps > row {
					// this will go into a different sector
					// find how far away sector delta is
					let remaining_steps = steps - row;
					let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
					let sector_delta = SectorID::new(0, -1 - delta_sec as i32);
					// find cell
					let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;
					let cell = FieldCell::new(col, FIELD_RESOLUTION - cell_remainder);

					(sector_delta, cell)
				} else {
					(SectorID::new(0, 0), FieldCell::new(col, row - steps))
				}
			}
			CompassDir::East => {
				let (col, row) = field_cell.get_column_row();
				if col + steps > FIELD_RESOLUTION - 1 {
					// goes into a different sector
					// find how far away sector delta is
					let remaining_steps = steps - ((FIELD_RESOLUTION - 1) - col);
					let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
					let sector_delta = SectorID::new(1 + delta_sec as i32, 0);
					// find cell
					let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;
					let cell = FieldCell::new(cell_remainder - 1, row);

					(sector_delta, cell)
				} else {
					(SectorID::new(0, 0), FieldCell::new(col + steps, row))
				}
			}
			CompassDir::South => {
				let (col, row) = field_cell.get_column_row();
				if row + steps > FIELD_RESOLUTION - 1 {
					// goes into a different sector
					// find how far away sector delta is
					let remaining_steps = steps - ((FIELD_RESOLUTION - 1) - row);
					let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
					let sector_delta = SectorID::new(0, 1 + delta_sec as i32);
					// find cell
					let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;
					let cell = FieldCell::new(col, cell_remainder - 1);

					(sector_delta, cell)
				} else {
					(SectorID::new(0, 0), FieldCell::new(col, row + steps))
				}
			}
			CompassDir::West => {
				let (col, row) = field_cell.get_column_row();
				if steps > col {
					// goes into a different sector
					// find how far away sector delta is
					let remaining_steps = steps - col;
					let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
					let sector_delta = SectorID::new(-1 - delta_sec as i32, 0);
					// find cell
					let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;
					let cell = FieldCell::new(FIELD_RESOLUTION - cell_remainder, row);

					(sector_delta, cell)
				} else {
					(SectorID::new(0, 0), FieldCell::new(col - steps, row))
				}
			}
			CompassDir::NorthEast => {
				// care: moving diagonally allows for several different sector deltas
				let (col, row) = field_cell.get_column_row();
				if col + steps < FIELD_RESOLUTION && steps <= row {
					// diag stays in sector
					(
						SectorID::new(0, 0),
						FieldCell::new(col + steps, row - steps),
					)
				} else {
					// sector col could be origin or easterly
					let (delta_sec_col, cell_col): (i32, usize) =
						if col + steps > FIELD_RESOLUTION - 1 {
							let remaining_steps = steps - ((FIELD_RESOLUTION - 1) - col);
							let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
							let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;

							(1 + delta_sec as i32, cell_remainder - 1)
						} else {
							(0, col + steps)
						};
					// sector row could be origin or northerly
					let (delta_sec_row, cell_row): (i32, usize) = if steps > row {
						let remaining_steps = steps - row;
						let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
						let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;

						(-1 - delta_sec as i32, FIELD_RESOLUTION - cell_remainder)
					} else {
						(0, row - steps)
					};

					let sector_delta = SectorID::new(delta_sec_col, delta_sec_row);
					let cell = FieldCell::new(cell_col, cell_row);

					(sector_delta, cell)
				}
			}
			CompassDir::SouthEast => {
				// care: moving diagonally allows for several different sector deltas
				let (col, row) = field_cell.get_column_row();
				if col + steps < FIELD_RESOLUTION && row + steps < FIELD_RESOLUTION {
					// diag stays in sector
					(
						SectorID::new(0, 0),
						FieldCell::new(col + steps, row + steps),
					)
				} else {
					// sector col could be origin or easterly
					let (delta_sec_col, cell_col): (i32, usize) =
						if col + steps > FIELD_RESOLUTION - 1 {
							let remaining_steps = steps - ((FIELD_RESOLUTION - 1) - col);
							let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
							let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;

							(1 + delta_sec as i32, cell_remainder - 1)
						} else {
							(0, col + steps)
						};
					// sector row could be origin or southerly
					let (delta_sec_row, cell_row): (i32, usize) =
						if row + steps > FIELD_RESOLUTION - 1 {
							let remaining_steps = steps - ((FIELD_RESOLUTION - 1) - row);
							let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
							let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;

							(1 + delta_sec as i32, cell_remainder - 1)
						} else {
							(0, row + steps)
						};

					let sector_delta = SectorID::new(delta_sec_col, delta_sec_row);
					let cell = FieldCell::new(cell_col, cell_row);

					(sector_delta, cell)
				}
			}
			CompassDir::SouthWest => {
				// care: moving diagonally allows for several different sector deltas
				let (col, row) = field_cell.get_column_row();
				if steps <= col && row + steps < FIELD_RESOLUTION {
					// diag stays in sector
					(
						SectorID::new(0, 0),
						FieldCell::new(col - steps, row + steps),
					)
				} else {
					// sector col could be origin or westerly
					let (delta_sec_col, cell_col): (i32, usize) = if steps > col {
						let remaining_steps = steps - col;
						let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
						let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;

						(-1 - delta_sec as i32, FIELD_RESOLUTION - cell_remainder)
					} else {
						(0, col - steps)
					};
					// sector row could be origin or southerly
					let (delta_sec_row, cell_row): (i32, usize) =
						if row + steps > FIELD_RESOLUTION - 1 {
							let remaining_steps = steps - ((FIELD_RESOLUTION - 1) - row);
							let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
							let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;

							(1 + delta_sec as i32, cell_remainder - 1)
						} else {
							(0, row + steps)
						};

					let sector_delta = SectorID::new(delta_sec_col, delta_sec_row);
					let cell = FieldCell::new(cell_col, cell_row);

					(sector_delta, cell)
				}
			}
			CompassDir::NorthWest => {
				// care: moving diagonally allows for several different sector deltas
				let (col, row) = field_cell.get_column_row();
				if steps <= col && steps <= row {
					// diag stays in sector
					(
						SectorID::new(0, 0),
						FieldCell::new(col - steps, row - steps),
					)
				} else {
					// sector col could be origin or westerly
					let (delta_sec_col, cell_col): (i32, usize) = if steps > col {
						let remaining_steps = steps - col;
						let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
						let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;

						(-1 - delta_sec as i32, FIELD_RESOLUTION - cell_remainder)
					} else {
						(0, col - steps)
					};
					// sector row could be origin or northerly
					let (delta_sec_row, cell_row): (i32, usize) = if steps > row {
						let remaining_steps = steps - row;
						let delta_sec = remaining_steps / (FIELD_RESOLUTION + 1);
						let cell_remainder = remaining_steps - FIELD_RESOLUTION * delta_sec;

						(-1 - delta_sec as i32, FIELD_RESOLUTION - cell_remainder)
					} else {
						(0, row - steps)
					};

					let sector_delta = SectorID::new(delta_sec_col, delta_sec_row);
					let cell = FieldCell::new(cell_col, cell_row);

					(sector_delta, cell)
				}
			}
			CompassDir::Zero => panic!("This should never be called"),
		}
	}

	/// Based on a field cells `(column, row)` position find its neighbours based on FIELD_RESOLUTION limits (up to 4)
	pub fn get_orthogonal_cell_neighbours(cell_id: FieldCell) -> Vec<FieldCell> {
		let row = cell_id.get_row();
		let column = cell_id.get_column();
		// 64 out of 100 field cells have 4 neighbours so this fast returns
		// the neighbours - based on profiling
		if row > 0 && column > 0 && row < FIELD_RESOLUTION - 1 && column < FIELD_RESOLUTION - 1 {
			return vec![
				FieldCell::new(column, row - 1),
				FieldCell::new(column + 1, row),
				FieldCell::new(column, row + 1),
				FieldCell::new(column - 1, row),
			];
		}
		let mut neighbours = Vec::new();
		if row > 0 {
			neighbours.push(FieldCell::new(column, row - 1)); // northern cell coords
		}
		if column < FIELD_RESOLUTION - 1 {
			neighbours.push(FieldCell::new(column + 1, row)); // eastern cell coords
		}
		if row < FIELD_RESOLUTION - 1 {
			neighbours.push(FieldCell::new(column, row + 1)); // southern cell coords
		}
		if column > 0 {
			neighbours.push(FieldCell::new(column - 1, row)); // western cell coords
		}
		neighbours
	}
	/// Based on a field cells `(column, row)` position find its diagonal neighbours based on FIELD_RESOLUTION limits (up to 4)
	pub fn get_diagonal_cell_neighbours(cell_id: FieldCell) -> Vec<FieldCell> {
		let mut neighbours = Vec::new();
		if cell_id.get_row() > 0 {
			if cell_id.get_column() < FIELD_RESOLUTION - 1 {
				neighbours.push(FieldCell::new(
					cell_id.get_column() + 1,
					cell_id.get_row() - 1,
				)); // north-east cell
			}
			if cell_id.get_column() > 0 {
				neighbours.push(FieldCell::new(
					cell_id.get_column() - 1,
					cell_id.get_row() - 1,
				)); // north-west cell
			}
		}
		if cell_id.get_row() < FIELD_RESOLUTION - 1 {
			if cell_id.get_column() < FIELD_RESOLUTION - 1 {
				neighbours.push(FieldCell::new(
					cell_id.get_column() + 1,
					cell_id.get_row() + 1,
				)); // south-east cell
			}
			if cell_id.get_column() > 0 {
				neighbours.push(FieldCell::new(
					cell_id.get_column() - 1,
					cell_id.get_row() + 1,
				)); // south-west cell
			}
		}
		neighbours
	}
	/// Based on a field cells `(column, row)` and an [CompassDir] direction find the neighbouring [FieldCell] if one exists
	pub fn get_cell_neighbour(cell_id: FieldCell, compass_dir: CompassDir) -> Option<FieldCell> {
		match compass_dir {
			CompassDir::North => {
				if cell_id.get_row() > 0 {
					Some(FieldCell::new(cell_id.get_column(), cell_id.get_row() - 1))
				} else {
					None
				}
			}
			CompassDir::East => {
				if cell_id.get_column() < FIELD_RESOLUTION - 1 {
					Some(FieldCell::new(cell_id.get_column() + 1, cell_id.get_row()))
				} else {
					None
				}
			}
			CompassDir::South => {
				if cell_id.get_row() < FIELD_RESOLUTION - 1 {
					Some(FieldCell::new(cell_id.get_column(), cell_id.get_row() + 1))
				} else {
					None
				}
			}
			CompassDir::West => {
				if cell_id.get_column() > 0 {
					Some(FieldCell::new(cell_id.get_column() - 1, cell_id.get_row()))
				} else {
					None
				}
			}
			CompassDir::NorthEast => {
				if cell_id.get_row() > 0 && cell_id.get_column() < FIELD_RESOLUTION - 1 {
					Some(FieldCell::new(
						cell_id.get_column() + 1,
						cell_id.get_row() - 1,
					))
				} else {
					None
				}
			}
			CompassDir::SouthEast => {
				if cell_id.get_row() < FIELD_RESOLUTION - 1
					&& cell_id.get_column() < FIELD_RESOLUTION - 1
				{
					Some(FieldCell::new(
						cell_id.get_column() + 1,
						cell_id.get_row() + 1,
					))
				} else {
					None
				}
			}
			CompassDir::SouthWest => {
				if cell_id.get_row() < FIELD_RESOLUTION - 1 && cell_id.get_column() > 0 {
					Some(FieldCell::new(
						cell_id.get_column() - 1,
						cell_id.get_row() + 1,
					))
				} else {
					None
				}
			}
			CompassDir::NorthWest => {
				if cell_id.get_row() > 0 && cell_id.get_column() > 0 {
					Some(FieldCell::new(
						cell_id.get_column() - 1,
						cell_id.get_row() - 1,
					))
				} else {
					None
				}
			}
			CompassDir::Zero => None,
		}
	}
	/// Based on a field cells `(column, row)` position find all possible neighbours including diagonal directions
	pub fn get_all_cell_neighbours(cell_id: FieldCell) -> Vec<FieldCell> {
		let mut neighbours = CompassDir::get_orthogonal_cell_neighbours(cell_id);
		let mut diagonals = CompassDir::get_diagonal_cell_neighbours(cell_id);
		neighbours.append(&mut diagonals);
		neighbours
	}
	/// Based on a field cells `(column, row)` position find all possible neighbours including diagonal directions and the [CompassDir] they are found in
	pub fn get_all_cell_neighbours_with_compass_dir(
		cell_id: FieldCell,
	) -> Vec<(CompassDir, FieldCell)> {
		let mut neighbours = Vec::new();
		if cell_id.get_row() > 0 {
			neighbours.push((
				CompassDir::North,
				FieldCell::new(cell_id.get_column(), cell_id.get_row() - 1),
			)); // northern cell coords
		}
		if cell_id.get_column() < FIELD_RESOLUTION - 1 {
			neighbours.push((
				CompassDir::East,
				FieldCell::new(cell_id.get_column() + 1, cell_id.get_row()),
			)); // eastern cell coords
		}
		if cell_id.get_row() < FIELD_RESOLUTION - 1 {
			neighbours.push((
				CompassDir::South,
				FieldCell::new(cell_id.get_column(), cell_id.get_row() + 1),
			)); // southern cell coords
		}
		if cell_id.get_column() > 0 {
			neighbours.push((
				CompassDir::West,
				FieldCell::new(cell_id.get_column() - 1, cell_id.get_row()),
			)); // western cell coords
		}
		if cell_id.get_row() > 0 && cell_id.get_column() < FIELD_RESOLUTION - 1 {
			neighbours.push((
				CompassDir::NorthEast,
				FieldCell::new(cell_id.get_column() + 1, cell_id.get_row() - 1),
			)); // north-east cell
		}
		if cell_id.get_row() < FIELD_RESOLUTION - 1 && cell_id.get_column() < FIELD_RESOLUTION - 1 {
			neighbours.push((
				CompassDir::SouthEast,
				FieldCell::new(cell_id.get_column() + 1, cell_id.get_row() + 1),
			)); // south-east cell
		}
		if cell_id.get_row() < FIELD_RESOLUTION - 1 && cell_id.get_column() > 0 {
			neighbours.push((
				CompassDir::SouthWest,
				FieldCell::new(cell_id.get_column() - 1, cell_id.get_row() + 1),
			)); // south-west cell
		}
		if cell_id.get_row() > 0 && cell_id.get_column() > 0 {
			neighbours.push((
				CompassDir::NorthWest,
				FieldCell::new(cell_id.get_column() - 1, cell_id.get_row() - 1),
			)); // north-west cell
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find its neighbours based on map size limits (up to 4)
	/// ```txt
	/// top left                     // top right
	/// has 2 valid neighbours      // has two valid neighbours
	/// ___________                 // ___________
	/// | x       |                 // |       x |
	/// |x        |                 // |        x|
	/// |         |                 // |         |
	/// |         |                 // |         |
	/// |_________|                 // |_________|
	/// bottom right                // bottom left sector
	/// has two valid neighbours    // has two valid neighbours
	/// ___________                 // ___________
	/// |         |                 // |         |
	/// |         |                 // |         |
	/// |         |                 // |         |
	/// |        x|                 // |x        |
	/// |_______x_|                 // |_x_______|
	/// northern row minus          // eastern column minus
	/// corners have three          // corners have three
	/// valid neighbours            // valid neighbours
	/// ___________                 // ___________
	/// |x       x|                 // |        x|
	/// |  xxxxx  |                 // |       x |
	/// |         |                 // |       x |
	/// |         |                 // |       x |
	/// |_________|                 // |________x|
	/// southern row minus          // western column minus
	/// corners have three          // corners have three
	/// valid neighbours            // valid neighbours
	/// ___________                 // ___________
	/// |         |                 // |x        |
	/// |         |                 // | x       |
	/// |         |                 // | x       |
	/// | xxxxxxx |                 // | x       |
	/// |x       x|                 // |x________|
	/// all other sectors not along an edge of the map have four valid sectors for portals
	/// ___________
	/// |         |
	/// |    x    |
	/// |   x x   |
	/// |    x    |
	/// |_________|
	/// ```
	pub fn get_sector_neighbours(
		sector_id: &SectorID,
		map_length: f32,
		map_depth: f32,
		world_unit_size: f32,
	) -> Vec<SectorID> {
		let mut neighbours = Vec::new();
		let sector_column_limit =
			(map_length / (world_unit_size * FIELD_RESOLUTION as f32)) as i32 - 1;
		let sector_row_limit = (map_depth / (world_unit_size * FIELD_RESOLUTION as f32)) as i32 - 1;
		if sector_id.get_row() > 0 {
			neighbours.push(SectorID::new(
				sector_id.get_column(),
				sector_id.get_row() - 1,
			)); // northern sector coords
		}
		if sector_id.get_column() < sector_column_limit {
			neighbours.push(SectorID::new(
				sector_id.get_column() + 1,
				sector_id.get_row(),
			)); // eastern sector coords
		}
		if sector_id.get_row() < sector_row_limit {
			neighbours.push(SectorID::new(
				sector_id.get_column(),
				sector_id.get_row() + 1,
			)); // southern sector coords
		}
		if sector_id.get_column() > 0 {
			neighbours.push(SectorID::new(
				sector_id.get_column() - 1,
				sector_id.get_row(),
			)); // western sector coords
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find the [CompassDir] directions for its boundaries that can support [crate::prelude::Portals]
	pub fn get_sector_portal_compass_dirs(
		sector_id: &SectorID,
		map_length: f32,
		map_depth: f32,
		world_unit_size: f32,
	) -> Vec<CompassDir> {
		let mut neighbours = Vec::new();
		let sector_column_limit =
			(map_length / (world_unit_size * FIELD_RESOLUTION as f32)) as i32 - 1;
		let sector_row_limit = (map_depth / (world_unit_size * FIELD_RESOLUTION as f32)) as i32 - 1;
		if sector_id.get_row() > 0 {
			neighbours.push(CompassDir::North); // northern sector coords
		}
		if sector_id.get_column() < sector_column_limit {
			neighbours.push(CompassDir::East); // eastern sector coords
		}
		if sector_id.get_row() < sector_row_limit {
			neighbours.push(CompassDir::South); // southern sector coords
		}
		if sector_id.get_column() > 0 {
			neighbours.push(CompassDir::West); // western sector coords
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find its neighbours based on map size limits (up to 4) and include the [CompassDir] direction in the result
	/// ```txt
	///top left                      top right
	/// has 2 valid neighbours       has two valid neighbours
	/// ___________                  ___________
	/// | x       |                  |       x |
	/// |x        |                  |        x|
	/// |         |                  |         |
	/// |         |                  |         |
	/// |_________|                  |_________|
	/// bottom right                 bottom left sector
	/// has two valid neighbours     has two valid neighbours
	/// ___________                  ___________
	/// |         |                  |         |
	/// |         |                  |         |
	/// |         |                  |         |
	/// |        x|                  |x        |
	/// |_______x_|                  |_x_______|
	/// northern row minus           eastern column minus
	/// corners have three           corners have three
	/// valid neighbours             valid neighbours
	/// ___________                  ___________
	/// |x       x|                  |        x|
	/// |  xxxxx  |                  |       x |
	/// |         |                  |       x |
	/// |         |                  |       x |
	/// |_________|                  |________x|
	/// southern row minus           western column minus
	/// corners have three           corners have three
	/// valid neighbours             valid neighbours
	/// ___________                  ___________
	/// |         |                  |x        |
	/// |         |                  | x       |
	/// |         |                  | x       |
	/// | xxxxxxx |                  | x       |
	/// |x       x|                  |x________|
	/// all other sectors not along an edge of the map have four valid sectors for portals
	/// ___________
	/// |         |
	/// |    x    |
	/// |   x x   |
	/// |    x    |
	/// |_________|
	/// ```
	pub fn get_sector_neighbours_with_compass_dir(
		sector_id: &SectorID,
		map_length: f32,
		map_depth: f32,
		world_unit_size: f32,
	) -> Vec<(CompassDir, SectorID)> {
		let mut neighbours = Vec::new();
		let sector_column_limit =
			(map_length / (world_unit_size * FIELD_RESOLUTION as f32)) as i32 - 1;
		let sector_row_limit = (map_depth / (world_unit_size * FIELD_RESOLUTION as f32)) as i32 - 1;
		if sector_id.get_row() > 0 {
			neighbours.push((
				CompassDir::North,
				SectorID::new(sector_id.get_column(), sector_id.get_row() - 1),
			)); // northern sector coords
		}
		if sector_id.get_column() < sector_column_limit {
			neighbours.push((
				CompassDir::East,
				SectorID::new(sector_id.get_column() + 1, sector_id.get_row()),
			)); // eastern sector coords
		}
		if sector_id.get_row() < sector_row_limit {
			neighbours.push((
				CompassDir::South,
				SectorID::new(sector_id.get_column(), sector_id.get_row() + 1),
			)); // southern sector coords
		}
		if sector_id.get_column() > 0 {
			neighbours.push((
				CompassDir::West,
				SectorID::new(sector_id.get_column() - 1, sector_id.get_row()),
			)); // western sector coords
		}
		neighbours
	}
	/// Returns the opposite [CompassDir] of the current
	pub fn inverse(&self) -> CompassDir {
		match self {
			CompassDir::North => CompassDir::South,
			CompassDir::East => CompassDir::West,
			CompassDir::South => CompassDir::North,
			CompassDir::West => CompassDir::East,
			CompassDir::NorthEast => CompassDir::SouthWest,
			CompassDir::SouthEast => CompassDir::NorthWest,
			CompassDir::SouthWest => CompassDir::NorthEast,
			CompassDir::NorthWest => CompassDir::SouthEast,
			CompassDir::Zero => CompassDir::Zero,
		}
	}
	/// For two cells next to each other it can be useful to find the [CompassDir] point from the `source` to the `target`
	pub fn cell_to_cell_direction(target: FieldCell, source: FieldCell) -> Self {
		let i32_target = (target.get_column() as i32, target.get_row() as i32);
		let i32_source = (source.get_column() as i32, source.get_row() as i32);

		let direction = (i32_target.0 - i32_source.0, i32_target.1 - i32_source.1);
		match direction {
			(0, -1) => CompassDir::North,
			(1, -1) => CompassDir::NorthEast,
			(1, 0) => CompassDir::East,
			(1, 1) => CompassDir::SouthEast,
			(0, 1) => CompassDir::South,
			(-1, 1) => CompassDir::SouthWest,
			(-1, 0) => CompassDir::West,
			(-1, -1) => CompassDir::NorthWest,
			_ => panic!(
				"Cell {:?} is not orthogonally or diagonally adjacent to {:?}",
				target, source
			),
		}
	}
	/// For two sectors next to each other it can be useful to find the [CompassDir] from the `source` to the `target`. If they are not adjacent None is returned
	pub fn sector_to_sector_direction(target: &SectorID, source: &SectorID) -> Option<Self> {
		let i32_target = (target.get_column(), target.get_row());
		let i32_source = (source.get_column(), source.get_row());

		let direction = (i32_target.0 - i32_source.0, i32_target.1 - i32_source.1);
		match direction {
			(0, -1) => Some(CompassDir::North),
			(1, 0) => Some(CompassDir::East),
			(0, 1) => Some(CompassDir::South),
			(-1, 0) => Some(CompassDir::West),
			_ => {
				error!(
					"Sector {:?} is not orthogonally adjacent to {:?}",
					target, source
				);
				None
			}
		}
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sector_entry_ne1() {
		// pure diag into NE sector
		let source_cell = FieldCell::new(7, 2);
		let compass_dir = CompassDir::NorthEast;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(1, -1);
		let actual_cell = FieldCell::new(0, 9);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_ne2() {
		// into N sector
		let source_cell = FieldCell::new(2, 2);
		let compass_dir = CompassDir::NorthEast;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(0, -1);
		let actual_cell = FieldCell::new(5, 9);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_ne3() {
		// into E sector
		let source_cell = FieldCell::new(6, 8);
		let compass_dir = CompassDir::NorthEast;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(1, 0);
		let actual_cell = FieldCell::new(0, 4);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_se1() {
		// into SE sector
		let source_cell = FieldCell::new(5, 5);
		let compass_dir = CompassDir::SouthEast;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(1, 1);
		let actual_cell = FieldCell::new(0, 0);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_se2() {
		// into E sector
		let source_cell = FieldCell::new(8, 4);
		let compass_dir = CompassDir::SouthEast;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(1, 0);
		let actual_cell = FieldCell::new(0, 6);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_se3() {
		// into S sector
		let source_cell = FieldCell::new(1, 6);
		let compass_dir = CompassDir::SouthEast;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(0, 1);
		let actual_cell = FieldCell::new(5, 0);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_sw1() {
		// into SW sector
		let source_cell = FieldCell::new(5, 4);
		let compass_dir = CompassDir::SouthWest;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(-1, 1);
		let actual_cell = FieldCell::new(9, 0);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_sw2() {
		// into S sector
		let source_cell = FieldCell::new(5, 7);
		let compass_dir = CompassDir::SouthWest;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(0, 1);
		let actual_cell = FieldCell::new(2, 0);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_sw3() {
		// into W sector
		let source_cell = FieldCell::new(2, 3);
		let compass_dir = CompassDir::SouthWest;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(-1, 0);
		let actual_cell = FieldCell::new(9, 6);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_nw1() {
		// into NW sector
		let source_cell = FieldCell::new(3, 3);
		let compass_dir = CompassDir::NorthWest;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(-1, -1);
		let actual_cell = FieldCell::new(9, 9);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_nw2() {
		// into N sector
		let source_cell = FieldCell::new(5, 1);
		let compass_dir = CompassDir::NorthWest;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(0, -1);
		let actual_cell = FieldCell::new(3, 9);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}
	#[test]
	fn sector_entry_nw3() {
		// into W sector
		let source_cell = FieldCell::new(3, 6);
		let compass_dir = CompassDir::NorthWest;

		let (result_delta, result_cell) = compass_dir.get_sector_cell_entry(&source_cell);
		let actual_delta = SectorID::new(-1, 0);
		let actual_cell = FieldCell::new(9, 2);

		assert_eq!(actual_delta, result_delta);
		assert_eq!(actual_cell, result_cell);
	}

	#[test]
	fn step_cell_n1() {
		// step cell in N dir
		let source_cell = FieldCell::new(3, 2);
		let compass_dir = CompassDir::North;
		let steps = 1;

		let actual_delta = SectorID::new(0, 0);
		let actual_cell = FieldCell::new(3, 1);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_n2() {
		// step cell in N dir
		let source_cell = FieldCell::new(3, 2);
		let compass_dir = CompassDir::North;
		let steps = 5;

		let actual_delta = SectorID::new(0, -1);
		let actual_cell = FieldCell::new(3, 7);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_n3() {
		// step cell in N dir
		let source_cell = FieldCell::new(3, 2);
		let compass_dir = CompassDir::North;
		let steps = 15;

		let actual_delta = SectorID::new(0, -2);
		let actual_cell = FieldCell::new(3, 7);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_e1() {
		// step cell in E dir
		let source_cell = FieldCell::new(3, 2);
		let compass_dir = CompassDir::East;
		let steps = 1;

		let actual_delta = SectorID::new(0, 0);
		let actual_cell = FieldCell::new(4, 2);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_e2() {
		// step cell in E dir
		let source_cell = FieldCell::new(3, 2);
		let compass_dir = CompassDir::East;
		let steps = 7;

		let actual_delta = SectorID::new(1, 0);
		let actual_cell = FieldCell::new(0, 2);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_e3() {
		// step cell in E dir
		let source_cell = FieldCell::new(3, 2);
		let compass_dir = CompassDir::East;
		let steps = 21;

		let actual_delta = SectorID::new(2, 0);
		let actual_cell = FieldCell::new(4, 2);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_s1() {
		// step cell in S dir
		let source_cell = FieldCell::new(3, 6);
		let compass_dir = CompassDir::South;
		let steps = 3;

		let actual_delta = SectorID::new(0, 0);
		let actual_cell = FieldCell::new(3, 9);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_s2() {
		// step cell in S dir
		let source_cell = FieldCell::new(3, 6);
		let compass_dir = CompassDir::South;
		let steps = 6;

		let actual_delta = SectorID::new(0, 1);
		let actual_cell = FieldCell::new(3, 2);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_s3() {
		// step cell in S dir
		let source_cell = FieldCell::new(3, 6);
		let compass_dir = CompassDir::South;
		let steps = 17;

		let actual_delta = SectorID::new(0, 2);
		let actual_cell = FieldCell::new(3, 3);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_w1() {
		// step cell in W dir
		let source_cell = FieldCell::new(3, 8);
		let compass_dir = CompassDir::West;
		let steps = 2;

		let actual_delta = SectorID::new(0, 0);
		let actual_cell = FieldCell::new(1, 8);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_w2() {
		// step cell in W dir
		let source_cell = FieldCell::new(3, 8);
		let compass_dir = CompassDir::West;
		let steps = 5;

		let actual_delta = SectorID::new(-1, 0);
		let actual_cell = FieldCell::new(8, 8);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_w3() {
		// step cell in W dir
		let source_cell = FieldCell::new(3, 8);
		let compass_dir = CompassDir::West;
		let steps = 18;

		let actual_delta = SectorID::new(-2, 0);
		let actual_cell = FieldCell::new(5, 8);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_ne1() {
		// step cell in NE dir
		let source_cell = FieldCell::new(9, 0);
		let compass_dir = CompassDir::NorthEast;
		let steps = 1;

		let actual_delta = SectorID::new(1, -1);
		let actual_cell = FieldCell::new(0, 9);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_ne2() {
		// step cell in NE dir
		let source_cell = FieldCell::new(6, 1);
		let compass_dir = CompassDir::NorthEast;
		let steps = 3;

		let actual_delta = SectorID::new(0, -1);
		let actual_cell = FieldCell::new(9, 8);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_ne3() {
		// step cell in NE dir
		let source_cell = FieldCell::new(8, 5);
		let compass_dir = CompassDir::NorthEast;
		let steps = 5;

		let actual_delta = SectorID::new(1, 0);
		let actual_cell = FieldCell::new(3, 0);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_ne4() {
		// step cell in NE dir
		let source_cell = FieldCell::new(8, 0);
		let compass_dir = CompassDir::NorthEast;
		let steps = 7;

		let actual_delta = SectorID::new(1, -1);
		let actual_cell = FieldCell::new(5, 3);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_se1() {
		// step cell in SE dir
		let source_cell = FieldCell::new(9, 9);
		let compass_dir = CompassDir::SouthEast;
		let steps = 1;

		let actual_delta = SectorID::new(1, 1);
		let actual_cell = FieldCell::new(0, 0);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_se2() {
		// step cell in SE dir
		let source_cell = FieldCell::new(8, 3);
		let compass_dir = CompassDir::SouthEast;
		let steps = 4;

		let actual_delta = SectorID::new(1, 0);
		let actual_cell = FieldCell::new(2, 7);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_se3() {
		// step cell in SE dir
		let source_cell = FieldCell::new(3, 8);
		let compass_dir = CompassDir::SouthEast;
		let steps = 3;

		let actual_delta = SectorID::new(0, 1);
		let actual_cell = FieldCell::new(6, 1);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_se4() {
		// step cell in SE dir
		let source_cell = FieldCell::new(4, 8);
		let compass_dir = CompassDir::SouthEast;
		let steps = 6;

		let actual_delta = SectorID::new(1, 1);
		let actual_cell = FieldCell::new(0, 4);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_sw1() {
		// step cell in SW dir
		let source_cell = FieldCell::new(0, 9);
		let compass_dir = CompassDir::SouthWest;
		let steps = 1;

		let actual_delta = SectorID::new(-1, 1);
		let actual_cell = FieldCell::new(9, 0);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_sw2() {
		// step cell in SW dir
		let source_cell = FieldCell::new(4, 8);
		let compass_dir = CompassDir::SouthWest;
		let steps = 3;

		let actual_delta = SectorID::new(0, 1);
		let actual_cell = FieldCell::new(1, 1);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_sw3() {
		// step cell in SW dir
		let source_cell = FieldCell::new(1, 4);
		let compass_dir = CompassDir::SouthWest;
		let steps = 5;

		let actual_delta = SectorID::new(-1, 0);
		let actual_cell = FieldCell::new(6, 9);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_sw4() {
		// step cell in SW dir
		let source_cell = FieldCell::new(3, 8);
		let compass_dir = CompassDir::SouthWest;
		let steps = 7;

		let actual_delta = SectorID::new(-1, 1);
		let actual_cell = FieldCell::new(6, 5);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_nw1() {
		// step cell in NW dir
		let source_cell = FieldCell::new(0, 0);
		let compass_dir = CompassDir::NorthWest;
		let steps = 1;

		let actual_delta = SectorID::new(-1, -1);
		let actual_cell = FieldCell::new(9, 9);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_nw2() {
		// step cell in NW dir
		let source_cell = FieldCell::new(6, 2);
		let compass_dir = CompassDir::NorthWest;
		let steps = 3;

		let actual_delta = SectorID::new(0, -1);
		let actual_cell = FieldCell::new(3, 9);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_nw3() {
		// step cell in NW dir
		let source_cell = FieldCell::new(2, 6);
		let compass_dir = CompassDir::NorthWest;
		let steps = 3;

		let actual_delta = SectorID::new(-1, 0);
		let actual_cell = FieldCell::new(9, 3);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}
	#[test]
	fn step_cell_nw4() {
		// step cell in NW dir
		let source_cell = FieldCell::new(3, 2);
		let compass_dir = CompassDir::NorthWest;
		let steps = 7;

		let actual_delta = SectorID::new(-1, -1);
		let actual_cell = FieldCell::new(6, 5);
		let result = compass_dir.step_cell_in_direction(&source_cell, steps);
		assert_eq!((actual_delta, actual_cell), result);
	}

	#[test]
	fn compass_dir_field_cell_neighbours() {
		let cell_id = FieldCell::new(0, 0);
		let result = CompassDir::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![FieldCell::new(1, 0), FieldCell::new(0, 1)];
		assert_eq!(actual, result);
	}
	#[test]
	fn compass_dir_field_cell_neighbours2() {
		let cell_id = FieldCell::new(9, 9);
		let result = CompassDir::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![FieldCell::new(9, 8), FieldCell::new(8, 9)];
		assert_eq!(actual, result);
	}
	#[test]
	fn compass_dir_field_cell_neighbours3() {
		let cell_id = FieldCell::new(4, 4);
		let result = CompassDir::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![
			FieldCell::new(4, 3),
			FieldCell::new(5, 4),
			FieldCell::new(4, 5),
			FieldCell::new(3, 4),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn compass_dir_field_cell_neighbours4() {
		let cell_id = FieldCell::new(5, 0);
		let result = CompassDir::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![
			FieldCell::new(6, 0),
			FieldCell::new(5, 1),
			FieldCell::new(4, 0),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn compass_dir_sector_neighbours() {
		let sector_id = SectorID::new(0, 0);
		let map_x_dimension = 300.0;
		let map_z_dimension = 550.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_neighbours(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![SectorID::new(1, 0), SectorID::new(0, 1)];
		assert_eq!(actual, result);
	}
	#[test]
	fn compass_dir_sector_neighbours2() {
		let sector_id = SectorID::new(29, 54);
		let map_x_dimension = 300.0;
		let map_z_dimension = 550.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_neighbours(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![SectorID::new(29, 53), SectorID::new(28, 54)];
		assert_eq!(actual, result);
	}
	#[test]
	fn compass_dir_sector_neighbours3() {
		let sector_id = SectorID::new(14, 31);
		let map_x_dimension = 300.0;
		let map_z_dimension = 550.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_neighbours(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![
			SectorID::new(14, 30),
			SectorID::new(15, 31),
			SectorID::new(14, 32),
			SectorID::new(13, 31),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn compass_dir_sector_neighbours4() {
		let sector_id = SectorID::new(0, 13);
		let map_x_dimension = 300.0;
		let map_z_dimension = 550.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_neighbours(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![
			SectorID::new(0, 12),
			SectorID::new(1, 13),
			SectorID::new(0, 14),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_northern_compass_dirs() {
		let sector_id = SectorID::new(3, 0);
		let map_x_dimension = 200.0;
		let map_z_dimension = 200.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_portal_compass_dirs(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![CompassDir::East, CompassDir::South, CompassDir::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_eastern_compass_dirs() {
		let sector_id = SectorID::new(19, 5);
		let map_x_dimension = 200.0;
		let map_z_dimension = 200.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_portal_compass_dirs(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![CompassDir::North, CompassDir::South, CompassDir::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_southern_compass_dirs() {
		let sector_id = SectorID::new(4, 19);
		let map_x_dimension = 200.0;
		let map_z_dimension = 200.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_portal_compass_dirs(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![CompassDir::North, CompassDir::East, CompassDir::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_western_compass_dirs() {
		let sector_id = SectorID::new(0, 5);
		let map_x_dimension = 200.0;
		let map_z_dimension = 200.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_portal_compass_dirs(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![CompassDir::North, CompassDir::East, CompassDir::South];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_centre_compass_dirs() {
		let sector_id = SectorID::new(4, 5);
		let map_x_dimension = 200.0;
		let map_z_dimension = 200.0;
		let world_unit_size = 1.0;
		let result = CompassDir::get_sector_portal_compass_dirs(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			world_unit_size,
		);
		let actual = vec![
			CompassDir::North,
			CompassDir::East,
			CompassDir::South,
			CompassDir::West,
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north() {
		let target = FieldCell::new(6, 2);
		let source = FieldCell::new(6, 3);
		let result = CompassDir::cell_to_cell_direction(target, source);
		let actual = CompassDir::North;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north_east() {
		let target = FieldCell::new(7, 2);
		let source = FieldCell::new(6, 3);
		let result = CompassDir::cell_to_cell_direction(target, source);
		let actual = CompassDir::NorthEast;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_east() {
		let target = FieldCell::new(6, 7);
		let source = FieldCell::new(5, 7);
		let result = CompassDir::cell_to_cell_direction(target, source);
		let actual = CompassDir::East;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south_east() {
		let target = FieldCell::new(5, 5);
		let source = FieldCell::new(4, 4);
		let result = CompassDir::cell_to_cell_direction(target, source);
		let actual = CompassDir::SouthEast;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south() {
		let target = FieldCell::new(3, 1);
		let source = FieldCell::new(3, 0);
		let result = CompassDir::cell_to_cell_direction(target, source);
		let actual = CompassDir::South;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south_west() {
		let target = FieldCell::new(6, 9);
		let source = FieldCell::new(7, 8);
		let result = CompassDir::cell_to_cell_direction(target, source);
		let actual = CompassDir::SouthWest;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_west() {
		let target = FieldCell::new(5, 7);
		let source = FieldCell::new(6, 7);
		let result = CompassDir::cell_to_cell_direction(target, source);
		let actual = CompassDir::West;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north_west() {
		let target = FieldCell::new(0, 0);
		let source = FieldCell::new(1, 1);
		let result = CompassDir::cell_to_cell_direction(target, source);
		let actual = CompassDir::NorthWest;
		assert_eq!(actual, result);
	}
	#[test]
	fn neighbours_with_compass_dir1() {
		let field = FieldCell::new(3, 4);
		let result = CompassDir::get_all_cell_neighbours_with_compass_dir(field);
		let actual = vec![
			(CompassDir::North, FieldCell::new(3, 3)),
			(CompassDir::East, FieldCell::new(4, 4)),
			(CompassDir::South, FieldCell::new(3, 5)),
			(CompassDir::West, FieldCell::new(2, 4)),
			(CompassDir::NorthEast, FieldCell::new(4, 3)),
			(CompassDir::SouthEast, FieldCell::new(4, 5)),
			(CompassDir::SouthWest, FieldCell::new(2, 5)),
			(CompassDir::NorthWest, FieldCell::new(2, 3)),
		];
		assert_eq!(actual, result)
	}
	#[test]
	fn neighbours_with_compass_dir2() {
		let field = FieldCell::new(0, 0);
		let result = CompassDir::get_all_cell_neighbours_with_compass_dir(field);
		let actual = vec![
			(CompassDir::East, FieldCell::new(1, 0)),
			(CompassDir::South, FieldCell::new(0, 1)),
			(CompassDir::SouthEast, FieldCell::new(1, 1)),
		];
		assert_eq!(actual, result)
	}
	#[test]
	fn sector_sector() {
		let source = SectorID::new(1, 1);

		let target = SectorID::new(1, 0);
		let actual = CompassDir::North;
		assert_eq!(
			actual,
			CompassDir::sector_to_sector_direction(&target, &source).unwrap()
		);

		let target = SectorID::new(2, 1);
		let actual = CompassDir::East;
		assert_eq!(
			actual,
			CompassDir::sector_to_sector_direction(&target, &source).unwrap()
		);

		let target = SectorID::new(1, 2);
		let actual = CompassDir::South;
		assert_eq!(
			actual,
			CompassDir::sector_to_sector_direction(&target, &source).unwrap()
		);

		let target = SectorID::new(0, 1);
		let actual = CompassDir::West;
		assert_eq!(
			actual,
			CompassDir::sector_to_sector_direction(&target, &source).unwrap()
		);

		let target = SectorID::new(2, 2);
		assert!(CompassDir::sector_to_sector_direction(&target, &source).is_none());
	}
}
