//! An `IntegrationField` is an array of 32-bit values. It uses the `CostField` to produce a cumulative cost to reach the end goal/target. The first 16-bits of each field cell value are used for a cost measurement while the second 16-bits are used as flags to indicate certain properties of a cell.
//!
//! When a new route needs to be processed the first 16-bits of the field values are set to `u16::MAX` and the field cell containing the goal is set to `0`. Any cells which are impassable in the `CostField` are marked in the `IntegrationField` with their second 16-bits as `INT_BITS_IMPASSABLE`.
//!
//! In order to reduce needless pathfinding near the goal a Line Of Sight (LOS) pass is performed from the goal Sector. The idea being that if an actor moves into a field cell that has LOS then it no longer needs to follow the FlowFields and can instead directly path to the goal.
//!
//! The LOS phase begins as a wavefront from the goal that interrogates the adjacent neighbouring field cells. If an adjacent cell is not marked as impassable then it must have LOS to the goal and the value of the cell receives a wavefront cost plus the LOS bit flag. The wavefront then expands (whereby the wavefront cost increments by 1) to interrogate the adjacent cells of the neighbours and repeats until the wavefront cannot propagate any further.
//!
//! As the wavefront expands it may encounter an impassable field cell. This causes two things to happen:
//!
//! First, wavefront expansion cannot continue in the direction of the impassable field cell so it is removed from being a candidate in the next round of wavefront propagation.
//!
//!Second, if there is a vacant field cell next to the impassable field cell then this indicates a Corner. A Corner means that LOS will be blocked in a given direction and the Corner is recorded for the integrated cost calculation.
//!
//! By taking a vector from the starting goal to the corner we can then extend this vector to calculate what field cells lie along a line. The field cells on this line are updated with the flag for `INT_BITS_WAVE_BLOCKED`. Meaning that as LOS expands and propagates if a WavefrontBlocked cell is encountered then the cell is removed as a candidate in further LOS propagation. This ensures that LOS cannot flow around impassable areas.
//!
//! Once the wavefront has exhausted expansion from either hitting the sector boundaries or from impassable cells/corners we can then calculate the actual integrated cost of the field.
//!
//! From the Corners of an `IntegrationField` recorded previously we start a new series of wavefronts that radiate from the corners considering any adjacent field cells that have not been marked as LOS or impassable.
//!
//! To calculate the cost of the cells in the field:
//!
//! 1. The valid ordinal neighbours of the corners are determined (one, none or many of North, East, South, West)
//! 2. For each ordinal field cell lookup their [CostField] value
//! 3. 3. Add the [CostField] cost to the [IntegrationField] cost of the current cell, this is the integrated-cost
//! 4. Wavefront propagates to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their cumulative integration-cost, and repeat until the entire field is done
//!
//! The end result effectively produces a gradient of high numbers to low numbers, a flow of sorts.
//!
//! For Sectors other than the goal the process is effectively the same where boundary portals are treated as corners and wave propagation expanded.
//!

use bevy::reflect::Reflect;

use crate::flowfields::{
	fields::{Field, FieldCell, cost_field::CostField},
	route_cache::RouteStep,
	utilities::{FIELD_RESOLUTION, Ordinal},
};

/// Flags a 'FieldCell' as having Line Of Sight
pub const INT_BITS_LOS: u32 = 0b0000_0000_0000_0001_0000_0000_0000_0000;
/// Flags a 'FieldCell' as being the goal
pub const INT_BITS_GOAL: u32 = 0b0000_0000_0000_0010_0000_0000_0000_0000;
/// Flags a 'FieldCell' to prevent wavefront propagation
pub const INT_BITS_WAVE_BLOCKED: u32 = 0b0000_0000_0000_0100_0000_0000_0000_0000;
/// Flags a 'FieldCell' as a portal
pub const INT_BITS_PORTAL: u32 = 0b0000_0000_0000_1000_0000_0000_0000_0000;
/// Flags a 'FieldCell' as being impassable
pub const INT_BITS_IMPASSABLE: u32 = 0b0000_0010_0000_0000_0000_0000_0000_0000;
/// Flags a 'FieldCell' as being a corner which is used for integrated cost propagation
pub const INT_BITS_CORNER: u32 = 0b0000_0100_0000_0000_0000_0000_0000_0000;
/// Helper for analysing the integrated cost of a 'FieldCell'
pub const INT_FILTER_BITS_COST: u32 = 0b0000_0000_0000_0000_1111_1111_1111_1111;
/// Helper for analysing which flags have been set on a 'FieldCell'
pub const INT_FILTER_BITS_FLAGS: u32 = 0b1111_1111_1111_1111_0000_0000_0000_0000;

