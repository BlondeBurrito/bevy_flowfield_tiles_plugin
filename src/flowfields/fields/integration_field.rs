//! An `IntegrationField` is an `MxN` 2D array of 32-bit values. It uses the `CostField` to produce a cumulative cost to reach the end goal/target. The first 16-bits of each field cell value are used for a cost measurement while the second 16-bits are used as flags to indicate certain properties of a cell.
//!
//! When a new route needs to be processed the first 16-bits of the field values are set to `u16::MAX` and the field cell containing the goal is set to `0`. Any cells which are impassable in the `CostField` are marked in the `IntegrationField` with their second 16-bits as `INT_BITS_IMPASSABLE`.
//!
//! In order to reduce needless pathfinding near the goal a Line Of Sight (LOS) pass is performed from the goal Sector. The idea being that if an actor moves into a field cell that has LOS then it no longer needs to follow the FlowFields and can instead directly path to the goal.
//!
//! The LOS phase begins as a wavefront from the goal that interrogates the adjacent neighbouring field cells. If an adjacent cell is not marked as impasssable then it must have LOS to the goal and the value of the cell receives a wavefront cost plus the LOS bit flag. The wavefront then expands (whereby the wavefront cost increments by 1) to interrogate the adjacent cells of the neighbours and repeats until the wavefront cannot propagate any further.
//!
//! As the wavefront expands it may encounter an impassable field cell. This causes two things to happen:
//!
//! First, wavefront expansion cannot continue in the direction of the impassable field cell so it is removed from being a candidate in the next round of wavefront propagation.
//!
//!Second, if there is a vacant field cell next to the impassable field cell then this indicates a Corner. A Corner means that LOS will be blocked in a given direction and the Corner is recorded for the integrated cost calculation.
//!
//! By taking a vector from the starting goal to the corner we can then extend this vector to calculate what field cells lie along a line. The field cells on this line are updated with the flag for `INT_BITS_WAVE_BLOCKED`. Meaning that as LOS expands and propagates if a WavefrontBlocked cell is encountered then the cell is removed as a candidate in further LOS porpagation. This ensures that LOS cannot flow around impassable areas.
//!
//! Once the wavefront has exhausted expansion from either hitting the sector boundaries or from impassable cells/corners we can then calculate the actual integrated cost of the field.
//!
//! From the Corners of an `IntegrationField` recorded previously we start a new series of wavefronts that radiate from the corners considering any adjacent field cells that have not been marked as LOS or impassable.
//!
//! To calculate the cost of the cells in the field:
//!
//! 1. The valid ordinal neighbours of the corners are determined (one, none or many of North, East, South, West)
//! 2. For each ordinal field cell lookup their `CostField` value
//! 3. 3. Add the `CostField` cost to the `IntegrationFields` cost of the current cell (at the corner the wavefront cost assigned was 4, assuming the `CostField` value of the adjacent cell is `1` then the integrated cost becomes `5`)
//! 4. Wavefront propagates to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their cumulative integration cost, and repeat until the entire field is done
//!
//! The end result effectively produces a gradient of high numbers to low numbers, a flow of sorts.
//!
//! For Sectors other than the goal the process is effectively the same where boundary portals are treated as corners and wave propagation exapaned.
//!

use crate::prelude::*;

/// Grouping of high-level route from goal to actor where the integration
/// fields get populated when the builder arrives at the front of the queue
#[derive(Default)]
pub struct IntegrationBuilder {
	//TODO try avoiding allocating path here and within int_fields
	/// Sector and Portals/goals describing the route from the target goal to the
	/// origin sector of the actor
	path: Route,
	/// List of [IntegrationField] aligned with Sector and Goals whereby the
	/// `integration_fields` is initially blank and gets built over passes
	integration_fields: Vec<(SectorID, Vec<FieldCell>, IntegrationField)>,
	/// Have the Portals been expanded to produce additional goals along sector boundaries
	has_expanded_portals: bool,
	/// Has a Line Of Sight pass been performed over the fields
	has_los_pass: bool,
	/// Has the integration cost of the fields been calculated
	has_cost_pass: bool,
}

