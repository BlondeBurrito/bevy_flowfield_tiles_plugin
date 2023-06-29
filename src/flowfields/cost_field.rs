//! The CostField contains a 2D array of 8-bit values. The values correspond to the cost of that
//! grid in the array. A value of 1 is the default, a value of 255 is a special case that idicates
//! that the grid cell is strictly forbidden from being used in a pathing calculation (effectively
//! saying there is a wall or cliff/impassable terrain there). Any other value indicates a harder
//! cost of movement which could be from a slope or marshland or others.
//!
//! Every [Sector] has a [CostsField] associated with it. An example cost field may look:
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

use super::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CostField([[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for CostField {
	fn default() -> Self {
		CostField([[1; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl CostField {
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
	/// From a `ron` file generate the [CostField]
	#[cfg(feature = "ron")]
	pub fn from_file(path: String) -> Self {
		let file = std::fs::File::open(&path).expect("Failed opening CostField file");
		let field: CostField = match ron::de::from_reader(file) {
			Ok(field) => field,
			Err(e) => panic!("Failed deserializing CostField: {}", e),
		};
		field
	}
}

// /// A [CostField] grid is made up of a ([SECTOR_RESOLUTION]x[SECTOR_RESOLUTION]) array
// fn get_inidices_of_neighbouring_grid_cells(
// 	grid_cell: (u32, u32),
// 	map_x_dimension: u32,
// 	map_z_dimension: u32,
// ) -> Vec<(u32, u32)> {
// 	let sector_x_column_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
// 	let sector_z_row_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;

// 	if sector_id.0 == 0 && sector_id.1 == 0 {
// 		//top left sector only has 2 valid neighbours
// 		vec![(1, 0), (0, 1)]
// 	} else if sector_id.0 == sector_x_column_limit && sector_id.1 == 0 {
// 		// top right sector has only two valid neighbours
// 		vec![(sector_x_column_limit, 1), (sector_x_column_limit - 1, 0)]
// 	} else if sector_id.0 == sector_x_column_limit && sector_id.1 == sector_z_row_limit {
// 		// bottom right sector only has two valid neighbours
// 		vec![
// 			(sector_x_column_limit, sector_z_row_limit - 1),
// 			(sector_x_column_limit - 1, sector_z_row_limit),
// 		]
// 	} else if sector_id.0 == 0 && sector_id.1 == sector_z_row_limit {
// 		// bottom left sector only has two valid neighbours
// 		vec![(0, sector_z_row_limit - 1), (1, sector_z_row_limit)]
// 	} else if sector_id.0 > 0 && sector_id.0 < sector_x_column_limit && sector_id.1 == 0 {
// 		// northern row minus the corners sectors have three valid neighbours
// 		vec![(sector_id.0 + 1, 0), (sector_id.0, 1), (sector_id.0 - 1, 0)]
// 	} else if sector_id.0 == sector_x_column_limit
// 		&& sector_id.1 > 0
// 		&& sector_id.1 < sector_z_row_limit
// 	{
// 		// eastern column minus the corners have three sectors of valid neighbours
// 		vec![
// 			(sector_x_column_limit, sector_id.1 - 1),
// 			(sector_x_column_limit, sector_id.1 + 1),
// 			(sector_x_column_limit - 1, sector_id.1),
// 		]
// 	} else if sector_id.0 > 0
// 		&& sector_id.0 < sector_x_column_limit
// 		&& sector_id.1 == sector_z_row_limit
// 	{
// 		// southern row minus corners have three sectors of valid neighbours
// 		vec![
// 			(sector_id.0, sector_z_row_limit - 1),
// 			(sector_id.0 + 1, sector_z_row_limit),
// 			(sector_id.0 - 1, sector_z_row_limit),
// 		]
// 	} else if sector_id.0 == 0 && sector_id.1 > 0 && sector_id.1 < sector_z_row_limit {
// 		// western column minus corners have three sectors of valid neighbours
// 		vec![(0, sector_id.1 - 1), (1, sector_id.1), (0, sector_id.1 + 1)]
// 	} else if sector_id.0 > 0 && sector_id.0 < sector_x_column_limit && sector_id.1 > 0 && sector_id.1 < sector_z_row_limit {
// 		// all other sectors not along an edge of the map have four valid sectors for portals
// 		vec![
// 			(sector_id.0, sector_id.1 - 1),
// 			(sector_id.0 + 1, sector_id.1),
// 			(sector_id.0, sector_id.1 + 1),
// 			(sector_id.0 - 1, sector_id.1),
// 		]
// 	} else {
// 		// special case of no neighbours
// 		warn!("Sector ID {:?} does not fit within map dimensions, there are only `{}x{}` sectors", sector_id, map_x_dimension / SECTOR_RESOLUTION as u32, map_z_dimension / SECTOR_RESOLUTION as u32);
// 		vec![]
// 	}
// }

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn get_cost_field_value() {
		let mut cost_field = CostField::default();
		cost_field.set_grid_value(255, 9, 9);
		let result = cost_field.get_grid_value(9, 9);
		let actual: u8 = 255;
		assert_eq!(actual, result);
	}
	#[test]
	#[cfg(feature = "ron")]
	fn cost_field_file() {
		let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/cost_field.ron";
		let _cost_field = CostField::from_file(path);
		assert!(true)
	}
}