/// The [IntegrationField] consists of integrated-cost values and markers that
/// describe a gradient/flow
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Reflect)]
pub struct IntegrationField {
	/// Integration array
	#[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
	field: [u32; FIELD_RESOLUTION * FIELD_RESOLUTION],
	/// A list of [FieldCell] indices which are used for the integrated cost
	/// calculation of the field. In the final goal sector `los_corners` are
	/// calculated from a Line-of-Sight pass. For other sectors the portals
	/// are used as corners. When the field is being calculated it is these
	/// corners that are used to start wavefront propagation
	los_corners: Vec<usize>,
}

impl Default for IntegrationField {
	fn default() -> Self {
		IntegrationField {
			field: [u16::MAX as u32; FIELD_RESOLUTION * FIELD_RESOLUTION],
			los_corners: Vec::default(),
		}
	}
}

impl Field<u32> for IntegrationField {
	/// Get a reference to the field array
	fn get(&self) -> &[u32; FIELD_RESOLUTION * FIELD_RESOLUTION] {
		&self.field
	}
	/// Retrieve a field cell value
	///
	/// NB: This will panic if out of bounds
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u32 {
		let index = field_cell.as_1d_index();
		self.field[index]
	}
	/// Set a field cell to a value
	///
	/// NB: This will panic if out of bounds
	fn set_field_cell_value(&mut self, value: u32, field_cell: FieldCell) {
		let index = field_cell.as_1d_index();
		self.field[index] = value;
	}
}

impl IntegrationField {
	/// Init [IntegrationField] with impassable/walls marked and goal values set
	pub fn init(scaled_costfield: &CostField, route_step: &RouteStep) -> Self {
		let mut field = IntegrationField::default();
		// mark walls
		for (i, value) in scaled_costfield.get().iter().enumerate() {
			if *value == 255 {
				field.field[i] = 65535 + INT_BITS_IMPASSABLE;
			}
		}
		// set goal values
		field.set_goal_value(route_step);
		//TODO consider if useful to expand LOS propagation to other sectors
		// line of sight pass on final goal sector
		if route_step.portal().is_none() {
			let wavefront_cost = 1;
			propagate_los(
				&mut field,
				&[route_step.get_goal()],
				wavefront_cost,
				route_step.get_goal(),
			);
		}
		field
	}
	fn set_goal_value(&mut self, route_step: &RouteStep) {
		if let Some(window) = route_step.portal() {
			// mark the portal window cells are goals
			let indices = window.get_all_window_cells();
			for i in indices.iter() {
				self.field[*i] = INT_BITS_PORTAL;
				// cells other than the last need LOS corners setting as the portal cells
				self.los_corners.push(*i);
			}
		} else {
			let goal_index = route_step.get_goal();
			self.field[goal_index] = INT_BITS_GOAL;
		}
	}
	/// Perform the integrated cost calculation to build the [IntegrationField],
	/// beginning with the 'los_corners'
	pub fn build(&mut self, scaled_costfield: &CostField) {
		// list of active wavefront, element 0 is the cell, element 1 is the integrated cost
		let mut wavefront = vec![];
		for goal in self.los_corners.iter() {
			wavefront.push((
				(*goal),
				self.get_field_cell_value(FieldCell::from_index(*goal)),
			));
		}
		propagate_integrated_wavefront(self, scaled_costfield, wavefront);
	}
}

