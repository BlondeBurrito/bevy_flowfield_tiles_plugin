//!
//!

use super::{integration_field::IntegrationField, *};


const BITS_NORTH: u8 = 0b0000_0001;
const BITS_EAST: u8 = 0b0000_0010;
const BITS_SOUTH: u8 = 0b0000_0100;
const BITS_WEST: u8 = 0b0000_1000;
const BITS_NORTH_EAST: u8 = 0b0000_0011;
const BITS_SOUTH_EAST: u8 = 0b0000_0110;
const BITS_SOUTH_WEST: u8 = 0b0000_1100;
const BITS_NORTH_WEST: u8 = 0b0000_1001;
const BITS_ZERO: u8 = 0b0000_0000;
const BITS_DEFAULT: u8 = 0b0000_1111;

const BITS_PATHABLE: u8 = 0b0001_0000;
const BITS_HAS_LOS: u8 = 0b0010_0000;
const BITS_GOAL: u8 = 0b0100_0000;
const BITS_PORTAL_GOAL: u8 = 0b1000_0000;

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
pub struct FlowField([[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for FlowField {
	fn default() -> Self {
		FlowField([[BITS_DEFAULT; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl FlowField {
	// pub fn new(&mut self, goals: &Vec<(usize, usize)>, previous_sector_ordinal: Option<Ordinal>) -> Self {
	// 	if previous_sector_ordinal.is_none() {
	// 		self.set_grid_value(BITS_GOAL, goals[0].0, goals[0].1);
	// 	} else {
	// 		for goal in goals.iter() {

	// 		}
	// 	}
	// }
	pub fn get_field(&self) -> &[[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION] {
		&self.0
	}
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
	pub fn calculate(
		&mut self,
		goals: &Vec<(usize, usize)>,
		previous_sector_ordinal: Option<Ordinal>,
		integration_field: &IntegrationField
	) {
		// set goal cells
		if previous_sector_ordinal.is_none() {
			self.set_grid_value(BITS_GOAL, goals[0].0, goals[0].1);
		} else {
			for goal in goals.iter() {
				let ord = previous_sector_ordinal.unwrap();
				let ordinal_bits = convert_ordinal_to_bits_dir(ord);
				let mut value = 0;
				value |= BITS_PORTAL_GOAL;
				value |= ordinal_bits;
				self.set_grid_value(value, goal.0, goal.1);
			}
		}
		// let mut queue: Vec<((usize, usize), u16)> = Vec::new();

		for (i, column) in integration_field.get_field().iter().enumerate() {
			for (j, _row) in column.iter().enumerate() {
				if self.get_grid_value(i, j) == BITS_DEFAULT {
					let current_cost = integration_field.get_grid_value(i, j);
					// mark impassable //TODO maybe skip? waste of time perhaps
					if current_cost == u16::MAX {
						self.set_grid_value(BITS_ZERO, i, j);
					} else if current_cost != 0 { // skip goals of zero
						// store the cheapest node
						let mut cheapest_value = u16::MAX;
						let mut cheapest_neighbour = None;
						let neighbours = Ordinal::get_all_cell_neighbours((i, j));
						for n in neighbours.iter() {
							let neighbour_cost = integration_field.get_grid_value(n.0, n.1);
							if neighbour_cost < cheapest_value {
								cheapest_value = neighbour_cost;
								cheapest_neighbour = Some(n);
							}
						}
						if let Some(target) = cheapest_neighbour {
							let ord = Ordinal::cell_to_cell_direction(*target, (i, j));
							let bit_ord = convert_ordinal_to_bits_dir(ord);
							let mut value = 0;
							value |= bit_ord;
							value |= BITS_PATHABLE;
							self.set_grid_value(value, i, j);
						}
					}
				}
			}
		}

		// fn process(flow_field: &mut FlowField, queue: Vec<(usize, usize)>, int_field: &IntegrationField) {
		// 	let mut next_neighbours = Vec::new();
		// }
	}
}

pub fn abc(cell_value: u8) {
	let flag_filter = 0b1111_0000;
	let dir_filter = 0b0000_1111;

	let flags = cell_value & flag_filter;
	match flags {
		BITS_GOAL => {
			// arrived at goal, 
		},
		BITS_PORTAL_GOAL => {},
		BITS_HAS_LOS => {},
		BITS_PATHABLE => {},
		_ => panic!("Last 4 bits of cell are not recognised flags"),
	}
}

pub fn get_ordinal_from_bits(cell_value: u8) -> Ordinal {
	let dir_filter = 0b0000_1111;
	let dir = cell_value & dir_filter;
	match dir {
		BITS_NORTH => Ordinal::North,
		BITS_EAST => Ordinal::East,
		BITS_SOUTH => Ordinal::South,
		BITS_WEST => Ordinal::West,
		BITS_NORTH_EAST => Ordinal::NorthEast,
		BITS_SOUTH_EAST => Ordinal::SouthEast,
		BITS_SOUTH_WEST => Ordinal::SouthWest,
		BITS_NORTH_WEST => Ordinal::NorthWest,
		BITS_ZERO => Ordinal::Zero,
		_ => panic!("First 4 bits og cell are not recognised directions"),
	}
}