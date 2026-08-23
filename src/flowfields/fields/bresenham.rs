//! Bresenham's line algorithm is for determining points on n-dimensional raster. Given a line across a grid of squares this algorithm tells you which squares lie on the line
//!
//! [Useful doc](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm)

use crate::flowfields::fields::FieldCell;

/// When finding a shallow raster representation of a line we step through the x-dimension and increment y based on an error bound which indicates which cells lie on the line
///
/// This function is used when the y (row) delta is smaller than the x (column) delta
pub fn walk_bresenham_shallow(col_0: i32, row_0: i32, col_1: i32, row_1: i32) -> Vec<FieldCell> {
	let mut cells = Vec::new();

	let delta_col = col_1 - col_0;
	let mut delta_row = row_1 - row_0;

	let mut row_increment = 1;
	if delta_row < 0 {
		row_increment = -1;
		delta_row *= -1;
	}
	let mut difference = 2 * delta_row - delta_col;
	let mut row = row_0;

	for col in col_0..=col_1 {
		cells.push(FieldCell::new(col as usize, row as usize));
		if difference > 0 {
			row += row_increment;
			difference += 2 * (delta_row - delta_col);
		} else {
			difference += 2 * delta_row;
		}
	}
	cells
}
/// When finding a steep raster representation of a line we step through the y-dimension and increment x based on an error bound which indicates which cells lie on the line
///
/// This function is used when the y (row) delta is larger than the x (column) delta
pub fn walk_bresenham_steep(col_0: i32, row_0: i32, col_1: i32, row_1: i32) -> Vec<FieldCell> {
	let mut cells = Vec::new();

	let mut delta_col = col_1 - col_0;
	let delta_row = row_1 - row_0;

	let mut col_increment = 1;
	if delta_col < 0 {
		col_increment = -1;
		delta_col *= -1;
	}
	let mut difference = 2 * delta_col - delta_row;
	let mut col = col_0;

	for row in row_0..=row_1 {
		cells.push(FieldCell::new(col as usize, row as usize));
		if difference > 0 {
			col += col_increment;
			difference += 2 * (delta_col - delta_row);
		} else {
			difference += 2 * delta_col;
		}
	}
	cells
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;

	//  ___________________________
	// |                           |
	// |                           |
	// |                           |
	// |                           |
	// |(0,5)                      |
	// |                           |
	// |           (5, 7)          |
	// |                           |
	// |                           |
	// |___________________________|
	#[test]
	fn shallow() {
		let source_col = 5;
		let source_row = 7;
		let target_col = 0;
		let target_row = 5;

		let result = if source_col > target_col {
			let mut fields = walk_bresenham_shallow(target_col, target_row, source_col, source_row);
			// ensure list points in the direction of source to target
			fields.reverse();
			fields
		} else {
			walk_bresenham_shallow(source_col, source_row, target_col, target_row)
		};
		let actual = vec![
			FieldCell::new(5, 7),
			FieldCell::new(4, 7),
			FieldCell::new(3, 6),
			FieldCell::new(2, 6),
			FieldCell::new(1, 5),
			FieldCell::new(0, 5),
		];
		assert_eq!(actual, result);
	}

	//  ___________________________
	// |                           |
	// |         (3, 1)            |
	// |                           |
	// |                           |
	// |                           |
	// |                           |
	// |           (5, 7)          |
	// |                           |
	// |                           |
	// |___________________________|
	#[test]
	fn steep() {
		let source_col = 3;
		let source_row = 1;
		let target_col = 5;
		let target_row = 7;

		let result = if source_row > target_row {
			let mut fields = walk_bresenham_steep(target_col, target_row, source_col, source_row);
			fields.reverse();
			fields
		} else {
			walk_bresenham_steep(source_col, source_row, target_col, target_row)
		};
		let actual = vec![
			FieldCell::new(3, 1),
			FieldCell::new(3, 2),
			FieldCell::new(4, 3),
			FieldCell::new(4, 4),
			FieldCell::new(4, 5),
			FieldCell::new(5, 6),
			FieldCell::new(5, 7),
		];
		assert_eq!(actual, result);
	}
}