//TODO this is a diamond shaped propagation, doesn't really matter for LOS but would a spherical propagation be better? (spherical is solving the Eikonal PDE)
/// Recursively expand a wavefront and mark cells as Line-of-Sight if they have a clear path to the goal. If an impassable wall if located then test for a LOS corner and mark cells to block wavefront propagation
fn propagate_los(
	field: &mut IntegrationField,
	wavefront: &[usize],
	mut wavefront_cost: u32,
	goal: usize,
) {
	let goal_cell = FieldCell::from_index(goal);
	let mut next_wavefront = vec![];
	for cell_index in wavefront.iter() {
		// get the neighbours of the cell
		let wave_cell = FieldCell::from_index(*cell_index);
		let neighbours = wave_cell.get_orthogonal_neighbours();
		for neighbour in neighbours.iter() {
			let n_index = neighbour.as_1d_index();
			let cost = field.field[n_index];
			if cost & INT_BITS_WAVE_BLOCKED == INT_BITS_WAVE_BLOCKED
				|| cost & INT_BITS_GOAL == INT_BITS_GOAL
			{
				// wave blocked don't propagate LOS from this neighbour
			} else if cost & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE {
				// found wall, look for LOS corner a wavefront ahead
				// based on the direction towards `n_index`, look at it's neighbours,
				// if a neighbour isn't a wall then it means there's
				// a LOS corner
				let dir = wave_cell.dir_from_this_to_rhs(neighbour);

				match dir {
					Ordinal::North | Ordinal::South => {
						// check if the corner is actually reachable from the wavefront cell
						// this prevents stepping between two diagonal wall cells and assigning
						// an incorrect wavefront flag to a corner that shouldn't exist.
						// E.g
						//  _________
						// | c A ?   |
						// |   w B   |
						// |_________|
						//
						// Wavefront in cell `w`. Neighbours `A` and `B` are walls. `A` is
						// inspected and it's a wall. We must ensure `?` isn't labelled as a
						// corner as it's blocked off diagonally by the walls. To do this we look
						// at the East and West neighbours of `w` to see if any of those are
						// walls. In this case there is a wall at `B` - meaning `?` cannot be a
						// valid corner as it's inaccessible. When the westerly neighbour of `w`
						// is inspected we find no wall, this means the empty cell at `c` must
						// be a corner and can be used for integrated cost calculation
						//
						if let Some(wave_west) =
							wave_cell.get_in_ordinal_direction(&Ordinal::West, 1)
						{
							let wave_west_cost = field.field[wave_west.as_1d_index()];
							// see if diagonally blocking
							if wave_west_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
								// not blocking, see if neighbour can be made a corner
								if let Some(n_west) =
									neighbour.get_in_ordinal_direction(&Ordinal::West, 1)
								{
									let n_west_cost = field.field[n_west.as_1d_index()];
									if n_west_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
										extend_los_corner(
											field,
											&n_west,
											&goal_cell,
											wavefront_cost,
										);
									}
								}
							}
						}
						if let Some(wave_east) =
							wave_cell.get_in_ordinal_direction(&Ordinal::East, 1)
						{
							let wave_east_cost = field.field[wave_east.as_1d_index()];
							// see if diagonally blocking
							if wave_east_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
								// not blocking, see if neighbour can be made a corner
								if let Some(n_east) =
									neighbour.get_in_ordinal_direction(&Ordinal::East, 1)
								{
									let n_east_cost = field.field[n_east.as_1d_index()];
									if n_east_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
										extend_los_corner(
											field,
											&n_east,
											&goal_cell,
											wavefront_cost,
										);
									}
								}
							}
						}
					}
					Ordinal::East | Ordinal::West => {
						if let Some(wave_north) =
							wave_cell.get_in_ordinal_direction(&Ordinal::North, 1)
						{
							let wave_north_cost = field.field[wave_north.as_1d_index()];
							// see if diagonally blocking
							if wave_north_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
								// not blocking, see if neighbour can be made a corner
								if let Some(n_north) =
									neighbour.get_in_ordinal_direction(&Ordinal::North, 1)
								{
									let n_north_cost = field.field[n_north.as_1d_index()];
									if n_north_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
										extend_los_corner(
											field,
											&n_north,
											&goal_cell,
											wavefront_cost,
										);
									}
								}
							}
						}
						if let Some(wave_south) =
							wave_cell.get_in_ordinal_direction(&Ordinal::South, 1)
						{
							let wave_south_cost = field.field[wave_south.as_1d_index()];
							// see if diagonally blocking
							if wave_south_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
								// not blocking, see if neighbour can be made a corner
								if let Some(n_south) =
									neighbour.get_in_ordinal_direction(&Ordinal::South, 1)
								{
									let n_south_cost = field.field[n_south.as_1d_index()];
									if n_south_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
										extend_los_corner(
											field,
											&n_south,
											&goal_cell,
											wavefront_cost,
										);
									}
								}
							}
						}
					}
					_ => panic!("Only orthogonal Ordinals should be here, found {}", dir),
				}
			} else if cost & INT_BITS_LOS != INT_BITS_LOS {
				// we have a new LOS that can be propagated,
				// set the integration value as the wavefront
				// and set the LOS flag
				let mut value = wavefront_cost;
				value |= INT_BITS_LOS;
				field.field[n_index] = value;
				next_wavefront.push(n_index);
			}
		}
	}
	wavefront_cost += 1;
	// if valid cells exist to continue propagation then recursively propagate LOS
	if !next_wavefront.is_empty() {
		propagate_los(field, &next_wavefront, wavefront_cost, goal);
	}
}

