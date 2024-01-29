//! A [FlowField] is a 2D array of 8-bit values. The various bit values
//! associated with it indicate directions of movement and flags to idenitfy
//! what's a goal, what's pathable and others. A steering pipeline/character
//! controller should read and interpret a [FlowField] to provide movement.
//!

use std::{
	collections::HashSet,
	sync::{Arc, Mutex},
};

use crate::prelude::*;
use bevy::prelude::*;
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
pub struct FlowField([[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for FlowField {
	fn default() -> Self {
		FlowField([[BITS_DEFAULT; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl Field<u8> for FlowField {
	/// Get a reference to the field array
	fn get(&self) -> &[[u8; FIELD_RESOLUTION]; FIELD_RESOLUTION] {
		&self.0
	}
	/// Retrieve a field cell value
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u8 {
		if field_cell.get_column() >= self.0.len() || field_cell.get_row() >= self.0[0].len() {
			panic!("Cannot get a CostField value, index out of bounds. Asked for column {}, row {}, field column length is {}, field row length is {}", field_cell.get_column(), field_cell.get_row(), self.0.len(), self.0[0].len())
		}
		self.0[field_cell.get_column()][field_cell.get_row()]
	}
	/// Set a field cell to a value
	fn set_field_cell_value(&mut self, value: u8, field_cell: FieldCell) {
		if field_cell.get_column() >= self.0.len() || field_cell.get_row() >= self.0[0].len() {
			panic!("Cannot set a CostField value, index out of bounds. Asked for column {}, row {}, field column length is {}, field row length is {}", field_cell.get_column(), field_cell.get_row(), self.0.len(), self.0[0].len())
		}
		self.0[field_cell.get_column()][field_cell.get_row()] = value;
	}
}
impl FlowField {
	/// Calculate the [FlowField] from an [IntegrationField], additionally for a sector in a chain of sectors along a path this will peak into the previous sectors [IntegrationField] to apply a directional optimisation to this sector's [FlowField]
	pub fn calculate(
		&mut self,
		goals: &[FieldCell],
		previous_sector_ord_int: Option<(Ordinal, &IntegrationField)>,
		integration_field: &IntegrationField,
	) {
		if let Some((ord, prev_field)) = previous_sector_ord_int {
			// peek into the previous sector to create better flows over the portal goals
			for goal in goals.iter() {
				// based on the ordinal get up to 3 neighbour int costs
				let possible_neighbours =
					lookup_portal_goal_neighbour_costs_in_previous_sector(goal, prev_field, ord);
				let mut cheapest_value = u16::MAX;
				let mut cheapest_ord = None;
				//TODO moving left to right around a wall can cause a bump north
				//TODO if <=, bottom to top aorund a wall can cause a siddeways bump
				for n in possible_neighbours.iter() {
					if n.1 < cheapest_value {
						cheapest_value = n.1;
						cheapest_ord = Some(n.0);
					}
				}
				if let Some(ord) = cheapest_ord {
					// point the portal goal towards the best one
					let ordinal_bits = convert_ordinal_to_bits_dir(ord);
					let mut value = 0;
					value |= BITS_PORTAL_GOAL;
					value |= ordinal_bits;
					self.set_field_cell_value(value, *goal);
				} //TODO this sould never ever be none...
			}
		} else {
			// set goal cells as this is the first flowfield i.e the end goal
			let mut goal_value = 0;
			goal_value |= BITS_HAS_LOS;
			goal_value |= BITS_GOAL;
			goal_value |= BITS_PATHABLE;
			self.set_field_cell_value(goal_value, goals[0]);
			self.calculate_line_of_sight(goals, integration_field);
		}

		for (i, column) in integration_field.get().iter().enumerate() {
			for (j, _row) in column.iter().enumerate() {
				let field_cell = FieldCell::new(i, j);
				if self.get_field_cell_value(field_cell) & BITS_DEFAULT == BITS_DEFAULT {
					let current_cost = integration_field.get_field_cell_value(field_cell);
					// mark impassable //TODO maybe skip? waste of time perhaps
					if current_cost == u16::MAX {
						self.set_field_cell_value(BITS_ZERO, field_cell);
					} else if current_cost != 0 {
						// skip goals of zero
						// store the cheapest node
						let mut cheapest_value = u16::MAX;
						let mut cheapest_neighbour = None;
						let mut neighbours = Ordinal::get_all_cell_neighbours(field_cell);

						// find any diagonal cells which are flanked by impassable cells and so
						// movement between them should be ignored/blocked, i.e
						//   X ~ <- ignore diagonal from o
						//   o X
						//
						let remove_diagonals =
							find_blocked_diagonals(field_cell, integration_field);
						for diag in remove_diagonals.iter() {
							neighbours.retain(|&n| n != *diag);
						}

						for n in neighbours.iter() {
							let neighbour_cost = integration_field.get_field_cell_value(*n);
							if neighbour_cost < cheapest_value {
								cheapest_value = neighbour_cost;
								cheapest_neighbour = Some(n);
							}
						}
						if let Some(target) = cheapest_neighbour {
							let ord = Ordinal::cell_to_cell_direction(*target, field_cell);
							let bit_ord = convert_ordinal_to_bits_dir(ord);
							let mut value = 0;
							value |= bit_ord;
							value |= BITS_PATHABLE;
							self.set_field_cell_value(value, field_cell);
						} //TODO this should never ever be none...
					}
				}
			}
		}
	}

	/// Calculate the [FlowField] from an [IntegrationField], additionally for a sector in a chain of sectors along a path this will peak into the previous sectors [IntegrationField] to apply a directional optimisation to this sector's [FlowField]
	pub fn calculate_arc(
		&mut self,
		goals: &[FieldCell],
		previous_sector_ord_int: Option<(Ordinal, &IntegrationField)>,
		integration_field: Arc<IntegrationField>,
	) {
		if let Some((ord, prev_field)) = previous_sector_ord_int {
			// peek into the previous sector to create better flows over the portal goals
			for goal in goals.iter() {
				// based on the ordinal get up to 3 neighbour int costs
				let possible_neighbours =
					lookup_portal_goal_neighbour_costs_in_previous_sector(goal, prev_field, ord);
				let mut cheapest_value = u16::MAX;
				let mut cheapest_ord = None;
				//TODO moving left to right around a wall can cause a bump north
				//TODO if <=, bottom to top aorund a wall can cause a siddeways bump
				for n in possible_neighbours.iter() {
					if n.1 < cheapest_value {
						cheapest_value = n.1;
						cheapest_ord = Some(n.0);
					}
				}
				if let Some(ord) = cheapest_ord {
					// point the portal goal towards the best one
					let ordinal_bits = convert_ordinal_to_bits_dir(ord);
					let mut value = 0;
					value |= BITS_PORTAL_GOAL;
					value |= ordinal_bits;
					self.set_field_cell_value(value, *goal);
				} //TODO this sould never ever be none...
			}
		} else {
			// set goal cells as this is the first flowfield i.e the end goal
			let mut goal_value = 0;
			goal_value |= BITS_HAS_LOS;
			goal_value |= BITS_GOAL;
			goal_value |= BITS_PATHABLE;
			self.set_field_cell_value(goal_value, goals[0]);
			self.calculate_line_of_sight_arc(Arc::new(goals[0]), &integration_field);
		}

		for (i, column) in integration_field.get().iter().enumerate() {
			for (j, _row) in column.iter().enumerate() {
				let integration_field = Arc::clone(&integration_field);
				let field_cell = FieldCell::new(i, j);
				if self.get_field_cell_value(field_cell) & BITS_DEFAULT == BITS_DEFAULT {
					let current_cost = integration_field.get_field_cell_value(field_cell);
					// mark impassable //TODO maybe skip? waste of time perhaps
					if current_cost == u16::MAX {
						self.set_field_cell_value(BITS_ZERO, field_cell);
					} else if current_cost != 0 {
						// skip goals of zero
						// store the cheapest node
						let mut cheapest_value = u16::MAX;
						let mut cheapest_neighbour = None;
						let mut neighbours = Ordinal::get_all_cell_neighbours(field_cell);

						// find any diagonal cells which are flanked by impassable cells and so
						// movement between them should be ignored/blocked, i.e
						//   X ~ <- ignore diagonal from o
						//   o X
						//
						let remove_diagonals =
							find_blocked_diagonals_arc(field_cell, integration_field.clone());
						for diag in remove_diagonals.iter() {
							neighbours.retain(|&n| n != *diag);
						}

						for n in neighbours.iter() {
							let neighbour_cost = integration_field.get_field_cell_value(*n);
							if neighbour_cost < cheapest_value {
								cheapest_value = neighbour_cost;
								cheapest_neighbour = Some(n);
							}
						}
						if let Some(target) = cheapest_neighbour {
							let ord = Ordinal::cell_to_cell_direction(*target, field_cell);
							let bit_ord = convert_ordinal_to_bits_dir(ord);
							let mut value = 0;
							value |= bit_ord;
							value |= BITS_PATHABLE;
							self.set_field_cell_value(value, field_cell);
						} //TODO this should never ever be none...
					}
				}
			}
		}
	}
	/// From each [FieldCell] analyse a line to the goal to ascertain if the cell has a clear Line of Sight to the goal and if so toggle the bit flag
	fn calculate_line_of_sight(&mut self, los_goals: &[FieldCell], int_field: &IntegrationField) {
		for i in 0..FIELD_RESOLUTION {
			for j in 0..FIELD_RESOLUTION {
				let start_cell = FieldCell::new(i, j);
				let path = start_cell.get_cells_between_points(&los_goals[0]);
				let mut has_los = true;
				'cells: for cell in path.iter() {
					let has_blocked_diag = has_blocked_diagonals(*cell, int_field);
					if int_field.get_field_cell_value(*cell) == u16::MAX || has_blocked_diag {
						has_los = false;
						break 'cells;
					}
				}
				if has_los {
					for cell in path.iter() {
						let mut value = 0;
						value |= BITS_HAS_LOS;
						value |= BITS_PATHABLE;
						self.set_field_cell_value(value, *cell);
					}
				}
			}
		}
	}
	/// From each [FieldCell] analyse a line to the goal to ascertain if the cell has a clear Line of Sight to the goal and if so toggle the bit flag
	fn calculate_line_of_sight_arc(
		&mut self,
		los_goal: Arc<FieldCell>,
		int_field: &Arc<IntegrationField>,
	) {
		let mut handles = Vec::new();
		// let (tx, rx) = std::sync::mpsc::channel();
		let los_cells = Arc::new(Mutex::new(HashSet::new()));
		for i in 0..FIELD_RESOLUTION {
			let int_field = Arc::clone(int_field);
			let los_goal = Arc::clone(&los_goal);
			let los_cells = Arc::clone(&los_cells);
			// let tx = tx.clone();
			let handle = std::thread::spawn(move || {
				for j in 0..FIELD_RESOLUTION {
					let los_goal = Arc::clone(&los_goal);
					let start_cell = FieldCell::new(i, j);
					let path = Arc::new(start_cell.get_cells_between_points_arc(los_goal));
					let mut has_los = true;
					'cells: for cell in path.iter() {
						let has_blocked_diag =
							has_blocked_diagonals_arc(*cell, Arc::clone(&int_field));
						if int_field.get_field_cell_value(*cell) == u16::MAX || has_blocked_diag {
							has_los = false;
							break 'cells;
						}
					}
					if has_los {
						let mut locked_los_cells = los_cells.lock().unwrap();
						for cell in path.iter() {
							locked_los_cells.insert(*cell);
							// tx.send(*cell).unwrap();
						}
					}
				}
				// drop(tx);
			});
			handles.push(handle);
		}
		for h in handles {
			h.join().unwrap();
		}
		// drop(tx);
		let locked_los_cells = los_cells.lock().unwrap();
		for received in locked_los_cells.iter() {
			// for received in rx {
			// println!("received");
			let mut value = 0;
			value |= BITS_HAS_LOS;
			value |= BITS_PATHABLE;
			self.set_field_cell_value(value, *received);
		}
	}
	// /// From the goals explore each [FieldCell] for the ordinal directions and set the [BITS_HAS_LOS] flag to indicate that a [FieldCell] has unobstructed sight of the goal. This means that an actor can transition to moving in a straight line vector to the goal instead of having to rely on the directional bits of the [FlowField]
	// fn calculate_line_of_sight2(&mut self, los_goals: &[FieldCell], int_field: &IntegrationField) {
	// 	let ordinals = [
	// 		Ordinal::North,
	// 		Ordinal::East,
	// 		Ordinal::South,
	// 		Ordinal::West,
	// 		Ordinal::NorthEast,
	// 		Ordinal::SouthEast,
	// 		Ordinal::SouthWest,
	// 		Ordinal::NorthWest,
	// 	];
	// 	for ord in ordinals.iter() {
	// 		if let Some(neighbour) = Ordinal::get_cell_neighbour(los_goals[0], *ord) {
	// 			// if not impassable then it has LOS
	// 			if int_field.get_field_cell_value(neighbour) != u16::MAX {
	// 				let mut value = 0;
	// 				value |= BITS_HAS_LOS;
	// 				value |= BITS_PATHABLE;
	// 				self.set_field_cell_value(value, neighbour);
	// 				self.explore_direction(neighbour, int_field, *ord);
	// 			}
	// 		}
	// 	}
	// }
	// fn explore_direction(
	// 	&mut self,
	// 	neighbour: FieldCell,
	// 	int_field: &IntegrationField,
	// 	ord: Ordinal,
	// ) {
	// 	if let Some(neighbour) = Ordinal::get_cell_neighbour(neighbour, ord) {
	// 		// if not impassable then it has LOS
	// 		if int_field.get_field_cell_value(neighbour) != u16::MAX {
	// 			let mut value = 0;
	// 			value |= BITS_HAS_LOS;
	// 			value |= BITS_PATHABLE;
	// 			self.set_field_cell_value(value, neighbour);
	// 			self.explore_direction(neighbour, int_field, ord);
	// 		}
	// 	}
	// }
}
/// Used by a [FlowField] calculation that needs to peek into the previous sectors [IntegrationField] to align portal goal directional bits to the most optimal integration costs
fn lookup_portal_goal_neighbour_costs_in_previous_sector(
	portal_goal: &FieldCell,
	previous_integration_field: &IntegrationField,
	sector_ordinal: Ordinal,
) -> Vec<(Ordinal, u16)> {
	let mut adjacent_neighbours = Vec::new();
	match sector_ordinal {
		Ordinal::North => {
			// orthogonal adjacent cost
			let adj_pos = (portal_goal.get_column(), 9);
			let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
			adjacent_neighbours.push((Ordinal::North, adj_cost));
			// try and get a cost left
			if portal_goal.get_column() > 0 {
				let adj_pos = (portal_goal.get_column() - 1, 9);
				let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
				adjacent_neighbours.push((Ordinal::NorthWest, adj_cost));
			}
			// try and get a cost right
			if portal_goal.get_column() < FIELD_RESOLUTION - 1 {
				let adj_pos = (portal_goal.get_column() + 1, 9);
				let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
				adjacent_neighbours.push((Ordinal::NorthEast, adj_cost));
			}
		}
		Ordinal::East => {
			// orthogonal adjacent cost
			let adj_pos = (0, portal_goal.get_row());
			let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
			adjacent_neighbours.push((Ordinal::East, adj_cost));
			// try and get a cost above
			if portal_goal.get_row() > 0 {
				let adj_pos = (0, portal_goal.get_row() - 1);
				let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
				adjacent_neighbours.push((Ordinal::NorthEast, adj_cost));
			}
			// try and get a cost below
			if portal_goal.get_row() < FIELD_RESOLUTION - 1 {
				let adj_pos = (0, portal_goal.get_row() + 1);
				let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
				adjacent_neighbours.push((Ordinal::SouthEast, adj_cost));
			}
		}
		Ordinal::South => {
			// orthogonal adjacent cost
			let adj_pos = (portal_goal.get_column(), 0);
			let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
			adjacent_neighbours.push((Ordinal::South, adj_cost));
			// try and get a cost left
			if portal_goal.get_column() > 0 {
				let adj_pos = (portal_goal.get_column() - 1, 0);
				let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
				adjacent_neighbours.push((Ordinal::SouthWest, adj_cost));
			}
			// try and get a cost right
			if portal_goal.get_column() < FIELD_RESOLUTION - 1 {
				let adj_pos = (portal_goal.get_column() + 1, 0);
				let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
				adjacent_neighbours.push((Ordinal::SouthEast, adj_cost));
			}
		}
		Ordinal::West => {
			// orthogonal adjacent cost
			let adj_pos = (9, portal_goal.get_row());
			let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
			adjacent_neighbours.push((Ordinal::West, adj_cost));
			// try and get a cost above
			if portal_goal.get_row() > 0 {
				let adj_pos = (9, portal_goal.get_row() - 1);
				let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
				adjacent_neighbours.push((Ordinal::NorthWest, adj_cost));
			}
			// try and get a cost below
			if portal_goal.get_row() < FIELD_RESOLUTION - 1 {
				let adj_pos = (9, portal_goal.get_row() + 1);
				let adj_cost = previous_integration_field.get()[adj_pos.0][adj_pos.1];
				adjacent_neighbours.push((Ordinal::SouthWest, adj_cost));
			}
		}
		_ => panic!("Invalid sector ordinal"),
	}
	adjacent_neighbours
}

/// Looks at the orthognal neighbours of a [FieldCell], determines whether any pairs are impassable and if so builds a list of any diagonal [FieldCell] which should be considered as unreachable from the inspected `field_cell`
fn find_blocked_diagonals(
	field_cell: FieldCell,
	integration_field: &IntegrationField,
) -> Vec<FieldCell> {
	let mut diagonals = Vec::new();
	if let Some(north) = Ordinal::get_cell_neighbour(field_cell, Ordinal::North) {
		if let Some(east) = Ordinal::get_cell_neighbour(field_cell, Ordinal::East) {
			if integration_field.get_field_cell_value(north) == u16::MAX
				&& integration_field.get_field_cell_value(east) == u16::MAX
			{
				if let Some(north_east) =
					Ordinal::get_cell_neighbour(field_cell, Ordinal::NorthEast)
				{
					diagonals.push(north_east);
				}
			}
		}
		if let Some(west) = Ordinal::get_cell_neighbour(field_cell, Ordinal::West) {
			if integration_field.get_field_cell_value(north) == u16::MAX
				&& integration_field.get_field_cell_value(west) == u16::MAX
			{
				if let Some(north_west) =
					Ordinal::get_cell_neighbour(field_cell, Ordinal::NorthWest)
				{
					diagonals.push(north_west);
				}
			}
		}
	}
	if let Some(south) = Ordinal::get_cell_neighbour(field_cell, Ordinal::South) {
		if let Some(east) = Ordinal::get_cell_neighbour(field_cell, Ordinal::East) {
			if integration_field.get_field_cell_value(south) == u16::MAX
				&& integration_field.get_field_cell_value(east) == u16::MAX
			{
				if let Some(south_east) =
					Ordinal::get_cell_neighbour(field_cell, Ordinal::SouthEast)
				{
					diagonals.push(south_east);
				}
			}
		}
		if let Some(west) = Ordinal::get_cell_neighbour(field_cell, Ordinal::West) {
			if integration_field.get_field_cell_value(south) == u16::MAX
				&& integration_field.get_field_cell_value(west) == u16::MAX
			{
				if let Some(south_west) =
					Ordinal::get_cell_neighbour(field_cell, Ordinal::SouthWest)
				{
					diagonals.push(south_west);
				}
			}
		}
	}
	diagonals
}

/// Looks at the orthognal neighbours of a [FieldCell], determines whether any pairs are impassable and if so builds a list of any diagonal [FieldCell] which should be considered as unreachable from the inspected `field_cell`
fn find_blocked_diagonals_arc(
	field_cell: FieldCell,
	integration_field: Arc<IntegrationField>,
) -> Vec<FieldCell> {
	let mut diagonals = Vec::new();
	if let Some(north) = Ordinal::get_cell_neighbour(field_cell, Ordinal::North) {
		if let Some(east) = Ordinal::get_cell_neighbour(field_cell, Ordinal::East) {
			if integration_field.get_field_cell_value(north) == u16::MAX
				&& integration_field.get_field_cell_value(east) == u16::MAX
			{
				if let Some(north_east) =
					Ordinal::get_cell_neighbour(field_cell, Ordinal::NorthEast)
				{
					diagonals.push(north_east);
				}
			}
		}
		if let Some(west) = Ordinal::get_cell_neighbour(field_cell, Ordinal::West) {
			if integration_field.get_field_cell_value(north) == u16::MAX
				&& integration_field.get_field_cell_value(west) == u16::MAX
			{
				if let Some(north_west) =
					Ordinal::get_cell_neighbour(field_cell, Ordinal::NorthWest)
				{
					diagonals.push(north_west);
				}
			}
		}
	}
	if let Some(south) = Ordinal::get_cell_neighbour(field_cell, Ordinal::South) {
		if let Some(east) = Ordinal::get_cell_neighbour(field_cell, Ordinal::East) {
			if integration_field.get_field_cell_value(south) == u16::MAX
				&& integration_field.get_field_cell_value(east) == u16::MAX
			{
				if let Some(south_east) =
					Ordinal::get_cell_neighbour(field_cell, Ordinal::SouthEast)
				{
					diagonals.push(south_east);
				}
			}
		}
		if let Some(west) = Ordinal::get_cell_neighbour(field_cell, Ordinal::West) {
			if integration_field.get_field_cell_value(south) == u16::MAX
				&& integration_field.get_field_cell_value(west) == u16::MAX
			{
				if let Some(south_west) =
					Ordinal::get_cell_neighbour(field_cell, Ordinal::SouthWest)
				{
					diagonals.push(south_west);
				}
			}
		}
	}
	diagonals
}

/// Looks at the orthogonal neighbours of a [FieldCell], determines whether any pairs are impassable and returns true if a diagonal field cell is blocked
fn has_blocked_diagonals(field_cell: FieldCell, integration_field: &IntegrationField) -> bool {
	if let Some(north) = Ordinal::get_cell_neighbour(field_cell, Ordinal::North) {
		if let Some(east) = Ordinal::get_cell_neighbour(field_cell, Ordinal::East) {
			if integration_field.get_field_cell_value(north) == u16::MAX
				&& integration_field.get_field_cell_value(east) == u16::MAX
				&& Ordinal::get_cell_neighbour(field_cell, Ordinal::NorthEast).is_some()
			{
				return true;
			}
		}
		if let Some(west) = Ordinal::get_cell_neighbour(field_cell, Ordinal::West) {
			if integration_field.get_field_cell_value(north) == u16::MAX
				&& integration_field.get_field_cell_value(west) == u16::MAX
				&& Ordinal::get_cell_neighbour(field_cell, Ordinal::NorthWest).is_some()
			{
				return true;
			}
		}
	}
	if let Some(south) = Ordinal::get_cell_neighbour(field_cell, Ordinal::South) {
		if let Some(east) = Ordinal::get_cell_neighbour(field_cell, Ordinal::East) {
			if integration_field.get_field_cell_value(south) == u16::MAX
				&& integration_field.get_field_cell_value(east) == u16::MAX
				&& Ordinal::get_cell_neighbour(field_cell, Ordinal::SouthEast).is_some()
			{
				return true;
			}
		}
		if let Some(west) = Ordinal::get_cell_neighbour(field_cell, Ordinal::West) {
			if integration_field.get_field_cell_value(south) == u16::MAX
				&& integration_field.get_field_cell_value(west) == u16::MAX
				&& Ordinal::get_cell_neighbour(field_cell, Ordinal::SouthWest).is_some()
			{
				return true;
			}
		}
	}
	false
}

/// Looks at the orthogonal neighbours of a [FieldCell], determines whether any pairs are impassable and returns true if a diagonal field cell is blocked
fn has_blocked_diagonals_arc(
	field_cell: FieldCell,
	integration_field: Arc<IntegrationField>,
) -> bool {
	if let Some(north) = Ordinal::get_cell_neighbour(field_cell, Ordinal::North) {
		if let Some(east) = Ordinal::get_cell_neighbour(field_cell, Ordinal::East) {
			if integration_field.get_field_cell_value(north) == u16::MAX
				&& integration_field.get_field_cell_value(east) == u16::MAX
				&& Ordinal::get_cell_neighbour(field_cell, Ordinal::NorthEast).is_some()
			{
				return true;
			}
		}
		if let Some(west) = Ordinal::get_cell_neighbour(field_cell, Ordinal::West) {
			if integration_field.get_field_cell_value(north) == u16::MAX
				&& integration_field.get_field_cell_value(west) == u16::MAX
				&& Ordinal::get_cell_neighbour(field_cell, Ordinal::NorthWest).is_some()
			{
				return true;
			}
		}
	}
	if let Some(south) = Ordinal::get_cell_neighbour(field_cell, Ordinal::South) {
		if let Some(east) = Ordinal::get_cell_neighbour(field_cell, Ordinal::East) {
			if integration_field.get_field_cell_value(south) == u16::MAX
				&& integration_field.get_field_cell_value(east) == u16::MAX
				&& Ordinal::get_cell_neighbour(field_cell, Ordinal::SouthEast).is_some()
			{
				return true;
			}
		}
		if let Some(west) = Ordinal::get_cell_neighbour(field_cell, Ordinal::West) {
			if integration_field.get_field_cell_value(south) == u16::MAX
				&& integration_field.get_field_cell_value(west) == u16::MAX
				&& Ordinal::get_cell_neighbour(field_cell, Ordinal::SouthWest).is_some()
			{
				return true;
			}
		}
	}
	false
}

//TODO? high level steering within this plugin??
// pub fn abc(cell_value: u8) {
// 	let flag_filter = 0b1111_0000;
// 	let dir_filter = 0b0000_1111;
// 	todo!();

// 	let flags = cell_value & flag_filter;
// 	match flags {
// 		BITS_GOAL => {
// 			// arrived at goal,
// 		}
// 		BITS_PORTAL_GOAL => {}
// 		BITS_HAS_LOS => {}
// 		BITS_PATHABLE => {}
// 		_ => panic!("Last 4 bits of cell are not recognised flags"),
// 	}
// }

/// Indicates that a cell is pathable
pub fn is_pathable(cell_value: u8) -> bool {
	cell_value & BITS_PATHABLE == BITS_PATHABLE
}

/// Indicates that a cell is the target goal
pub fn is_goal(cell_value: u8) -> bool {
	cell_value & BITS_GOAL == BITS_GOAL
}

/// Indicates that a cell is a portal goal
pub fn is_portal_goal(cell_value: u8) -> bool {
	cell_value & BITS_PORTAL_GOAL == BITS_PORTAL_GOAL
}

/// If a cell has direct vision to the goal then the [FlowField] should be
/// disregarded as the actor can move in a stright line to the goal
pub fn has_line_of_sight(cell_value: u8) -> bool {
	// let flag_filter = 0b1111_0000;
	cell_value & BITS_HAS_LOS == BITS_HAS_LOS
}
/// From a pathable [FlowField] cell get the directional [Ordinal] of movement
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
		_ => panic!("First 4 bits of cell are not recognised directions"),
	}
}
/// Reading the directional bits of a [FlowField] field cell obtain a unit
/// vector in 2d space of the direction
pub fn get_2d_direction_unit_vector_from_bits(cell_value: u8) -> Vec2 {
	let dir_filter = 0b0000_1111;
	let dir = cell_value & dir_filter;
	match dir {
		BITS_NORTH => Vec2::new(0.0, 1.0),
		BITS_EAST => Vec2::new(1.0, 0.0),
		BITS_SOUTH => Vec2::new(0.0, -1.0),
		BITS_WEST => Vec2::new(-1.0, 0.0),
		BITS_NORTH_EAST => Vec2::new(1.0, 1.0),
		BITS_SOUTH_EAST => Vec2::new(1.0, -1.0),
		BITS_SOUTH_WEST => Vec2::new(-1.0, -1.0),
		BITS_NORTH_WEST => Vec2::new(-1.0, 1.0),
		BITS_ZERO => Vec2::new(0.0, 0.0),
		_ => panic!("First 4 bits of cell are not recognised directions"),
	}
}
/// Reading the directional bits of a [FlowField] field cell obtain a unit
/// vector in 3d space of the direction across the x-z plane
pub fn get_3d_direction_unit_vector_from_bits(cell_value: u8) -> Vec3 {
	let dir_filter = 0b0000_1111;
	let dir = cell_value & dir_filter;
	match dir {
		BITS_NORTH => Vec3::new(0.0, 0.0, -1.0),
		BITS_EAST => Vec3::new(1.0, 0.0, 0.0),
		BITS_SOUTH => Vec3::new(0.0, 0.0, 1.0),
		BITS_WEST => Vec3::new(-1.0, 0.0, 0.0),
		BITS_NORTH_EAST => Vec3::new(1.0, 0.0, -1.0),
		BITS_SOUTH_EAST => Vec3::new(1.0, 0.0, 1.0),
		BITS_SOUTH_WEST => Vec3::new(-1.0, 0.0, 1.0),
		BITS_NORTH_WEST => Vec3::new(-1.0, 0.0, -1.0),
		BITS_ZERO => Vec3::new(0.0, 0.0, 0.0),
		_ => panic!("First 4 bits of cell are not recognised directions"),
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn default_init() {
		let flow_field = FlowField::default();
		let v = flow_field.get_field_cell_value(FieldCell::new(0, 0));
		assert_eq!(BITS_DEFAULT, v);
	}
	/// Flowfield of a single sector, all far southern cells are goals, verify direct paths from top to bottom
	#[test]
	fn calculate_flow_target_south() {
		let cost_field = CostField::default();
		// int field pair pointing towards goal in orthognal south direction
		let ordinal_to_previous_sector = Ordinal::South;
		let goals = vec![
			FieldCell::new(0, 9),
			FieldCell::new(1, 9),
			FieldCell::new(2, 9),
			FieldCell::new(3, 9),
			FieldCell::new(4, 9),
			FieldCell::new(5, 9),
			FieldCell::new(6, 9),
			FieldCell::new(7, 9),
			FieldCell::new(8, 9),
			FieldCell::new(9, 9),
		];
		let mut previous_int_field = IntegrationField::new(&goals);
		previous_int_field.calculate_field(&goals, &cost_field);
		let previous_sector_ord_int = Some((ordinal_to_previous_sector, &previous_int_field));

		let mut integration_field = IntegrationField::new(&goals);
		integration_field.calculate_field(&goals, &cost_field);

		let mut flow_field = FlowField::default();
		flow_field.calculate(&goals, previous_sector_ord_int, &integration_field);

		for column in flow_field.get().iter() {
			for row_value in column.iter() {
				if *row_value != BITS_PATHABLE + BITS_SOUTH
					&& *row_value != BITS_PORTAL_GOAL + BITS_SOUTH
				{
					println!("Flow field: {:?}", flow_field.get());
					panic!("Some FlowField default bits have not been replaced");
				}
			}
		}
	}
	/// Flowfield of a single sector, all far western cells are goals, verify direct paths from right to left
	#[test]
	fn calculate_flow_target_west() {
		let cost_field = CostField::default();
		// int field pair pointing towards goal in orthognal west direction
		let ordinal_to_previous_sector = Ordinal::West;
		let goals = vec![
			FieldCell::new(0, 0),
			FieldCell::new(0, 1),
			FieldCell::new(0, 2),
			FieldCell::new(0, 3),
			FieldCell::new(0, 4),
			FieldCell::new(0, 5),
			FieldCell::new(0, 6),
			FieldCell::new(0, 7),
			FieldCell::new(0, 8),
			FieldCell::new(0, 9),
		];
		let mut previous_int_field = IntegrationField::new(&goals);
		previous_int_field.calculate_field(&goals, &cost_field);
		let previous_sector_ord_int = Some((ordinal_to_previous_sector, &previous_int_field));

		let mut integration_field = IntegrationField::new(&goals);
		integration_field.calculate_field(&goals, &cost_field);

		let mut flow_field = FlowField::default();
		flow_field.calculate(&goals, previous_sector_ord_int, &integration_field);

		for column in flow_field.get().iter() {
			for row_value in column.iter() {
				if *row_value != BITS_PATHABLE + BITS_WEST
					&& *row_value != BITS_PORTAL_GOAL + BITS_WEST
				{
					println!("Flow field: {:?}", flow_field.get());
					panic!("Some FlowField default bits have not been replaced");
				}
			}
		}
	}
}