impl IntegrationBuilder {
	/// Create a new instance [IntegrationBuilder] initialised with a `path`
	pub fn new(path: Route, cost_fields: &SectorCostFields) -> Self {
		let mut int_fields = Vec::with_capacity(path.get().len());
		for (sector, goal) in path.get().iter() {
			let cost = cost_fields.get_scaled().get(sector).unwrap();
			int_fields.push((*sector, Vec::new(), IntegrationField::new(goal, cost)));
		}
		IntegrationBuilder {
			path,
			integration_fields: int_fields,
			has_expanded_portals: false,
			has_los_pass: false,
			has_cost_pass: false,
		}
	}
	/// Get the series of sectors and connecting portals of the path
	pub fn get_route(&self) -> &Route {
		&self.path
	}
	/// Get the list of fields
	pub fn get_integration_fields(&self) -> &Vec<(SectorID, Vec<FieldCell>, IntegrationField)> {
		&self.integration_fields
	}
	/// Get the list of fields
	pub fn get_mut_integration_fields(
		&mut self,
	) -> &mut Vec<(SectorID, Vec<FieldCell>, IntegrationField)> {
		&mut self.integration_fields
	}
	/// Indicates whether Portals have been expanded for the fields
	pub fn has_expanded_portals(&self) -> bool {
		self.has_expanded_portals
	}
	/// Sets that Portals have been expanded across the fields
	pub fn set_expanded_portals(&mut self) {
		self.has_expanded_portals = true;
	}
	/// Indicates whether Line Of Sight has been calculated across the fields
	pub fn has_los_pass(&self) -> bool {
		self.has_los_pass
	}
	/// Sets that Line Of Sight calculations have been completed
	pub fn set_los_pass(&mut self) {
		self.has_los_pass = true;
	}
	/// Indicates whether integration costs have been computed across the
	/// fields. If so then the FlowFields can be computed from them
	pub fn has_cost_pass(&self) -> bool {
		self.has_cost_pass
	}
	/// Sets that integrated costs have been calculated across the fields
	pub fn set_cost_pass(&mut self) {
		self.has_cost_pass = true;
	}
	/// Portals may represent multiple [FieldCell]s along a boundary, expand
	/// them within the IntegrationFields to provide multiple goal [FieldCell]s
	/// for crossing from one sector to another
	pub fn expand_field_portals(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields_scaled: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) {
		for (i, (sector_id, goals, field)) in self.integration_fields.iter_mut().enumerate() {
			// first element is always the end target, don't bother with portal expansion,
			// just store the single end goal in the list
			if i == 0 {
				goals.push(self.path.get()[i].1);
				field.set_field_cell_value(INT_BITS_GOAL, self.path.get()[i].1);
			} else {
				// portals represent the boundary to another sector, a portal can be spread over
				// multple field cells, expand the portal to provide multiple goal
				// targets for moving to another sector
				let neighbour_sector_id = self.path.get()[i - 1].0;
				let expanded_goals = sector_portals
					.get()
					.get(sector_id)
					.unwrap()
					.expand_portal_into_goals(
						sector_cost_fields_scaled,
						sector_id,
						&self.path.get()[i].1, // portal
						&neighbour_sector_id,
						map_dimensions,
					);
				for g in expanded_goals.iter() {
					// set the goals of the expanded portal, value and the bit flag
					goals.push(*g);
					field.set_field_cell_value(INT_BITS_PORTAL, *g)
				}
			}
		}
	}
	/// From the target goal perform a Line Of Sight pass in an expanding
	/// wavefront to mark any `FieldCell` that can see the goal with the LOS
	/// flag and mark any LOS corners that can be expanded in the integration
	/// cost layer
	pub fn calculate_los(&mut self) {
		let fields = self.get_mut_integration_fields();
		if let Some((_sector, goals, field)) = fields.first_mut() {
			field.set_initial_los(goals[0]);
			field.calculate_sector_goal_los(goals, &goals[0]);
		}
		//TODO propagate LOS across sectors
		//until then set LOS corners in other sectors as the goals (this is
		// portal goals) for int calc layer
		for (_sector, goals, field) in fields.iter_mut() {
			if field.los_corners.is_empty() {
				for g in goals {
					field.add_los_corner(*g);
				}
			}
		}
	}
	/// From identified LOS corners calcualte the integrated cost of unmarked `FieldCell`
	pub fn build_integrated_cost(&mut self, cost_fields: &SectorCostFields) {
		for (sector_id, _goals, int_field) in self.get_mut_integration_fields() {
			let cost_field = cost_fields.get_scaled().get(sector_id).unwrap();
			//TODO explain using los corners
			int_field.calculate_field(cost_field);
		}
	}
}

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

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
pub struct IntegrationField {
	/// Integration array
	field: [[u32; FIELD_RESOLUTION]; FIELD_RESOLUTION],
	/// A list of [FieldCell] which are used for the integrated cost
	/// calculation of the field
	los_corners: Vec<FieldCell>,
}