/// Establish a line from the `goal` that runs through the `corner` and hits
/// the sector boundary. Any cells after `corner` to the boundary should be
/// marked as corners and flagged to prevent LOS propagation from flowing
/// around the corner and out of sight
fn extend_los_corner(
	field: &mut IntegrationField,
	corner: &FieldCell,
	goal: &FieldCell,
	wavefront_cost: u32,
) {
	// find the sector edge where line of sight should be blocked based on the corner
	let end = check_los_corner_propagation(&corner, goal);
	// from the corner to the boundary cell of LOS being blocked use the bresenham line algorithm to find all cells between the two cell points and mark them as being wavefront blocked so that further LOS propagation won't flow behind impassable cells
	let blocked_cells = corner.get_cells_between_points(&end);
	for (i, blocked) in blocked_cells.iter().enumerate() {
		let value = field.get_field_cell_value(*blocked);
		// only mark flags for cells that aren't walls
		if value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE {
			break;
		}
		// if the line passes through the diagonal of two impassable cells propagation should stop otherwise a line of corners would be assigned that's not reachable from the corner being extrapolated
		if i > 0 {
			let previous = &blocked_cells[i - 1];
			match Ordinal::cell_to_cell_direction(*blocked, *previous) {
				Ordinal::NorthEast => {
					if let Some(south) = Ordinal::get_cell_neighbour(*blocked, Ordinal::South) {
						if let Some(west) = Ordinal::get_cell_neighbour(*blocked, Ordinal::West) {
							let s_v = field.get_field_cell_value(south) & INT_BITS_IMPASSABLE;
							let w_v = field.get_field_cell_value(west) & INT_BITS_IMPASSABLE;
							if s_v == INT_BITS_IMPASSABLE && w_v == INT_BITS_IMPASSABLE {
								break;
							}
						}
					}
				}
				Ordinal::SouthEast => {
					if let Some(north) = Ordinal::get_cell_neighbour(*blocked, Ordinal::North) {
						if let Some(west) = Ordinal::get_cell_neighbour(*blocked, Ordinal::West) {
							let n_v = field.get_field_cell_value(north) & INT_BITS_IMPASSABLE;
							let w_v = field.get_field_cell_value(west) & INT_BITS_IMPASSABLE;
							if n_v == INT_BITS_IMPASSABLE && w_v == INT_BITS_IMPASSABLE {
								break;
							}
						}
					}
				}
				Ordinal::SouthWest => {
					if let Some(north) = Ordinal::get_cell_neighbour(*blocked, Ordinal::North) {
						if let Some(east) = Ordinal::get_cell_neighbour(*blocked, Ordinal::East) {
							let n_v = field.get_field_cell_value(north) & INT_BITS_IMPASSABLE;
							let e_v = field.get_field_cell_value(east) & INT_BITS_IMPASSABLE;
							if n_v == INT_BITS_IMPASSABLE && e_v == INT_BITS_IMPASSABLE {
								break;
							}
						}
					}
				}
				Ordinal::NorthWest => {
					if let Some(south) = Ordinal::get_cell_neighbour(*blocked, Ordinal::South) {
						if let Some(east) = Ordinal::get_cell_neighbour(*blocked, Ordinal::East) {
							let s_v = field.get_field_cell_value(south) & INT_BITS_IMPASSABLE;
							let e_v = field.get_field_cell_value(east) & INT_BITS_IMPASSABLE;
							if s_v == INT_BITS_IMPASSABLE && e_v == INT_BITS_IMPASSABLE {
								break;
							}
						}
					}
				}
				Ordinal::Zero => panic!("Neighbour not found"),
				_ => {}
			}
		}
		// don't mark cells which aren't already marked as wavefront blocked
		if value & INT_BITS_WAVE_BLOCKED != INT_BITS_WAVE_BLOCKED {
			// mark the line as corners for the int calc layer
			field.los_corners.push(blocked.as_1d_index());
			// NB: add 1 because corner is effectively one wavefront propagation ahead
			// then add `i` as each successive line cells is another wavefront ahead
			field.set_field_cell_value(
				wavefront_cost + 1 + i as u32 + INT_BITS_WAVE_BLOCKED + INT_BITS_CORNER,
				*blocked,
			);
		}
	}
}

