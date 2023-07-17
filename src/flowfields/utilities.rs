//! Useful structures and tools used by the fields
//!

use bevy::prelude::*;

/// Determines the number of Sectors by dividing the map length and depth by this value
pub const SECTOR_RESOLUTION: usize = 10;
/// Defines the dimenions of all field arrays
pub const FIELD_RESOLUTION: usize = 10;

/// Convenience way of accessing the 4 sides of a sector in [crate::prelude::Portals], the 4 sides of a grid cell in [crate::prelude::IntegrationField] and the 8 directions
/// of movement in [crate::prelude::flow_field::FlowField]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, PartialEq, Clone, Copy)]
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
	/// Based on a grid cells `(column, row)` position find its neighbours based on FIELD_RESOLUTION limits (up to 4)
	pub fn get_orthogonal_cell_neighbours(cell_id: (usize, usize)) -> Vec<(usize, usize)> {
		let mut neighbours = Vec::new();
		if cell_id.1 > 0 {
			neighbours.push((cell_id.0, cell_id.1 - 1)); // northern cell coords
		}
		if cell_id.0 < FIELD_RESOLUTION - 1 {
			neighbours.push((cell_id.0 + 1, cell_id.1)); // eastern cell coords
		}
		if cell_id.1 < FIELD_RESOLUTION - 1 {
			neighbours.push((cell_id.0, cell_id.1 + 1)); // southern cell coords
		}
		if cell_id.0 > 0 {
			neighbours.push((cell_id.0 - 1, cell_id.1)); // western cell coords
		}
		neighbours
	}
	/// Based on a grid cells `(column, row)` position find all possible neighbours inclduing diagonal directions
	pub fn get_all_cell_neighbours(cell_id: (usize, usize)) -> Vec<(usize, usize)> {
		let mut neighbours = Vec::new();
		if cell_id.1 > 0 {
			neighbours.push((cell_id.0, cell_id.1 - 1)); // northern cell coords
		}
		if cell_id.0 < FIELD_RESOLUTION - 1 {
			neighbours.push((cell_id.0 + 1, cell_id.1)); // eastern cell coords
		}
		if cell_id.1 < FIELD_RESOLUTION - 1 {
			neighbours.push((cell_id.0, cell_id.1 + 1)); // southern cell coords
		}
		if cell_id.0 > 0 {
			neighbours.push((cell_id.0 - 1, cell_id.1)); // western cell coords
		}
		if cell_id.1 > 0 && cell_id.0 < FIELD_RESOLUTION - 1 {
			neighbours.push((cell_id.0 + 1, cell_id.1 - 1)); // north-east cell
		}
		if cell_id.1 < FIELD_RESOLUTION - 1 && cell_id.0 < FIELD_RESOLUTION - 1 {
			neighbours.push((cell_id.0 + 1, cell_id.1 + 1)); // south-east cell
		}
		if cell_id.1 < FIELD_RESOLUTION - 1 && cell_id.0 > 0 {
			neighbours.push((cell_id.0 - 1, cell_id.1 + 1)); // south-west cell
		}
		if cell_id.1 > 0 && cell_id.0 > 0 {
			neighbours.push((cell_id.0 - 1, cell_id.1 - 1)); // north-west cell
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find its neighbours based on map size limits (up to 4)
	pub fn get_sector_neighbours(
		sector_id: &(u32, u32),
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> Vec<(u32, u32)> {
		let mut neighbours = Vec::new();
		let sector_x_column_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
		let sector_z_row_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;
		if sector_id.1 > 0 {
			neighbours.push((sector_id.0, sector_id.1 - 1)); // northern sector coords
		}
		if sector_id.0 < sector_x_column_limit {
			neighbours.push((sector_id.0 + 1, sector_id.1)); // eastern sector coords
		}
		if sector_id.1 < sector_z_row_limit {
			neighbours.push((sector_id.0, sector_id.1 + 1)); // southern sector coords
		}
		if sector_id.0 > 0 {
			neighbours.push((sector_id.0 - 1, sector_id.1)); // western sector coords
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find the [Ordinal] directions for its boundaries that can support [crate::prelude::Portals]
	pub fn get_sector_portal_ordinals(
		sector_id: &(u32, u32),
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> Vec<Ordinal> {
		let mut neighbours = Vec::new();
		let sector_x_column_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
		let sector_z_row_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;
		if sector_id.1 > 0 {
			neighbours.push(Ordinal::North); // northern sector coords
		}
		if sector_id.0 < sector_x_column_limit {
			neighbours.push(Ordinal::East); // eastern sector coords
		}
		if sector_id.1 < sector_z_row_limit {
			neighbours.push(Ordinal::South); // southern sector coords
		}
		if sector_id.0 > 0 {
			neighbours.push(Ordinal::West); // western sector coords
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find its neighbours based on map size limits (up to 4) and include the [Ordinal] direction in the result
	pub fn get_sector_neighbours_with_ordinal(
		sector_id: &(u32, u32),
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> Vec<(Ordinal, (u32, u32))> {
		let mut neighbours = Vec::new();
		let sector_x_column_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
		let sector_z_row_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;
		if sector_id.1 > 0 {
			neighbours.push((Ordinal::North, (sector_id.0, sector_id.1 - 1))); // northern sector coords
		}
		if sector_id.0 < sector_x_column_limit {
			neighbours.push((Ordinal::East, (sector_id.0 + 1, sector_id.1))); // eastern sector coords
		}
		if sector_id.1 < sector_z_row_limit {
			neighbours.push((Ordinal::South, (sector_id.0, sector_id.1 + 1))); // southern sector coords
		}
		if sector_id.0 > 0 {
			neighbours.push((Ordinal::West, (sector_id.0 - 1, sector_id.1))); // western sector coords
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
	pub fn cell_to_cell_direction(target: (usize, usize), source: (usize, usize)) -> Self {
		let i32_target = (target.0 as i32, target.1 as i32);
		let i32_source = (source.0 as i32, source.1 as i32);

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
	/// For two sectors next to each other it can be useful to find the [Ordinal] from the `source` to the `target`. This will panic if the two sectors are not orthogonally adjacent
	pub fn sector_to_sector_direction(target: (u32, u32), source: (u32, u32)) -> Option<Self> {
		let i32_target = (target.0 as i32, target.1 as i32);
		let i32_source = (source.0 as i32, source.1 as i32);

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
	fn ordinal_grid_cell_neighbours() {
		let cell_id = (0, 0);
		let result = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![(1, 0), (0, 1)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_grid_cell_neighbours2() {
		let cell_id = (9, 9);
		let result = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![(9, 8), (8, 9)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_grid_cell_neighbours3() {
		let cell_id = (4, 4);
		let result = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![(4, 3), (5, 4), (4, 5), (3, 4)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_grid_cell_neighbours4() {
		let cell_id = (5, 0);
		let result = Ordinal::get_orthogonal_cell_neighbours(cell_id);
		let actual = vec![(6, 0), (5, 1), (4, 0)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours() {
		let sector_id = (0, 0);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let result = Ordinal::get_sector_neighbours(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(1, 0), (0, 1)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours2() {
		let sector_id = (29, 54);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let result = Ordinal::get_sector_neighbours(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(29, 53), (28, 54)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours3() {
		let sector_id = (14, 31);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let result = Ordinal::get_sector_neighbours(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(14, 30), (15, 31), (14, 32), (13, 31)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours4() {
		let sector_id = (0, 13);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let result = Ordinal::get_sector_neighbours(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(0, 12), (1, 13), (0, 14)];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_northern_oridnals() {
		let sector_id = (3, 0);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::East, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_eastern_oridnals() {
		let sector_id = (19, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_southern_oridnals() {
		let sector_id = (4, 19);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_western_oridnals() {
		let sector_id = (0, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::South];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_centre_oridnals() {
		let sector_id = (4, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north() {
		let target = (6, 2);
		let source = (6, 3);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::North;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north_east() {
		let target = (7, 2);
		let source = (6, 3);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::NorthEast;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_east() {
		let target = (6, 7);
		let source = (5, 7);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::East;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south_east() {
		let target = (5, 5);
		let source = (4, 4);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::SouthEast;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south() {
		let target = (3, 1);
		let source = (3, 0);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::South;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_south_west() {
		let target = (6, 9);
		let source = (7, 8);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::SouthWest;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_west() {
		let target = (5, 7);
		let source = (6, 7);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::West;
		assert_eq!(actual, result);
	}
	#[test]
	fn cell_to_cell_north_west() {
		let target = (0, 0);
		let source = (1, 1);
		let result = Ordinal::cell_to_cell_direction(target, source);
		let actual = Ordinal::NorthWest;
		assert_eq!(actual, result);
	}
}