impl Default for IntegrationField {
	fn default() -> Self {
		IntegrationField {
			field: [[u16::MAX as u32; FIELD_RESOLUTION]; FIELD_RESOLUTION],
			los_corners: Vec::default(),
		}
	}
}

impl Field<u32> for IntegrationField {
	/// Get a reference to the field array
	fn get(&self) -> &[[u32; FIELD_RESOLUTION]; FIELD_RESOLUTION] {
		&self.field
	}
	/// Retrieve a field cell value
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u32 {
		self.field[field_cell.get_column()][field_cell.get_row()]
	}
	/// Set a field cell to a value
	fn set_field_cell_value(&mut self, value: u32, field_cell: FieldCell) {
		self.field[field_cell.get_column()][field_cell.get_row()] = value;
	}
}
impl IntegrationField {
	/// Creates a new [IntegrationField] where all cells are set to `u16::MAX`
	/// and impassable cells are set to include a bit flag  and while the goal
	/// is set to `0`
	pub fn new(goal: &FieldCell, cost: &CostField) -> Self {
		let mut field = IntegrationField::default();
		for (column, rows) in cost.get().iter().enumerate() {
			for (row, value) in rows.iter().enumerate() {
				if *value == u8::MAX {
					field.set_field_cell_value(
						65535 + INT_BITS_IMPASSABLE,
						FieldCell::new(column, row),
					);
				}
			}
		}
		field.set_field_cell_value(INT_BITS_GOAL, *goal);
		field
	}
	/// Sets the goal (not any portals) of the target sector as having Line Of Sight
	pub fn set_initial_los(&mut self, cell_id: FieldCell) {
		self.set_field_cell_value(INT_BITS_LOS, cell_id);
	}
	/// Append a new Line Of Sight corner to the integration field
	pub fn add_los_corner(&mut self, corner: FieldCell) {
		self.los_corners.push(corner);
	}
	/// From the goal of the target sector calcualte LOS
	pub fn calculate_sector_goal_los(&mut self, active_wavefront: &[FieldCell], goal: &FieldCell) {
		let wavefront_cost = 1;
		propagate_los(self, active_wavefront, wavefront_cost, goal);
	}

