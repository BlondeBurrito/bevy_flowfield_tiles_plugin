//! A map is split into a series of `MxN` sectors composed of various fields
//! used for path calculation
//!
//!

pub mod sector_cost;

use bevy::prelude::*;

use crate::flowfields::utilities::CompassDir;

/// Unique ID of a sector
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Reflect)]
pub struct SectorID {
	/// Column
	column: i32,
	/// Row
	row: i32,
}

impl SectorID {
	/// Create a new instance of [SectorID]
	pub fn new(column: i32, row: i32) -> Self {
		SectorID { column, row }
	}
	/// Get the sector `(column, row)` tuple
	pub fn get(&self) -> (i32, i32) {
		(self.column, self.row)
	}
	/// Get the sector column
	pub fn get_column(&self) -> i32 {
		self.column
	}
	/// Get the sector row
	pub fn get_row(&self) -> i32 {
		self.row
	}
	/// Get the [SectorID] in an [CompassDir] direction a number of `steps` away from self
	///
	/// Warning: the calculated sector may not exist and should be verified after
	/// computation
	pub fn get_in_compass_direction(&self, compass_dir: &CompassDir, steps: usize) -> SectorID {
		let steps = steps as i32;
		match compass_dir {
			CompassDir::North => SectorID::new(self.column, self.row - steps),
			CompassDir::East => SectorID::new(self.column + steps, self.row),
			CompassDir::South => SectorID::new(self.column, self.row + steps),
			CompassDir::West => SectorID::new(self.column - steps, self.row),
			CompassDir::NorthEast => SectorID::new(self.column + steps, self.row - steps),
			CompassDir::SouthEast => SectorID::new(self.column + steps, self.row + steps),
			CompassDir::SouthWest => SectorID::new(self.column - steps, self.row + steps),
			CompassDir::NorthWest => SectorID::new(self.column - steps, self.row - steps),
			CompassDir::Zero => panic!("CompassDir::Zero should never be used to sector stepping"),
		}
	}
	/// Get all possible [SectorID] around `self`, including diagonals
	///
	/// Warning: some calculated sectors may not exist and should be verified
	/// after computation
	pub fn get_surrounding_sectors(&self) -> [SectorID; 8] {
		[
			self.get_in_compass_direction(&CompassDir::North, 1),
			self.get_in_compass_direction(&CompassDir::East, 1),
			self.get_in_compass_direction(&CompassDir::South, 1),
			self.get_in_compass_direction(&CompassDir::West, 1),
			self.get_in_compass_direction(&CompassDir::NorthEast, 1),
			self.get_in_compass_direction(&CompassDir::SouthEast, 1),
			self.get_in_compass_direction(&CompassDir::SouthWest, 1),
			self.get_in_compass_direction(&CompassDir::NorthWest, 1),
		]
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compass_dir_north() {
		let sector = SectorID::new(1, 1);
		let compass_dir = CompassDir::North;
		let steps = 1;

		let result = sector.get_in_compass_direction(&compass_dir, steps);
		let actual = SectorID::new(1, 0);

		assert_eq!(actual, result);
	}

	#[test]
	fn compass_dir_east() {
		let sector = SectorID::new(1, 1);
		let compass_dir = CompassDir::East;
		let steps = 1;

		let result = sector.get_in_compass_direction(&compass_dir, steps);
		let actual = SectorID::new(2, 1);

		assert_eq!(actual, result);
	}

	#[test]
	fn compass_dir_south() {
		let sector = SectorID::new(1, 1);
		let compass_dir = CompassDir::South;
		let steps = 1;

		let result = sector.get_in_compass_direction(&compass_dir, steps);
		let actual = SectorID::new(1, 2);

		assert_eq!(actual, result);
	}

	#[test]
	fn compass_dir_west() {
		let sector = SectorID::new(1, 1);
		let compass_dir = CompassDir::West;
		let steps = 1;

		let result = sector.get_in_compass_direction(&compass_dir, steps);
		let actual = SectorID::new(0, 1);

		assert_eq!(actual, result);
	}

	#[test]
	fn compass_dir_northeast() {
		let sector = SectorID::new(1, 1);
		let compass_dir = CompassDir::NorthEast;
		let steps = 1;

		let result = sector.get_in_compass_direction(&compass_dir, steps);
		let actual = SectorID::new(2, 0);

		assert_eq!(actual, result);
	}

	#[test]
	fn compass_dir_southeast() {
		let sector = SectorID::new(1, 1);
		let compass_dir = CompassDir::SouthEast;
		let steps = 1;

		let result = sector.get_in_compass_direction(&compass_dir, steps);
		let actual = SectorID::new(2, 2);

		assert_eq!(actual, result);
	}

	#[test]
	fn compass_dir_southwest() {
		let sector = SectorID::new(1, 1);
		let compass_dir = CompassDir::SouthWest;
		let steps = 1;

		let result = sector.get_in_compass_direction(&compass_dir, steps);
		let actual = SectorID::new(0, 2);

		assert_eq!(actual, result);
	}

	#[test]
	fn compass_dir_northwest() {
		let sector = SectorID::new(1, 1);
		let compass_dir = CompassDir::NorthWest;
		let steps = 1;

		let result = sector.get_in_compass_direction(&compass_dir, steps);
		let actual = SectorID::new(0, 0);

		assert_eq!(actual, result);
	}
}
