//! The IntegrationField contains a 2D array of 32-bit values and it uses a [CostField] to
//! produce a cumulative cost of reaching the goal/target. Every Sector has a [IntegrationField] associated with it.
//!
//! When a new route needs to be processed the field is set to `0` and any impassable cells from the associated CostField are set to `u16::MAX` (as a u32). A series of passes are performed from the goal as an expanding wavefront calculating the field values:
//!
//! 1. The valid ordinal neighbours of the goal are determined (North, East, South, West, when not against a boundary)
//! 2. For each ordinal field cell lookup their `CostField` value
//! 3. Add their cost to the `IntegrationField`s cost of the current cell (at the beginning this is the goal so + `0`)
//! 4. Propagate to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their integration cost, and repeat until the entire field is done
//!
//! This produces a nice diamond-like pattern as the wave expands (the underlying `CostField` are set to `1` here):
//!
//! ```text
//!  ___________________________________________________________
//! |     |     |     |     |     |     |     |     |     |     |
//! |  8  |  7  |  6  |  5  |  4  |  5  |  6  |  7  |  8  |  9  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  7  |  6  |  5  |  4  |  3  |  4  |  5  |  6  |  7  |  8  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  6  |  5  |  4  |  3  |  2  |  3  |  4  |  5  |  6  |  7  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  5  |  4  |  3  |  2  |  1  |  2  |  3  |  4  |  5  |  6  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  4  |  3  |  2  |  1  |  0  |  1  |  2  |  3  |  4  |  5  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  5  |  4  |  3  |  2  |  1  |  2  |  3  |  4  |  5  |  6  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  6  |  5  |  4  |  3  |  2  |  3  |  4  |  5  |  6  |  7  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  7  |  6  |  5  |  4  |  3  |  4  |  5  |  6  |  7  |  8  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  8  |  7  |  6  |  5  |  4  |  5  |  6  |  7  |  8  |  9  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  9  |  8  |  7  |  6  |  5  |  6  |  7  |  8  |  9  | 10  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! ```
//!
//! When it comes to `CostField` containing impassable markers, `255` as black boxes, they are ignored so the wave flows around those areas and when your `CostField` is using a range of values to indicate different areas to traverse, such as a steep hill, then you have various intermediate values similar to a terrain gradient.
//!
//! So this encourages the pathing algorithm around obstacles and expensive regions.
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
	has_expanded_portals: bool,
	has_los_pass: bool,
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
	pub fn get_integration_fields(
		&self,
	) -> &Vec<(SectorID, Vec<FieldCell>, IntegrationField)> {
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
			field.set_field_cell_value(0 + BITS_GOAL, self.path.get()[i].1);
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
				field.set_field_cell_value(0 + BITS_PORTAL, *g)
			}
		}
	}
	}
	//TODO docs
	pub fn calculate_los(&mut self) {
		let fields = self.get_mut_integration_fields();
		if let Some((_sector, goals, field)) = fields.first_mut() {
			field.set_initial_los(goals[0]);
			field.calcualte_sector_goal_los(goals, &goals[0]);
		}
		//TODO propagate LOS across sectors
		//until then set LOS corners in other sectors as the goals for int calc layer
		for (_sector, goals, field) in fields.iter_mut() {
			if field.los_corners.is_empty() {
				for g in goals {
					field.add_los_corner(*g);
				}
			}
		}
	}
	// fn propagate_los() {}
	pub fn build_integrated_cost(&mut self, cost_fields: &SectorCostFields) {
		for (sector_id, goals, int_field) in self.get_mut_integration_fields() {
			let cost_field = cost_fields.get_scaled()
			.get(sector_id)
			.unwrap();
			//TODO explain using los corners
			int_field.calculate_field(cost_field);
		}
	}
}

pub const BITS_IMPASSABLE: u32 = 0b0000_0000_0000_0000_1111_1111_1111_1111; //TODO maybe make flag
pub const BITS_LOS: u32 = 0b0000_0000_0000_0001_0000_0000_0000_0000;
pub const BITS_GOAL: u32 = 0b0000_0000_0000_0010_0000_0000_0000_0000;
pub const BITS_WAVE_BLOCKED: u32 = 0b0000_0000_0000_0100_0000_0000_0000_0000;
pub const BITS_PORTAL: u32 = 0b0000_0000_0000_1000_0000_0000_0000_0000;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
pub struct IntegrationField {
	field: [[u32; FIELD_RESOLUTION]; FIELD_RESOLUTION],
	los_corners: Vec<FieldCell>,
}

