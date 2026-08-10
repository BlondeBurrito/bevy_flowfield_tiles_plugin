//! Bresenham's line algorithm is for determining points on n-dimensional raster. Given a line across a grid of squares this algorithm tells you which squares lie on the line

use crate::v2::flowfields::fields::FieldCell;

/// When finding a shallow raster representation of a line we step through the x-dimension and increment y based on an error bound which indicates which cells lie on the line
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
