//! A [FlowField] is a 2D array of 8-bit values. The various bit values
//! associated with it indicate directions of movement and flags to identify
//! what's a goal, what's pathable and others. A steering pipeline/character
//! controller should read and interpret a [FlowField] to provide movement.
//!

use bevy::prelude::*;
use serde_big_array::BigArray;

use crate::v2::flowfields::{
	fields::{Field, FieldCell},
	utilities::{FIELD_RESOLUTION, Ordinal},
};

/// Bit to indicate a northerly direction
const BITS_NORTH: u8 = 0b0000_0001;
/// Bit to indicate an easterly direction
const BITS_EAST: u8 = 0b0000_0010;
/// Bit to indicate a southerly direction
const BITS_SOUTH: u8 = 0b0000_0100;
/// Bit to indicate a westerly direction
const BITS_WEST: u8 = 0b0000_1000;
/// Bit to indicate a north-easterly direction
const BITS_NORTH_EAST: u8 = 0b0000_0011;
/// Bit to indicate a south-easterly direction
const BITS_SOUTH_EAST: u8 = 0b0000_0110;
/// Bit to indicate south-westerly direction
const BITS_SOUTH_WEST: u8 = 0b0000_1100;
/// Bit to indicate a north-westerly direction
const BITS_NORTH_WEST: u8 = 0b0000_1001;
/// Bit to indicate an impassable field
const BITS_ZERO: u8 = 0b0000_0000;
/// Default field cell value of a new [FlowField]
const BITS_DEFAULT: u8 = 0b0000_1111;
/// Flags a pathable field cell
const BITS_PATHABLE: u8 = 0b0001_0000;
/// Flags a field cell that has line-of-sight to the goal
const BITS_HAS_LOS: u8 = 0b0010_0000;
/// Flags a field cell as being the goal
const BITS_GOAL: u8 = 0b0100_0000;
/// Flags a field cell as being a portal to another sector
const BITS_PORTAL_GOAL: u8 = 0b1000_0000;

/// Convert an [Ordinal] to a bit representation
pub fn convert_ordinal_to_bits_dir(ordinal: Ordinal) -> u8 {
	match ordinal {
		Ordinal::North => BITS_NORTH,
		Ordinal::East => BITS_EAST,
		Ordinal::South => BITS_SOUTH,
		Ordinal::West => BITS_WEST,
		Ordinal::NorthEast => BITS_NORTH_EAST,
		Ordinal::SouthEast => BITS_SOUTH_EAST,
		Ordinal::SouthWest => BITS_SOUTH_WEST,
		Ordinal::NorthWest => BITS_NORTH_WEST,
		Ordinal::Zero => BITS_ZERO,
	}
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Reflect)]
pub struct FlowField {
	/// One dimensional array of bit vectors
	#[serde(with = "BigArray")]
	field: [u8; FIELD_RESOLUTION * FIELD_RESOLUTION],
}

impl Default for FlowField {
	fn default() -> Self {
		FlowField {
			field: [BITS_DEFAULT; FIELD_RESOLUTION * FIELD_RESOLUTION],
		}
	}
}

impl Field<u8> for FlowField {
	/// Get a reference to the field array
	fn get(&self) -> &[u8; FIELD_RESOLUTION * FIELD_RESOLUTION] {
		&self.field
	}
	/// Retrieve a field cell value
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u8 {
		self.field[field_cell.as_1d_index()]
	}
	/// Set a field cell to a value
	fn set_field_cell_value(&mut self, value: u8, field_cell: FieldCell) {
		self.field[field_cell.as_1d_index()] = value;
	}
}
