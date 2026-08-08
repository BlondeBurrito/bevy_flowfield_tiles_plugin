//! The CostField contains an array of 8-bit values. The values correspond to the cost of that
//! cell in the array. A value of 1 is the default, a value of 255 is a special case that indicates
//! that the field cell is strictly forbidden from being used in a pathing calculation (effectively
//! saying there is a wall or cliff/impassable terrain there). Any other value indicates a harder
//! cost of movement which could be from a slope or marshland or others.
//!
//! Every Sector has a [CostField] associated with it. An example cost field visualised as a 2d grid may look:
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

// pub mod neighbours;

use bevy::reflect::Reflect;
use serde_big_array::BigArray;

use crate::v2::flowfields::{
	fields::{Field, FieldCell},
	utilities::FIELD_RESOLUTION,
};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Reflect, Debug)]
pub struct CostField {
	/// One dimensional array of cost values
	#[serde(with = "BigArray")]
	field: [u8; FIELD_RESOLUTION * FIELD_RESOLUTION],
}

impl Default for CostField {
	fn default() -> Self {
		CostField {
			field: [1_u8; FIELD_RESOLUTION * FIELD_RESOLUTION],
		}
	}
}

impl Field<u8> for CostField {
	/// Get a reference to the field array
	fn get(&self) -> &[u8; FIELD_RESOLUTION * FIELD_RESOLUTION] {
		&self.field
	}
	/// Retrieve a field cell value
	///
	/// NB: This will panic if out of bounds
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u8 {
		let index = field_cell.as_1d_index();
		self.field[index]
	}
	/// Set a field cell to a value
	///
	/// NB: This will panic if out of bounds
	fn set_field_cell_value(&mut self, value: u8, field_cell: FieldCell) {
		let index = field_cell.as_1d_index();
		self.field[index] = value;
	}
}

impl CostField {
	/// Create a new [CostField] with all cell values initialised with `cost`
	pub fn new_with_cost(cost: u8) -> Self {
		CostField {
			field: [cost; FIELD_RESOLUTION * FIELD_RESOLUTION],
		}
	}
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn update() {
		let mut field = CostField::new_with_cost(5);
		let cell = FieldCell::new(3, 6);
		field.set_field_cell_value(128, cell);
		let actual = 128;
		let result = field.get_field_cell_value(cell);
		assert_eq!(actual, result);
	}
}