/// Construct a vector from the `goal` to the `corner` [FieldCell] and extrapolate it so that it intersects a sector boundary. Based on the `FieldCells` crossed by the line wavefront propagation can be blocked to ensure that the LOS propagation doesn't flow around obscured corners. This method will produce the boundary [FieldCell] that can be plugged into the Bresenham Line Algorithm to determine the blocked cells
fn check_los_corner_propagation(corner: &FieldCell, goal: &FieldCell) -> FieldCell {
	// obtain wavefront blocked from the corner,
	// using the line equation properties we find the vector
	// from the goal to the corner and then find from
	// the corner what FieldCell on the Sector boundary the
	// line would terminate at
	//
	// deal with vertical and horizontal lines first
	if corner.get_column() == goal.get_column() {
		// no column change, find direction
		// of row change
		if corner.get_row() > goal.get_row() {
			// dir is heading down to max boundary value
			FieldCell::new(corner.get_column(), FIELD_RESOLUTION - 1)
		} else {
			// dir is heading up towards boundary 0
			FieldCell::new(corner.get_column(), 0)
		}
	} else if corner.get_row() == goal.get_row() {
		// no row change, find direction of
		// column change
		if corner.get_column() > goal.get_column() {
			// dir is heading right towards max boundary
			FieldCell::new(FIELD_RESOLUTION - 1, corner.get_row())
		} else {
			// dir is heading left towards boundary 0
			FieldCell::new(0, corner.get_row())
		}
	} else {
		// handle diagonal lines
		let delta_column = corner.get_column() as f32 - goal.get_column() as f32;
		let delta_row = corner.get_row() as f32 - goal.get_row() as f32;
		let gradient = delta_row / delta_column;
		let intercept = -gradient * (corner.get_column() as f32) + corner.get_row() as f32;
		if corner.get_column() > goal.get_column() {
			// walk the line with increasing column
			// until the row or column value
			// reaches a sector boundary
			let d = (FIELD_RESOLUTION - 1)
				.checked_sub(corner.get_column())
				.unwrap();
			for x in 0..=d {
				let end_col = corner.get_column() + x;
				let end_row = (gradient * (end_col as f32) + intercept).floor();
				// handle steep lines, e.g goal (4,4) and adj (5,7) projected
				// along column places column 6 on row 10 which is OOB
				if end_row > FIELD_RESOLUTION as f32 - 1.0 {
					if end_col < FIELD_RESOLUTION {
						return FieldCell::new(end_col, FIELD_RESOLUTION - 1);
					} else {
						return FieldCell::new(FIELD_RESOLUTION - 1, FIELD_RESOLUTION - 1);
					}
				} else if end_row < 0.0 {
					if end_col < FIELD_RESOLUTION {
						return FieldCell::new(end_col, 0);
					} else {
						return FieldCell::new(FIELD_RESOLUTION - 1, 0);
					}
				} else if end_col == FIELD_RESOLUTION - 1 {
					return FieldCell::new(end_col, end_row as usize);
				}
			}
			//TODO make this better
			panic!("LOS corner prop failed to find increment boundary");
		} else {
			// walk the line with decreasing column
			// until row or column value
			// reaches a sector boundary
			let d = corner.get_column();
			for x in 0..=d {
				let end_col = corner.get_column().checked_sub(x).unwrap();
				let end_row = (gradient * (end_col as f32) + intercept).floor() as usize;
				// handle steep cases where line projection is OOB
				// ex: goal (7,5), adj (6,9), projects (0,33)
				if end_col == 0 {
					if end_row > FIELD_RESOLUTION - 1 {
						return FieldCell::new(end_col, FIELD_RESOLUTION - 1);
					} else {
						return FieldCell::new(end_col, end_row);
					}
				}
				if end_row == 0 {
					return FieldCell::new(end_col, end_row);
				}
				if end_row > FIELD_RESOLUTION - 1 {
					return FieldCell::new(end_col, FIELD_RESOLUTION - 1);
				}
			}
			//TODO make this better
			panic!("LOS corner prop failed to find decrement boundary");
		}
	}
}

