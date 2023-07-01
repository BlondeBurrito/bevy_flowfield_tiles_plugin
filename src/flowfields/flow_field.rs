//!
//!

use super::{integration_field::IntegrationField, *};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FlowField([[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for FlowField {
	fn default() -> Self {
		FlowField([[0b0000_1111; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl FlowField {
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
	pub fn calculate(&mut self, source: (u32, u32), integration_field: &IntegrationField) {
		let mut queue: Vec<((usize, usize), u16)> = Vec::new();
	}
}