impl Default for IntegrationField {
	fn default() -> Self {
		IntegrationField{
			field: [[0; FIELD_RESOLUTION]; FIELD_RESOLUTION],
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
	/// Creates a new [IntegrationField] where all impassable cells are set to
	/// `u16::MAX` and the rest `0`
	pub fn new(goal: &FieldCell, cost: &CostField) -> Self {
		let mut field = IntegrationField::default();
		for (column, rows) in cost.get().iter().enumerate() {
			for (row, value) in rows.iter().enumerate() {
				if *value == u8::MAX {
					field.set_field_cell_value(BITS_IMPASSABLE, FieldCell::new(column, row));
				}
			}
		}
		field.set_field_cell_value(0 + BITS_GOAL, *goal);
		field
	}
	//TODO remove this
	/// Reset all the cells of the [IntegrationField] to `u16::MAX` apart from
	/// the `goals` which are the starting points of calculating the field which is set to `0`
	pub fn reset(&mut self, goals: &Vec<FieldCell>) {
		for i in 0..FIELD_RESOLUTION {
			for j in 0..FIELD_RESOLUTION {
				self.set_field_cell_value(u16::MAX as u32, FieldCell::new(i, j));
			}
		}
		for goal in goals {
			self.set_field_cell_value(0, *goal);
		}
	}
	/// Sets the goal (not any portals) of the target sector as having Line Of Sight
	pub fn set_initial_los(&mut self, cell_id: FieldCell) {
		self.set_field_cell_value(BITS_LOS, cell_id);
	}
	fn get_los_coners(&self) -> &Vec<FieldCell>{
		&self.los_corners
	}
	fn add_los_corner(&mut self, corner: FieldCell) {
		self.los_corners.push(corner);
	}
	/// From the goal of the target sector calcualte LOS
	pub fn calcualte_sector_goal_los(&mut self, active_wavefront: &Vec<FieldCell>, goal: &FieldCell) {
		let wavefront_cost = 1;
		propagate_los(self, active_wavefront, wavefront_cost, goal);
		//TODO cell is just the goal, change argument and remove loop
		// for cell in active_wavefront.iter() {
		// 	let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
		// 	for n in neighbours.iter() {
		// 		let cost = self.get_field_cell_value(*n);
		// 		if cost & BITS_WAVE_BLOCKED == BITS_WAVE_BLOCKED {
		// 			// wave blocked don't propagate LOS from this neighbour
		// 		} else if cost & BITS_IMPASSABLE == BITS_IMPASSABLE {
		// 			// found impassable, look for LOS corner
		// 			// based on the direction towards `n`, look at n's neighbours,
		// 			// if a neighbour isn't impassable then it means there's
		// 			// a LOS corner
		// 			let dir = Ordinal::cell_to_cell_direction(*n, *cell);
		// 			// depending on direction get the cells next to `n`
		// 			match dir {
		// 				Ordinal::North => {
		// 					for ord in [Ordinal::West, Ordinal::East] {
		// 						if let Some(adj) = Ordinal::get_cell_neighbour(*n, ord) {
		// 							let value = self.get_field_cell_value(adj);
		// 							if value & BITS_IMPASSABLE != BITS_IMPASSABLE {
		// 								// LOS corner found, store it for use in the cost integration calc later
		// 								los_corners.push(adj);
		// 								// mark wavefront blocked from the corner,
		// 								// using the line equation properties we find the vector
		// 								// from the goal to the corner and then find from 
		// 								// the corner what FieldCell on the Sector boundary the
		// 								// line would terminate at
		// 								//
		// 								// deal with vertical and horizontal lines first
		// 								let end = if adj.get_column() == cell.get_column() {
		// 									// no column change, find direction
		// 									// of y change
		// 									if adj.get_row() > cell.get_row() {
		// 										// dir is heading down to max boundary value
		// 										FieldCell::new(adj.get_column(), FIELD_RESOLUTION - 1)
		// 									} else {
		// 										// dir is heading up towards boundary 0
		// 										FieldCell::new(adj.get_column(), 0)
		// 									}
		// 								} else if adj.get_row() == cell.get_row() {
		// 									// no row change, find direction of
		// 									// x change
		// 									if adj.get_column() > cell.get_column() {
		// 										// dir is heading right towards max boundary
		// 										FieldCell::new(FIELD_RESOLUTION -1, adj.get_row())
		// 									} else {
		// 										// dir is heading left towards boundary 0
		// 										FieldCell::new(0, adj.get_row())
		// 									}
		// 								} else {
		// 									// handle diagonal lines
		// 									let delta_column = adj.get_column() as f32 - cell.get_column() as f32;
		// 									let delta_row = adj.get_row() as f32 - cell.get_row() as f32;
		// 									let gradient = delta_row/delta_column;
		// 									let intercept = -gradient * (adj.get_column() as f32) + adj.get_row() as f32;
		// 									let mut exists = None;
		// 									if adj.get_column() > cell.get_column() {
		// 										// walk the line with increasing column
		// 										// until the row or column value
		// 										// reaches a sector boundary
		// 										let d = FIELD_RESOLUTION -1 - adj.get_column();
		// 										for x in 0..=d {
		// 											let end_col = adj.get_column() + x;
		// 											let end_row = (gradient * (end_col as f32) + intercept).floor() as usize;
		// 											if end_col == FIELD_RESOLUTION -1 || end_row == 0 || end_row == FIELD_RESOLUTION -1 {
		// 												exists = Some(FieldCell::new(end_col, end_row))
		// 											}
		// 										}
		// 										if let Some(end) = exists {
		// 											end
		// 										} else {
		// 											//TODO make this better
		// 											panic!("LOS corner prop failed to find boundary");
		// 										}
		// 									} else {
		// 										// walk the line with decreasing column
		// 										// until row or column value
		// 										// reaches a sector boundary
		// 										let d = adj.get_column();
		// 										for x in 0..=d {
		// 											//TODO prevent overflow
		// 											let end_col = adj.get_column() - x;
		// 											let end_row = (gradient * (end_col as f32) + intercept).floor() as usize;
		// 											if end_col == 0 || end_row == 0 || end_row == FIELD_RESOLUTION -1 {
		// 												exists = Some(FieldCell::new(end_col, end_row))
		// 											}
		// 										}
		// 										if let Some(end) = exists {
		// 											end
		// 										} else {
		// 											//TODO make this better
		// 											panic!("LOS corner prop failed to find boundary");
		// 										}
		// 									}
		// 								};
		// 								// from the corner to the boundary cell of LOS being blocked use the bresenham line algorithm to find all cells between the two cell points and mark them as being wavefront blocked so that further LOS propagation won't flow behind impassable cells
		// 								let blocked_cells = adj.get_cells_between_points(&end);
		// 								for blocked in blocked_cells.iter() {
		// 									let value = self.get_field_cell_value(*blocked);
		// 									// only mark flags for cells that aren't impassable and which aren't already marked as wavefront blocked
		// 									if value & BITS_IMPASSABLE != BITS_IMPASSABLE && value & BITS_WAVE_BLOCKED != BITS_WAVE_BLOCKED {
		// 										self.set_field_cell_value(value + BITS_WAVE_BLOCKED, *blocked);
		// 									}
		// 								}
		// 							}
		// 						}
		// 					}
		// 				},
		// 				Ordinal::East => todo!(),
		// 				Ordinal::South => todo!(),
		// 				Ordinal::West => todo!(),
		// 				_ => {panic!("Dir should only be orthogonal")}
		// 			}
		// 		} else if cost & BITS_LOS != BITS_LOS {
		// 			// we have a new LOS that can be propagated
		// 			self.set_field_cell_value(wavefront_cost + BITS_LOS, *cell);
		// 			moved_wavefront.push(*n);
		// 		}
		// 	}
		// 	wavefront_cost +=1;
		// }
	}
	/// From active wavefronts and blocked wavefront directions propagate LOS into other sectors
	pub fn propagate_boundary_los(&mut self) {}





	//TODO: diamond like propagation and wasted extra lookups looking at previously calcualted neighbours, try fast marching method of solving Eikonal PDE for a spherical approx that visits each cell once
	/// From a list of `goals` (the actual end target goal or portal field cells
	/// to the next sector towards the goal sector) field cells iterate over
	/// successive neighbouring cells and calculate the field values from the
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
fn propagate_los(
	field: &mut IntegrationField,
	active_wavefront: &Vec<FieldCell>,
	mut wavefront_cost: u32,
	goal: &FieldCell,
) {
	let mut moved_wavefront: Vec<FieldCell> = Vec::new();
	for wavefront in active_wavefront.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*wavefront);
		for n in neighbours.iter() {
			let cost = field.get_field_cell_value(*n);
			if cost & BITS_WAVE_BLOCKED == BITS_WAVE_BLOCKED {
				// wave blocked don't propagate LOS from this neighbour
			} else if cost & BITS_IMPASSABLE == BITS_IMPASSABLE {
				// found impassable, look for LOS corner
				// based on the direction towards `n`, look at n's neighbours,
				// if a neighbour isn't impassable then it means there's
				// a LOS corner
				let dir = Ordinal::cell_to_cell_direction(*n, *wavefront);
				// depending on direction get the cells next to `n`
				match dir {
					Ordinal::North | Ordinal::South => {
						for ord in [Ordinal::West, Ordinal::East] {
							extend_los_corner(field, n, ord, goal);
						}
					},
					Ordinal::East| Ordinal::West => {
						for ord in [Ordinal::North, Ordinal::South] {
							extend_los_corner(field, n, ord, goal);
						}
					}
					_ => {panic!("Dir should only be orthogonal")}
				}
			} else if cost & BITS_LOS != BITS_LOS {
				// we have a new LOS that can be propagated
				field.set_field_cell_value(wavefront_cost + BITS_LOS, *n);
				moved_wavefront.push(*n);
			}
		}
		wavefront_cost +=1;
		// if valid cells exist to continue propagation then recursively propagate LOS
		if !moved_wavefront.is_empty() {
			propagate_los(field, &moved_wavefront, wavefront_cost, goal);
		}
	}
}

fn extend_los_corner(
	field: &mut IntegrationField,
	neighbour: &FieldCell,
	ord: Ordinal,
	goal: &FieldCell,
) {
	if let Some(adj) = Ordinal::get_cell_neighbour(*neighbour, ord) {
		let value = field.get_field_cell_value(adj);
		if value & BITS_IMPASSABLE != BITS_IMPASSABLE {
			// LOS corner found, store it for use in the cost integration calc later
			field.add_los_corner(adj);//TODO need ot set LOS corner as LOS? int calc might be broken
			// mark wavefront blocked from the corner,
			// using the line equation properties we find the vector
			// from the goal to the corner and then find from 
			// the corner what FieldCell on the Sector boundary the
			// line would terminate at
			//
			// deal with vertical and horizontal lines first
			let end = if adj.get_column() == goal.get_column() {
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
					FieldCell::new(FIELD_RESOLUTION -1, adj.get_row())
				} else {
					// dir is heading left towards boundary 0
					FieldCell::new(0, adj.get_row())
				}
			} else {
				// handle diagonal lines
				let delta_column = adj.get_column() as f32 - goal.get_column() as f32;
				let delta_row = adj.get_row() as f32 - goal.get_row() as f32;
				let gradient = delta_row/delta_column;
				let intercept = -gradient * (adj.get_column() as f32) + adj.get_row() as f32;
				let mut exists = None;
				if adj.get_column() > goal.get_column() {
					// walk the line with increasing column
					// until the row or column value
					// reaches a sector boundary
					let d = FIELD_RESOLUTION -1 - adj.get_column();
					for x in 0..=d {
						let end_col = adj.get_column() + x;
						let end_row = (gradient * (end_col as f32) + intercept).floor() as usize;
						if end_col == FIELD_RESOLUTION -1 || end_row == 0 || end_row == FIELD_RESOLUTION -1 {
							exists = Some(FieldCell::new(end_col, end_row))
						}
					}
					if let Some(end) = exists {
						end
					} else {
						//TODO make this better
						panic!("LOS corner prop failed to find boundary");
					}
				} else {
					// walk the line with decreasing column
					// until row or column value
					// reaches a sector boundary
					let d = adj.get_column();
					for x in 0..=d {
						//TODO prevent overflow
						let end_col = adj.get_column() - x;
						let end_row = (gradient * (end_col as f32) + intercept).floor() as usize;
						if end_col == 0 || end_row == 0 || end_row == FIELD_RESOLUTION -1 {
							exists = Some(FieldCell::new(end_col, end_row))
						}
					}
					if let Some(end) = exists {
						end
					} else {
						//TODO make this better
						panic!("LOS corner prop failed to find boundary");
					}
				}
			};
			// from the corner to the boundary cell of LOS being blocked use the bresenham line algorithm to find all cells between the two cell points and mark them as being wavefront blocked so that further LOS propagation won't flow behind impassable cells
			let blocked_cells = adj.get_cells_between_points(&end);
			for blocked in blocked_cells.iter() {
				let value = field.get_field_cell_value(*blocked);
				// only mark flags for cells that aren't impassable and which aren't already marked as wavefront blocked
				if value & BITS_IMPASSABLE != BITS_IMPASSABLE && value & BITS_WAVE_BLOCKED != BITS_WAVE_BLOCKED {
					field.set_field_cell_value(value + BITS_WAVE_BLOCKED, *blocked);
				}
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
	let filter = 0b0000_0000_0000_0000_1111_1111_1111_1111;
	let mut next_neighbours = Vec::new();
	// iterate over the queue calculating neighbour int costs
	for (cell, prev_int_cost) in queue.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
		// iterate over the neighbours calculating int costs
		for n in neighbours.iter() {
			// ensure neighbour isn't impassable or already assigned LOS
			let n_int = int_field.get_field_cell_value(*n);
			if n_int & BITS_IMPASSABLE != BITS_IMPASSABLE && n_int & BITS_LOS != BITS_LOS {
				let cell_cost = cost_field.get_field_cell_value(*n) as u32;
				let int_cost = cell_cost + prev_int_cost & filter;
				if int_cost < int_field.get_field_cell_value(*n) {
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
	/// Calculate integration field from a uniform cost field with a source near the centre
	#[test]
	fn basic_field() {
		let cost_field = CostField::default();
		let mut integration_field = IntegrationField::default();
		let source = vec![FieldCell::new(4, 4)];
		integration_field.reset(&source);
		integration_field.calculate_field(&source, &cost_field);
		let result = integration_field.get();

		let actual: [[u32; FIELD_RESOLUTION]; FIELD_RESOLUTION] = [
			[8,7,6,5,4,5,6,7,8,9], [7,6,5,4,3,4,5,6,7,8], [6,5,4,3,2,3,4,5,6,7], [5,4,3,2,1,2,3,4,5,6], [4,3,2,1,0,1,2,3,4,5], [5,4,3,2,1,2,3,4,5,6], [6,5,4,3,2,3,4,5,6,7], [7,6,5,4,3,4,5,6,7,8], [8,7,6,5,4,5,6,7,8,9], [9,8,7,6,5,6,7,8,9,10]
		];


		assert_eq!(actual, *result);
	}
	/// Calculate integration field from a custom cost field set
	#[test]
	fn complex_field() {
		let mut cost_field = CostField::default();
		cost_field.set_field_cell_value(255, FieldCell::new(5, 6));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 9));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 8));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 4));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 9));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 4));
		cost_field.set_field_cell_value(255, FieldCell::new(8, 4));
		cost_field.set_field_cell_value(255, FieldCell::new(9, 4));
		cost_field.set_field_cell_value(255, FieldCell::new(1, 2));
		cost_field.set_field_cell_value(255, FieldCell::new(1, 1));
		cost_field.set_field_cell_value(255, FieldCell::new(2, 1));
		cost_field.set_field_cell_value(255, FieldCell::new(2, 2));
		let mut integration_field = IntegrationField::default();
		let source = vec![FieldCell::new(4, 4)];
		integration_field.reset(&source);
		integration_field.calculate_field(&source, &cost_field);
		let result = integration_field.get();

		let actual: [[u32; FIELD_RESOLUTION]; FIELD_RESOLUTION] = [
			[8,7,6,5,4,5,6,7,8,9], [7,65535,65535,4,3,4,5,6,7,8], [6,65535,65535,3,2,3,4,5,6,7], [5,4,3,2,1,2,3,4,5,6], [4,3,2,1,0,1,2,3,4,5], [5,4,3,2,1,2,65535,65535,5,6], [6,5,4,3,65535,3,4,65535,65535,65535], [7,6,5,4,65535,4,5,6,7,65535], [8,7,6,5,65535,5,6,7,8,9], [9,8,7,6,65535,6,7,8,9,10]
		];
		assert_eq!(actual, *result);
	}
}