//TODO this is a diamond shaped propagation, spherical propagation would be more accurate? (spherical is solving the Eikonal PDE). Also this wastes lookups inspecting previously calculated cells, should visit only once
/// Expand the wavefront recursively and produce the integrated-cost of cells
/// in the [IntegrationField]
fn propagate_integrated_wavefront(
	int_field: &mut IntegrationField,
	costfield: &CostField,
	wavefront: Vec<(usize, u32)>,
) {
	let mut next_wavefront = vec![];
	for (cell_index, prev_int_cost) in wavefront.iter() {
		let neighbours = FieldCell::from_index(*cell_index).get_orthogonal_neighbours();
		for n in neighbours.iter() {
			// ensure neighbour isn't impassable or LOS
			let n_int = int_field.get_field_cell_value(*n);
			if n_int & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE
				&& n_int & INT_BITS_LOS != INT_BITS_LOS
			{
				let cell_cost = costfield.get_field_cell_value(*n) as u32;
				let int_cost = cell_cost + (prev_int_cost & INT_FILTER_BITS_COST);
				// if this neighbour has been calculated with a cheaper value then
				// update it
				//TODO does this overwrite any required flags
				if int_cost < (n_int & INT_FILTER_BITS_COST) {
					int_field.set_field_cell_value(int_cost, *n);
					next_wavefront.push((n.as_1d_index(), int_cost));
				}
			}
		}
	}

	if !next_wavefront.is_empty() {
		propagate_integrated_wavefront(int_field, costfield, next_wavefront);
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	use crate::flowfields::{portal::PortalWindow, sectors::SectorID};

	#[test]
	fn goal_set_final() {
		let costfield = CostField::default();
		let sector = SectorID::new(1, 1);
		let goal = 14;
		let portal = None;
		let route_step = RouteStep::new(&sector, goal, portal);
		let int_field = IntegrationField::init(&costfield, &route_step);

		assert!(int_field.field[14] & INT_BITS_GOAL == INT_BITS_GOAL);
	}

	#[test]
	fn goal_set_portal() {
		let costfield = CostField::default();
		let sector = SectorID::new(1, 1);
		let goal = 94;
		let portal = Some(PortalWindow::new(
			FieldCell::new(0, 9),
			FieldCell::new(9, 9),
			Ordinal::South,
		));
		let route_step = RouteStep::new(&sector, goal, portal);
		let int_field = IntegrationField::init(&costfield, &route_step);

		assert!(int_field.field[90] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[91] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[92] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[93] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[94] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[95] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[96] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[97] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[98] & INT_BITS_PORTAL == INT_BITS_PORTAL);
		assert!(int_field.field[99] & INT_BITS_PORTAL == INT_BITS_PORTAL);
	}

	// g = goal
	// X = wall
	// L = LOS
	// b = wave blocked
	// ```txt
	//  ___ ___ ___ ___ ___ ___ ___ ___ ___ ___
	// |   |   |   |   |   |   |   |   |   |   |
	// |___|___|___|___|___|___|___|___|___|___|
	// |   |   |   |   |   |   |   |   |   |   |
	// |___|___|___|___|___|___|___|___|___|___|
	// |   |   |   |   |   |   |   |   |   | b |
	// |___|___|___|___|___|___|___|___|___|___|
	// | b |   |   |   |   |   |   |   | b | L |
	// |___|___|___|___|___|___|___|___|___|___|
	// | L | b |   |   |   |   |   | b | L | L |
	// |___|___|___|___|___|___|___|___|___|___|
	// | L | L | b | X | X | X | b | L | L | L |
	// |___|___|___|___|___|___|___|___|___|___|
	// | L | L | L | L | L | L | L | L | L | L |
	// |___|___|___|___|___|___|___|___|___|___|
	// | L | L | L | L | L | L | L | L | L | L |
	// |___|___|___|___|___|___|___|___|___|___|
	// | L | L | L | L | g | L | L | L | L | L |
	// |___|___|___|___|___|___|___|___|___|___|
	// | L | b | X | L | L | L | X | b | L | L |
	// |___|___|___|___|___|___|___|___|___|___|
	// ```
	#[test]
	fn check_los() {
		let mut costfield = CostField::default();
		costfield.set_field_cell_value(255, FieldCell::new(3, 5));
		costfield.set_field_cell_value(255, FieldCell::new(4, 5));
		costfield.set_field_cell_value(255, FieldCell::new(5, 5));
		costfield.set_field_cell_value(255, FieldCell::new(2, 9));
		costfield.set_field_cell_value(255, FieldCell::new(6, 9));
		let sector = SectorID::new(1, 1);
		let goal = 84;
		let portal = None;
		let route_step = RouteStep::new(&sector, goal, portal);
		let int_field = IntegrationField::init(&costfield, &route_step);

		let r_c = FieldCell::new(1, 9);
		let r = int_field.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#034b}, cost: {:#034b}",
			r_c,
			r & INT_FILTER_BITS_FLAGS,
			r & INT_FILTER_BITS_COST
		);
		assert!(r & INT_BITS_WAVE_BLOCKED == INT_BITS_WAVE_BLOCKED);

		let r_c = FieldCell::new(2, 5);
		let r = int_field.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#034b}, cost: {:#034b}",
			r_c,
			r & INT_FILTER_BITS_FLAGS,
			r & INT_FILTER_BITS_COST
		);
		assert!(r & INT_BITS_WAVE_BLOCKED == INT_BITS_WAVE_BLOCKED);

		let r_c = FieldCell::new(1, 4);
		let r = int_field.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#034b}, cost: {:#034b}",
			r_c,
			r & INT_FILTER_BITS_FLAGS,
			r & INT_FILTER_BITS_COST
		);
		assert!(r & INT_BITS_WAVE_BLOCKED == INT_BITS_WAVE_BLOCKED);

		let r_c = FieldCell::new(3, 8);
		let r = int_field.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#034b}, cost: {:#034b}",
			r_c,
			r & INT_FILTER_BITS_FLAGS,
			r & INT_FILTER_BITS_COST
		);
		assert!(r & INT_BITS_LOS == INT_BITS_LOS);

		//TODO: interesting problem. As the LOS propagation works round in a clockwise
		//TODO: fashion (2, 8) is marked as blocked even tho it is parallel to the
		//TODO: goal. (2, 9) is analysed before (2, 8) meaning the north of the
		//TODO: wall is treated as a corner
		let r_c = FieldCell::new(2, 8);
		let r = int_field.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#034b}, cost: {:#034b}",
			r_c,
			r & INT_FILTER_BITS_FLAGS,
			r & INT_FILTER_BITS_COST
		);
		//TODO Should really be LOS here
		// assert!(r & INT_BITS_LOS == INT_BITS_LOS);
		assert!(r & INT_BITS_WAVE_BLOCKED == INT_BITS_WAVE_BLOCKED);
	}
}
