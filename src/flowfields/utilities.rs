//! Useful structures and tools used by the fields
//!

use crate::prelude::*;
use bevy::prelude::*;

/// Defines the dimenions of all field arrays
pub const FIELD_RESOLUTION: usize = 10;

/// Convenience way of accessing the 4 sides of a sector in [crate::prelude::Portals], the 4 sides of a field cell in [crate::prelude::IntegrationField] and the 8 directions
/// of movement in [crate::prelude::flow_field::FlowField]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, PartialEq, Clone, Copy, Reflect)]
pub enum Ordinal {
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

impl Ordinal {
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
	/// Based on a field cells `(column, row)` and an [Ordinal] direction find the neighbouring [FieldCell] if one exists
	pub fn get_cell_neighbour(cell_id: FieldCell, ordinal: Ordinal) -> Option<FieldCell> {
		match ordinal {
			Ordinal::North => {
				if cell_id.get_row() > 0 {
					Some(FieldCell::new(cell_id.get_column(), cell_id.get_row() - 1))
				} else {
					None
				}
			}
			Ordinal::East => {
				if cell_id.get_column() < FIELD_RESOLUTION - 1 {
					Some(FieldCell::new(cell_id.get_column() + 1, cell_id.get_row()))
				} else {
					None
				}
			}
			Ordinal::South => {
				if cell_id.get_row() < FIELD_RESOLUTION - 1 {
					Some(FieldCell::new(cell_id.get_column(), cell_id.get_row() + 1))
				} else {
					None
				}
			}
			Ordinal::West => {
				if cell_id.get_column() > 0 {
					Some(FieldCell::new(cell_id.get_column() - 1, cell_id.get_row()))
				} else {
					None
				}
			}
			Ordinal::NorthEast => {
				if cell_id.get_row() > 0 && cell_id.get_column() < FIELD_RESOLUTION - 1 {
					Some(FieldCell::new(
						cell_id.get_column() + 1,
						cell_id.get_row() - 1,
					))
				} else {
					None
				}
			}
			Ordinal::SouthEast => {
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
			Ordinal::SouthWest => {
				if cell_id.get_row() < FIELD_RESOLUTION - 1 && cell_id.get_column() > 0 {
					Some(FieldCell::new(
						cell_id.get_column() - 1,
						cell_id.get_row() + 1,
					))
				} else {
					None
				}
			}
			Ordinal::NorthWest => {
				if cell_id.get_row() > 0 && cell_id.get_column() > 0 {
					Some(FieldCell::new(
						cell_id.get_column() - 1,
						cell_id.get_row() - 1,
					))
				} else {
					None
				}
			}
			Ordinal::Zero => None,
		}
	}
	/// Based on a field cells `(column, row)` position find all possible neighbours including diagonal directions
	pub fn get_all_cell_neighbours(cell_id: FieldCell) -> Vec<FieldCell> {
		let mut neighbours = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let mut diagonals = Ordinal::get_diagonal_cell_neighbours(cell_id);
		neighbours.append(&mut diagonals);
		neighbours
	}
	/// Based on a field cells `(column, row)` position find all possible neighbours including diagonal directions and the Ordinal they are found in
	pub fn get_all_cell_neighbours_with_ordinal(cell_id: FieldCell) -> Vec<(Ordinal, FieldCell)> {
		let mut neighbours = Vec::new();
		if cell_id.get_row() > 0 {
			neighbours.push((
				Ordinal::North,
				FieldCell::new(cell_id.get_column(), cell_id.get_row() - 1),
			)); // northern cell coords
		}
		if cell_id.get_column() < FIELD_RESOLUTION - 1 {
			neighbours.push((
				Ordinal::East,
				FieldCell::new(cell_id.get_column() + 1, cell_id.get_row()),
			)); // eastern cell coords
		}
		if cell_id.get_row() < FIELD_RESOLUTION - 1 {
			neighbours.push((
				Ordinal::South,
				FieldCell::new(cell_id.get_column(), cell_id.get_row() + 1),
			)); // southern cell coords
		}
		if cell_id.get_column() > 0 {
			neighbours.push((
				Ordinal::West,
				FieldCell::new(cell_id.get_column() - 1, cell_id.get_row()),
			)); // western cell coords
		}
		if cell_id.get_row() > 0 && cell_id.get_column() < FIELD_RESOLUTION - 1 {
			neighbours.push((
				Ordinal::NorthEast,
				FieldCell::new(cell_id.get_column() + 1, cell_id.get_row() - 1),
			)); // north-east cell
		}
		if cell_id.get_row() < FIELD_RESOLUTION - 1 && cell_id.get_column() < FIELD_RESOLUTION - 1 {
			neighbours.push((
				Ordinal::SouthEast,
				FieldCell::new(cell_id.get_column() + 1, cell_id.get_row() + 1),
			)); // south-east cell
		}
		if cell_id.get_row() < FIELD_RESOLUTION - 1 && cell_id.get_column() > 0 {
			neighbours.push((
				Ordinal::SouthWest,
				FieldCell::new(cell_id.get_column() - 1, cell_id.get_row() + 1),
			)); // south-west cell
		}
		if cell_id.get_row() > 0 && cell_id.get_column() > 0 {
			neighbours.push((
				Ordinal::NorthWest,
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
		map_length: u32,
		map_depth: u32,
		sector_resolution: u32,
	) -> Vec<SectorID> {
		let mut neighbours = Vec::new();
		let sector_column_limit = map_length / sector_resolution - 1;
		let sector_row_limit = map_depth / sector_resolution - 1;
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
	/// Based on a sectors `(column, row)` position find the [Ordinal] directions for its boundaries that can support [crate::prelude::Portals]
	pub fn get_sector_portal_ordinals(
		sector_id: &SectorID,
		map_length: u32,
		map_depth: u32,
		sector_resolution: u32,
	) -> Vec<Ordinal> {
		let mut neighbours = Vec::new();
		let sector_column_limit = map_length / sector_resolution - 1;
		let sector_row_limit = map_depth / sector_resolution - 1;
		if sector_id.get_row() > 0 {
			neighbours.push(Ordinal::North); // northern sector coords
		}
		if sector_id.get_column() < sector_column_limit {
			neighbours.push(Ordinal::East); // eastern sector coords
		}
		if sector_id.get_row() < sector_row_limit {
			neighbours.push(Ordinal::South); // southern sector coords
		}
		if sector_id.get_column() > 0 {
			neighbours.push(Ordinal::West); // western sector coords
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find its neighbours based on map size limits (up to 4) and include the [Ordinal] direction in the result
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
	pub fn get_sector_neighbours_with_ordinal(
		sector_id: &SectorID,
		map_x_dimension: u32,
		map_z_dimension: u32,
		sector_resolution: u32,
	) -> Vec<(Ordinal, SectorID)> {
		let mut neighbours = Vec::new();
		let sector_x_column_limit = map_x_dimension / sector_resolution - 1;
		let sector_z_row_limit = map_z_dimension / sector_resolution - 1;
		if sector_id.get_row() > 0 {
			neighbours.push((
				Ordinal::North,
				SectorID::new(sector_id.get_column(), sector_id.get_row() - 1),
			)); // northern sector coords
		}
		if sector_id.get_column() < sector_x_column_limit {
			neighbours.push((
				Ordinal::East,
				SectorID::new(sector_id.get_column() + 1, sector_id.get_row()),
			)); // eastern sector coords
		}
		if sector_id.get_row() < sector_z_row_limit {
			neighbours.push((
				Ordinal::South,
				SectorID::new(sector_id.get_column(), sector_id.get_row() + 1),
			)); // southern sector coords
		}
		if sector_id.get_column() > 0 {
			neighbours.push((
				Ordinal::West,
				SectorID::new(sector_id.get_column() - 1, sector_id.get_row()),
			)); // western sector coords
		}
		neighbours
	}
	/// Returns the opposite [Ordinal] of the current
	pub fn inverse(&self) -> Ordinal {
		match self {
			Ordinal::North => Ordinal::South,
			Ordinal::East => Ordinal::West,
			Ordinal::South => Ordinal::North,
			Ordinal::West => Ordinal::East,
			Ordinal::NorthEast => Ordinal::SouthWest,
			Ordinal::SouthEast => Ordinal::NorthWest,
			Ordinal::SouthWest => Ordinal::NorthEast,
			Ordinal::NorthWest => Ordinal::SouthEast,
			Ordinal::Zero => Ordinal::Zero,
		}
	}
	/// For two cells next to each other it can be useful to find the [Ordinal] point from the `source` to the `target`
	pub fn cell_to_cell_direction(target: FieldCell, source: FieldCell) -> Self {
		let i32_target = (target.get_column() as i32, target.get_row() as i32);
		let i32_source = (source.get_column() as i32, source.get_row() as i32);

		let direction = (i32_target.0 - i32_source.0, i32_target.1 - i32_source.1);
		match direction {
			(0, -1) => Ordinal::North,
			(1, -1) => Ordinal::NorthEast,
			(1, 0) => Ordinal::East,
			(1, 1) => Ordinal::SouthEast,
			(0, 1) => Ordinal::South,
			(-1, 1) => Ordinal::SouthWest,
			(-1, 0) => Ordinal::West,
			(-1, -1) => Ordinal::NorthWest,
			_ => panic!(
				"Cell {:?} is not orthogonally or diagonally adjacent to {:?}",
				target, source
			),
		}
	}
	/// For two sectors next to each other it can be useful to find the [Ordinal] from the `source` to the `target`. If they are not adjacent None is returned
	pub fn sector_to_sector_direction(target: SectorID, source: SectorID) -> Option<Self> {
		let i32_target = (target.get_column() as i32, target.get_row() as i32);
		let i32_source = (source.get_column() as i32, source.get_row() as i32);

		let direction = (i32_target.0 - i32_source.0, i32_target.1 - i32_source.1);
		match direction {
			(0, -1) => Some(Ordinal::North),
			(1, 0) => Some(Ordinal::East),
			(0, 1) => Some(Ordinal::South),
			(-1, 0) => Some(Ordinal::West),
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
	fn ordinal_field_cell_neighbours() {
		let cell_id = FieldCell::new(0, 0);
		let result = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![FieldCell::new(1, 0), FieldCell::new(0, 1)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_field_cell_neighbours2() {
		let cell_id = FieldCell::new(9, 9);
		let result = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![FieldCell::new(9, 8), FieldCell::new(8, 9)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_field_cell_neighbours3() {
		let cell_id = FieldCell::new(4, 4);
		let result = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![
			FieldCell::new(4, 3),
			FieldCell::new(5, 4),
			FieldCell::new(4, 5),
			FieldCell::new(3, 4),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_field_cell_neighbours4() {
		let cell_id = FieldCell::new(5, 0);
		let result = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![
			FieldCell::new(6, 0),
			FieldCell::new(5, 1),
			FieldCell::new(4, 0),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours() {
		let sector_id = SectorID::new(0, 0);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_neighbours(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
		);
		let actual = vec![SectorID::new(1, 0), SectorID::new(0, 1)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours2() {
		let sector_id = SectorID::new(29, 54);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_neighbours(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
		);
		let actual = vec![SectorID::new(29, 53), SectorID::new(28, 54)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours3() {
		let sector_id = SectorID::new(14, 31);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_neighbours(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
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
	fn ordinal_sector_neighbours4() {
		let sector_id = SectorID::new(0, 13);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_neighbours(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
		);
		let actual = vec![
			SectorID::new(0, 12),
			SectorID::new(1, 13),
			SectorID::new(0, 14),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_northern_oridnals() {
		let sector_id = SectorID::new(3, 0);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_portal_ordinals(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
		);
		let actual = vec![Ordinal::East, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_eastern_oridnals() {
		let sector_id = SectorID::new(19, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_portal_ordinals(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
		);
		let actual = vec![Ordinal::North, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_southern_oridnals() {
		let sector_id = SectorID::new(4, 19);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_portal_ordinals(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
		);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_western_oridnals() {
		let sector_id = SectorID::new(0, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_portal_ordinals(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
		);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::South];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_centre_oridnals() {
		let sector_id = SectorID::new(4, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let sector_resolution = 10;
		let result = Ordinal::get_sector_portal_ordinals(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
			sector_resolution,
		);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north() {
		let target = FieldCell::new(6, 2);
		let source = FieldCell::new(6, 3);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::North;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north_east() {
		let target = FieldCell::new(7, 2);
		let source = FieldCell::new(6, 3);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::NorthEast;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_east() {
		let target = FieldCell::new(6, 7);
		let source = FieldCell::new(5, 7);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::East;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south_east() {
		let target = FieldCell::new(5, 5);
		let source = FieldCell::new(4, 4);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::SouthEast;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south() {
		let target = FieldCell::new(3, 1);
		let source = FieldCell::new(3, 0);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::South;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south_west() {
		let target = FieldCell::new(6, 9);
		let source = FieldCell::new(7, 8);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::SouthWest;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_west() {
		let target = FieldCell::new(5, 7);
		let source = FieldCell::new(6, 7);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::West;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north_west() {
		let target = FieldCell::new(0, 0);
		let source = FieldCell::new(1, 1);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::NorthWest;
		assert_eq!(actual, result);
	}
	#[test]
	fn neighbours_with_ordinal1() {
		let field = FieldCell::new(3, 4);
		let result = Ordinal::get_all_cell_neighbours_with_ordinal(field);
		let actual = vec![
			((Ordinal::North, FieldCell::new(3, 3))),
			((Ordinal::East, FieldCell::new(4, 4))),
			((Ordinal::South, FieldCell::new(3, 5))),
			((Ordinal::West, FieldCell::new(2, 4))),
			((Ordinal::NorthEast, FieldCell::new(4, 3))),
			((Ordinal::SouthEast, FieldCell::new(4, 5))),
			((Ordinal::SouthWest, FieldCell::new(2, 5))),
			((Ordinal::NorthWest, FieldCell::new(2, 3))),
		];
		assert_eq!(actual, result)
	}
	#[test]
	fn neighbours_with_ordinal2() {
		let field = FieldCell::new(0, 0);
		let result = Ordinal::get_all_cell_neighbours_with_ordinal(field);
		let actual = vec![
			((Ordinal::East, FieldCell::new(1, 0))),
			((Ordinal::South, FieldCell::new(0, 1))),
			((Ordinal::SouthEast, FieldCell::new(1, 1))),
		];
		assert_eq!(actual, result)
	}
}