	//TODO: diamond like propagation and wasted extra lookups looking at previously calcualted neighbours, try fast marching method of solving Eikonal PDE for a spherical approx that visits each cell once
	/// From a list of Corners field cells iterate over successive neighbouring
	/// cells and calculate the integrated-cost field values from the
	/// `cost_field`
	pub fn calculate_field(&mut self, cost_field: &CostField) {
		// further positions to process, tuple element 0 is the position, element 1 is the integration cost from the previous cell needed to help calculate element 0s cost
		let mut queue: Vec<(FieldCell, u32)> = Vec::new();
		for goal in self.los_corners.iter() {
			queue.push(((*goal), self.get_field_cell_value(*goal)));
		}
		process_neighbours(self, queue, cost_field);
	}
}
//TODO how woudl portals work with a goal
/// From an `active_wavefront` peek at neighbouring cells to determine which
/// [FieldCell] have Line Of Sight to the `goal`. This method is recursive
/// until LOS ends due to sector boundaries or impassable areas
fn propagate_los(
	field: &mut IntegrationField,
	active_wavefront: &[FieldCell],
	mut wavefront_cost: u32,
	goal: &FieldCell,
) {
	let mut moved_wavefront: Vec<FieldCell> = Vec::new();
	for wavefront in active_wavefront.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*wavefront);
		for n in neighbours.iter() {
			let cost = field.get_field_cell_value(*n);
			if cost & INT_BITS_WAVE_BLOCKED == INT_BITS_WAVE_BLOCKED
				|| cost & INT_BITS_GOAL == INT_BITS_GOAL
			{
				// wave blocked don't propagate LOS from this neighbour
			} else if cost & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE {
				// found impassable, look for LOS corner
				// based on the direction towards `n`, look at n's neighbours,
				// if a neighbour isn't impassable then it means there's
				// a LOS corner
				let dir = Ordinal::cell_to_cell_direction(*n, *wavefront);
				// depending on direction get the cells next to `n`
				match dir {
					Ordinal::North | Ordinal::South => {
						// check if the corner is actually reachable from the neighbiouring wavefront cell
						// this prevents stepping between two diagonal impassable cells
						// and assinging an incorrect wavefront cost to a corner that shouldn't exist
						if let Some(west) = Ordinal::get_cell_neighbour(*wavefront, Ordinal::West) {
							let west_cost = field.get_field_cell_value(west);
							if west_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
								extend_los_corner(field, n, Ordinal::West, goal, wavefront_cost);
							}
						}
						if let Some(east) = Ordinal::get_cell_neighbour(*wavefront, Ordinal::East) {
							let east_cost = field.get_field_cell_value(east);
							if east_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
								extend_los_corner(field, n, Ordinal::East, goal, wavefront_cost);
							}
						}
					}
					Ordinal::East | Ordinal::West => {
						// check if the corner is actually reachable from the neighbiouring wavefront cell
						// this prevents stepping between two diagonal impassable cells
						// and assinging an incorrect wavefront cost to a corner that shouldn't exist
						if let Some(north) = Ordinal::get_cell_neighbour(*wavefront, Ordinal::North)
						{
							let north_cost = field.get_field_cell_value(north);
							if north_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
								extend_los_corner(field, n, Ordinal::North, goal, wavefront_cost);
							}
						}
						if let Some(south) = Ordinal::get_cell_neighbour(*wavefront, Ordinal::South)
						{
							let south_cost = field.get_field_cell_value(south);
							if south_cost & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
								extend_los_corner(field, n, Ordinal::South, goal, wavefront_cost);
							}
						}
					}
					_ => {
						panic!("Dir should only be orthogonal")
					}
				}
			} else if cost & INT_BITS_LOS != INT_BITS_LOS {
				// we have a new LOS that can be propagated
				let mut value = wavefront_cost;
				value |= INT_BITS_LOS;
				field.set_field_cell_value(value, *n);
				moved_wavefront.push(*n);
			}
		}
	}
	wavefront_cost += 1;
	// if valid cells exist to continue propagation then recursively propagate LOS
	if !moved_wavefront.is_empty() {
		propagate_los(field, &moved_wavefront, wavefront_cost, goal);
	}
}
/// From a Line Of Sight corner extrapolate a line from the goal to the corner
/// and through to the sector boundary. For any FieldCell between the corner
/// and the boundary that lie on the line mark them with the WavefrontBlocked
/// flag, this prevents further LOS passes from reaching areas that are out of
/// LOS
fn extend_los_corner(
	field: &mut IntegrationField,
	neighbour: &FieldCell,
	ord: Ordinal,
	goal: &FieldCell,
	wavefront_cost: u32,
) {
	if let Some(adj) = Ordinal::get_cell_neighbour(*neighbour, ord) {
		let value = field.get_field_cell_value(adj);
		if value & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE {
			// find the sector edge where line fo sight should be blocked based on the corner
			let end = check_los_corner_propagation(&adj, goal);
			// from the corner to the boundary cell of LOS being blocked use the bresenham line algorithm to find all cells between the two cell points and mark them as being wavefront blocked so that further LOS propagation won't flow behind impassable cells
			let blocked_cells = adj.get_cells_between_points(&end);
			for (i, blocked) in blocked_cells.iter().enumerate() {
				let value = field.get_field_cell_value(*blocked);
				// only mark flags for cells that aren't impassable and which aren't already marked as wavefront blocked
				if value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE {
					break;
				}
				// if the line passes through the diagonal of two impassable cells propagation should stop otherwise a line of corners would be assigned that's not reachable from the corner being extrapolated
				if i > 0 {
					let previous = &blocked_cells[i - 1];
					match Ordinal::cell_to_cell_direction(*blocked, *previous) {
						Ordinal::NorthEast => {
							if let Some(south) =
								Ordinal::get_cell_neighbour(*blocked, Ordinal::South)
							{
								if let Some(west) =
									Ordinal::get_cell_neighbour(*blocked, Ordinal::West)
								{
									let s_v =
										field.get_field_cell_value(south) & INT_BITS_IMPASSABLE;
									let w_v =
										field.get_field_cell_value(west) & INT_BITS_IMPASSABLE;
									if s_v == INT_BITS_IMPASSABLE && w_v == INT_BITS_IMPASSABLE {
										break;
									}
								}
							}
						}
						Ordinal::SouthEast => {
							if let Some(north) =
								Ordinal::get_cell_neighbour(*blocked, Ordinal::North)
							{
								if let Some(west) =
									Ordinal::get_cell_neighbour(*blocked, Ordinal::West)
								{
									let n_v =
										field.get_field_cell_value(north) & INT_BITS_IMPASSABLE;
									let w_v =
										field.get_field_cell_value(west) & INT_BITS_IMPASSABLE;
									if n_v == INT_BITS_IMPASSABLE && w_v == INT_BITS_IMPASSABLE {
										break;
									}
								}
							}
						}
						Ordinal::SouthWest => {
							if let Some(north) =
								Ordinal::get_cell_neighbour(*blocked, Ordinal::North)
							{
								if let Some(east) =
									Ordinal::get_cell_neighbour(*blocked, Ordinal::East)
								{
									let n_v =
										field.get_field_cell_value(north) & INT_BITS_IMPASSABLE;
									let e_v =
										field.get_field_cell_value(east) & INT_BITS_IMPASSABLE;
									if n_v == INT_BITS_IMPASSABLE && e_v == INT_BITS_IMPASSABLE {
										break;
									}
								}
							}
						}
						Ordinal::NorthWest => {
							if let Some(south) =
								Ordinal::get_cell_neighbour(*blocked, Ordinal::South)
							{
								if let Some(east) =
									Ordinal::get_cell_neighbour(*blocked, Ordinal::East)
								{
									let s_v =
										field.get_field_cell_value(south) & INT_BITS_IMPASSABLE;
									let e_v =
										field.get_field_cell_value(east) & INT_BITS_IMPASSABLE;
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
				if value & INT_BITS_WAVE_BLOCKED != INT_BITS_WAVE_BLOCKED {
					// mark the line as corners for the int calc layer
					field.add_los_corner(*blocked);
					// NB: add 1 because adj is effectively one wavefront propagation ahead
					// then add `i` as each successive line cells is another wavefront ahead
					field.set_field_cell_value(
						wavefront_cost + 1 + i as u32 + INT_BITS_WAVE_BLOCKED + INT_BITS_CORNER,
						*blocked,
					);
				}
			}
		}
	}
}
/// Construct a vector from the `goal` to the `adjacent` (corner) [FieldCell] and extrapolate it so that it intersects a sector boundary. Based on the `FieldCells` crossed by the line wavefront propagation can be blocked to ensure that the LOS propagation doesn't flow around obscured corners. This method will produce the boundary [FieldCell] that can be plugged into the Breshenham Line Algorithm to determine the blocked cells
fn check_los_corner_propagation(adj: &FieldCell, goal: &FieldCell) -> FieldCell {
	// obtain wavefront blocked from the corner,
	// using the line equation properties we find the vector
	// from the goal to the corner and then find from
	// the corner what FieldCell on the Sector boundary the
	// line would terminate at
	//
	// deal with vertical and horizontal lines first
	if adj.get_column() == goal.get_column() {
		// no column change, find direction
		// of y change
		if adj.get_row() > goal.get_row() {
			// dir is heading down to max boundary value
			FieldCell::new(adj.get_column(), FIELD_RESOLUTION - 1)
		} else {
			// dir is heading up towards boundary 0
			FieldCell::new(adj.get_column(), 0)
		}
	} else if adj.get_row() == goal.get_row() {
		// no row change, find direction of
		// x change
		if adj.get_column() > goal.get_column() {
			// dir is heading right towards max boundary
			FieldCell::new(FIELD_RESOLUTION - 1, adj.get_row())
		} else {
			// dir is heading left towards boundary 0
			FieldCell::new(0, adj.get_row())
		}
	} else {
		// handle diagonal lines
		let delta_column = adj.get_column() as f32 - goal.get_column() as f32;
		let delta_row = adj.get_row() as f32 - goal.get_row() as f32;
		let gradient = delta_row / delta_column;
		let intercept = -gradient * (adj.get_column() as f32) + adj.get_row() as f32;
		let mut exists = None;
		if adj.get_column() > goal.get_column() {
			// walk the line with increasing column
			// until the row or column value
			// reaches a sector boundary
			let d = (FIELD_RESOLUTION - 1)
				.checked_sub(adj.get_column())
				.unwrap();
			for x in 0..=d {
				let end_col = adj.get_column() + x;
				let end_row = (gradient * (end_col as f32) + intercept).floor();
				// handle steep lines, e.g goal (4,4) and adj (5,7) projected
				// along column places column 6 on row 10 which is OOB
				if end_row > FIELD_RESOLUTION as f32 - 1.0 {
					if end_col < FIELD_RESOLUTION {
						exists = Some(FieldCell::new(end_col, FIELD_RESOLUTION - 1));
						break;
					} else {
						exists = Some(FieldCell::new(FIELD_RESOLUTION - 1, FIELD_RESOLUTION - 1));
						break;
					}
				} else if end_row < 0.0 {
					if end_col < FIELD_RESOLUTION {
						exists = Some(FieldCell::new(end_col, 0));
						break;
					} else {
						exists = Some(FieldCell::new(FIELD_RESOLUTION - 1, 0));
						break;
					}
				} else if end_col == FIELD_RESOLUTION - 1 {
					exists = Some(FieldCell::new(end_col, end_row as usize));
					break;
				}
			}
			if let Some(end) = exists {
				end
			} else {
				//TODO make this better
				panic!("LOS corner prop failed to find increment boundary");
			}
		} else {
			// walk the line with decreasing column
			// until row or column value
			// reaches a sector boundary
			let d = adj.get_column();
			for x in 0..=d {
				let end_col = adj.get_column().checked_sub(x).unwrap();
				let end_row = (gradient * (end_col as f32) + intercept).floor() as usize;
				// handle steep cases where line projection is OOB
				// ex: goal (7,5), adj (6,9), projects (0,33)
				if end_col == 0 {
					if end_row > FIELD_RESOLUTION - 1 {
						exists = Some(FieldCell::new(end_col, FIELD_RESOLUTION - 1));
						break;
					} else {
						exists = Some(FieldCell::new(end_col, end_row));
						break;
					}
				}
				if end_row == 0 {
					exists = Some(FieldCell::new(end_col, end_row));
					break;
				}
				if end_row > FIELD_RESOLUTION - 1 {
					exists = Some(FieldCell::new(end_col, FIELD_RESOLUTION - 1));
					break;
				}
				// if end_col == 0 && (end_row == 0 || end_row == FIELD_RESOLUTION -1) {
				// 	exists = Some(FieldCell::new(end_col, end_row))
				// }
			}
			if let Some(end) = exists {
				end
			} else {
				//TODO make this better
				panic!("LOS corner prop failed to find decrement boundary");
			}
		}
	}
}

/// Recursively expand the neighbours of a list of [FieldCell] and calculate
/// their value in the [IntegrationField]
fn process_neighbours(
	int_field: &mut IntegrationField,
	queue: Vec<(FieldCell, u32)>,
	cost_field: &CostField,
) {
	let mut next_neighbours = Vec::new();
	// iterate over the queue calculating neighbour int costs
	for (cell, prev_int_cost) in queue.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
		// iterate over the neighbours calculating int costs
		for n in neighbours.iter() {
			// ensure neighbour isn't impassable
			let n_int = int_field.get_field_cell_value(*n);
			if n_int & INT_BITS_IMPASSABLE != INT_BITS_IMPASSABLE
				&& n_int & INT_BITS_LOS != INT_BITS_LOS
			{
				let cell_cost = cost_field.get_field_cell_value(*n) as u32;
				let int_cost = cell_cost + (prev_int_cost & INT_FILTER_BITS_COST);
				if int_cost < (n_int & INT_FILTER_BITS_COST) {
					int_field.set_field_cell_value(int_cost, *n);
					next_neighbours.push((*n, int_cost));
				}
			}
		}
	}
	if !next_neighbours.is_empty() {
		process_neighbours(int_field, next_neighbours, cost_field);
	}
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hori_los_prop_max() {
		let goal = FieldCell::new(4, 3);
		let adjacent = FieldCell::new(5, 3);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(9, 3);
		assert_eq!(actual, result)
	}
	#[test]
	fn hori_los_prop_min() {
		let goal = FieldCell::new(5, 3);
		let adjacent = FieldCell::new(4, 3);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(0, 3);
		assert_eq!(actual, result)
	}
	#[test]
	fn vert_los_prop_max() {
		let goal = FieldCell::new(4, 3);
		let adjacent = FieldCell::new(4, 5);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(4, 9);
		assert_eq!(actual, result)
	}
	#[test]
	fn vert_los_prop_min() {
		let goal = FieldCell::new(4, 5);
		let adjacent = FieldCell::new(4, 3);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(4, 0);
		assert_eq!(actual, result)
	}
	/// Left right down check
	#[test]
	fn los_prop_left_right_down() {
		let goal = FieldCell::new(4, 3);
		let adjacent = FieldCell::new(5, 7);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(6, 9);
		assert_eq!(actual, result)
	}
	#[test]
	fn los_prop_left_right_up() {
		let goal = FieldCell::new(4, 3);
		let adjacent = FieldCell::new(7, 2);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(9, 1);
		assert_eq!(actual, result)
	}
	#[test]
	fn los_prop_right_left_down() {
		let goal = FieldCell::new(5, 1);
		let adjacent = FieldCell::new(3, 3);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(0, 6);
		assert_eq!(actual, result)
	}
	#[test]
	fn los_prop_right_left_up() {
		let goal = FieldCell::new(8, 7);
		let adjacent = FieldCell::new(6, 3);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(4, 0);
		assert_eq!(actual, result)
	}
	#[test]
	fn los_prop_right_left_up2() {
		let goal = FieldCell::new(4, 6);
		let adjacent = FieldCell::new(3, 5);
		let result = check_los_corner_propagation(&adjacent, &goal);
		let actual = FieldCell::new(0, 2);
		assert_eq!(actual, result)
	}
	/// Calculate integration field without a LOS pass to check propagation of a uniform cost field with a source near the centre
	#[test]
	fn basic_field() {
		let cost_field = CostField::default();
		let goal = FieldCell::new(4, 4);
		let mut integration_field = IntegrationField::new(&goal, &cost_field);
		// set the corner as the goal as we're skipping a LOS pass
		integration_field.add_los_corner(goal);
		integration_field.calculate_field(&cost_field);
		let mut result = *integration_field.get();
		// strip flags from result
		for col in result.iter_mut() {
			for value in col.iter_mut() {
				*value &= INT_FILTER_BITS_COST
			}
		}
		// visually this is weird, columns look like rows, see comments on array lines
		let actual: [[u32; FIELD_RESOLUTION]; FIELD_RESOLUTION] = [
			[8,7,6,5,4,5,6,7,8,9], // column 0
			[7,6,5,4,3,4,5,6,7,8], // column 1, etc
			[6,5,4,3,2,3,4,5,6,7], // row 0 is 8 from first array, then 7 from next, etc
			[5,4,3,2,1,2,3,4,5,6], 
			[4,3,2,1,0,1,2,3,4,5], 
			[5,4,3,2,1,2,3,4,5,6], 
			[6,5,4,3,2,3,4,5,6,7], 
			[7,6,5,4,3,4,5,6,7,8], 
			[8,7,6,5,4,5,6,7,8,9], 
			[9,8,7,6,5,6,7,8,9,10]
		];
		assert_eq!(actual, result);
	}
	// /// Calculate integration field from a custom cost field set
	// #[test]
	// fn complex_field() {
	// 	let mut cost_field = CostField::default();
	// 	cost_field.set_field_cell_value(255, FieldCell::new(5, 6));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(5, 7));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(6, 9));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(6, 8));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(6, 7));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(6, 4));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(7, 9));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(7, 4));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(8, 4));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(9, 4));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(1, 2));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(1, 1));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(2, 1));
	// 	cost_field.set_field_cell_value(255, FieldCell::new(2, 2));
	// 	let goal = FieldCell::new(4, 4);
	// 	let mut integration_field = IntegrationField::new(&goal, &cost_field);
	// 	integration_field.
	// 	integration_field.calculate_field(&cost_field);
	// 	let mut result = *integration_field.get();
	// 	// strip flags from result
	// 	for column in result.iter_mut() {
	// 		for value in column.iter_mut() {
	// 			*value = *value | INT_FILTER_BITS_COST
	// 		}
	// 	}

	// 	let actual: [[u32; FIELD_RESOLUTION]; FIELD_RESOLUTION] = [
	// 		[8,7,6,5,4,5,6,7,8,9], [7,65535,65535,4,3,4,5,6,7,8], [6,65535,65535,3,2,3,4,5,6,7], [5,4,3,2,1,2,3,4,5,6], [4,3,2,1,0,1,2,3,4,5], [5,4,3,2,1,2,65535,65535,5,6], [6,5,4,3,65535,3,4,65535,65535,65535], [7,6,5,4,65535,4,5,6,7,65535], [8,7,6,5,65535,5,6,7,8,9], [9,8,7,6,65535,6,7,8,9,10]
	// 	];
	// 	assert_eq!(actual, result);
	// }
}
