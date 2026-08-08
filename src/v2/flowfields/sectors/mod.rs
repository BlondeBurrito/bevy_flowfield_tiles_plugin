//! A map is split into a series of `MxN` sectors composed of various fields
//! used for path calculation
//!
//!

pub mod sector_cost;
pub mod sector_portals;

use bevy::prelude::*;

use crate::v2::flowfields::utilities::Ordinal;

/// Unique ID of a sector
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Reflect)]
pub struct SectorID {
	column: i32,
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
	/// Get the [SectorID] in an [Ordinal] direction a number of `steps` away from self
	///
	/// Warning: the calculated sector may not exist and should be verified after
	/// computation
	pub fn get_in_ordinal_direction(&self, ordinal: &Ordinal, steps: usize) -> SectorID {
		let steps = steps as i32;
		match ordinal {
			Ordinal::North => SectorID::new(self.column, self.row - steps),
			Ordinal::East => SectorID::new(self.column + steps, self.row),
			Ordinal::South => SectorID::new(self.column, self.row + steps),
			Ordinal::West => SectorID::new(self.column - steps, self.row),
			Ordinal::NorthEast => SectorID::new(self.column + steps, self.row - steps),
			Ordinal::SouthEast => SectorID::new(self.column + steps, self.row + steps),
			Ordinal::SouthWest => SectorID::new(self.column - steps, self.row + steps),
			Ordinal::NorthWest => SectorID::new(self.column - steps, self.row - steps),
			Ordinal::Zero => panic!("Ordinal::Zero should never be used to sector stepping"),
		}
	}
	/// Get all possible [SectorID] around `self`, including diagonals
	///
	/// Warning: some calculated sectors may not exist and should be verified
	/// after computation
	pub fn get_surrounding_sectors(&self) -> [SectorID; 8] {
		[
			self.get_in_ordinal_direction(&Ordinal::North, 1),
			self.get_in_ordinal_direction(&Ordinal::East, 1),
			self.get_in_ordinal_direction(&Ordinal::South, 1),
			self.get_in_ordinal_direction(&Ordinal::West, 1),
			self.get_in_ordinal_direction(&Ordinal::NorthEast, 1),
			self.get_in_ordinal_direction(&Ordinal::SouthEast, 1),
			self.get_in_ordinal_direction(&Ordinal::SouthWest, 1),
			self.get_in_ordinal_direction(&Ordinal::NorthWest, 1),
		]
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ordinal_dir_north() {
		let sector = SectorID::new(1, 1);
		let ordinal = Ordinal::North;
		let steps = 1;

		let result = sector.get_in_ordinal_direction(&ordinal, steps);
		let actual = SectorID::new(1, 0);

		assert_eq!(actual, result);
	}

	#[test]
	fn ordinal_dir_east() {
		let sector = SectorID::new(1, 1);
		let ordinal = Ordinal::East;
		let steps = 1;

		let result = sector.get_in_ordinal_direction(&ordinal, steps);
		let actual = SectorID::new(2, 1);

		assert_eq!(actual, result);
	}

	#[test]
	fn ordinal_dir_south() {
		let sector = SectorID::new(1, 1);
		let ordinal = Ordinal::South;
		let steps = 1;

		let result = sector.get_in_ordinal_direction(&ordinal, steps);
		let actual = SectorID::new(1, 2);

		assert_eq!(actual, result);
	}

	#[test]
	fn ordinal_dir_west() {
		let sector = SectorID::new(1, 1);
		let ordinal = Ordinal::West;
		let steps = 1;

		let result = sector.get_in_ordinal_direction(&ordinal, steps);
		let actual = SectorID::new(0, 1);

		assert_eq!(actual, result);
	}

	#[test]
	fn ordinal_dir_northeast() {
		let sector = SectorID::new(1, 1);
		let ordinal = Ordinal::NorthEast;
		let steps = 1;

		let result = sector.get_in_ordinal_direction(&ordinal, steps);
		let actual = SectorID::new(2, 0);

		assert_eq!(actual, result);
	}

	#[test]
	fn ordinal_dir_southeast() {
		let sector = SectorID::new(1, 1);
		let ordinal = Ordinal::SouthEast;
		let steps = 1;

		let result = sector.get_in_ordinal_direction(&ordinal, steps);
		let actual = SectorID::new(2, 2);

		assert_eq!(actual, result);
	}

	#[test]
	fn ordinal_dir_southwest() {
		let sector = SectorID::new(1, 1);
		let ordinal = Ordinal::SouthWest;
		let steps = 1;

		let result = sector.get_in_ordinal_direction(&ordinal, steps);
		let actual = SectorID::new(0, 2);

		assert_eq!(actual, result);
	}

	#[test]
	fn ordinal_dir_northwest() {
		let sector = SectorID::new(1, 1);
		let ordinal = Ordinal::NorthWest;
		let steps = 1;

		let result = sector.get_in_ordinal_direction(&ordinal, steps);
		let actual = SectorID::new(0, 0);

		assert_eq!(actual, result);
	}
}
