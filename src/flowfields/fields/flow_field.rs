//! A [FlowField] is an array of 8-bit values. The various bit values
//! associated with it indicate directions of movement and flags to identify
//! what's a goal, what's pathable and others. A steering pipeline/character
//! controller should read and interpret a [FlowField] to provide movement.
//!

use bevy::prelude::*;

use crate::flowfields::{
	fields::{
		Field, FieldCell,
		integration_field::{
			INT_BITS_GOAL, INT_BITS_IMPASSABLE, INT_BITS_LOS, INT_BITS_PORTAL,
			INT_FILTER_BITS_COST, IntegrationField,
		},
	},
	portal::PortalWindow,
	route::RouteStep,
	utilities::{CompassDir, FIELD_RESOLUTION},
};

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
/// Default field cell value of a new [FlowField]
const BITS_DEFAULT: u8 = 0b0000_0000;
/// Flags a pathable field cell
const BITS_PATHABLE: u8 = 0b0001_0000;
/// Flags a field cell that has line-of-sight to the goal
const BITS_HAS_LOS: u8 = 0b0010_0000;
/// Flags a field cell as being the goal
const BITS_GOAL: u8 = 0b0100_0000;
/// Flags a field cell as being a portal to another sector
const BITS_PORTAL_GOAL: u8 = 0b1000_0000;
/// Bit to indicate an impassable cell
const BITS_IMPASSABLE: u8 = 0b1110_0000;
/// Helper for filtering a value for flags
#[allow(dead_code)] // used in tests, might be useful elsewhere
const BITS_FLAG_FILTER: u8 = 0b1111_0000;
/// Helper for filtering a value for directional bits
const BITS_COST_FILTER: u8 = 0b0000_1111;

/// Convert an [CompassDir] to a bit representation
pub fn convert_compass_dir_to_bits_dir(compass_dir: &CompassDir) -> u8 {
	match compass_dir {
		CompassDir::North => BITS_NORTH,
		CompassDir::East => BITS_EAST,
		CompassDir::South => BITS_SOUTH,
		CompassDir::West => BITS_WEST,
		CompassDir::NorthEast => BITS_NORTH_EAST,
		CompassDir::SouthEast => BITS_SOUTH_EAST,
		CompassDir::SouthWest => BITS_SOUTH_WEST,
		CompassDir::NorthWest => BITS_NORTH_WEST,
		CompassDir::Zero => BITS_IMPASSABLE,
	}
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Reflect)]
pub struct FlowField {
	/// One dimensional array of bit vectors
	#[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
	field: [u8; FIELD_RESOLUTION * FIELD_RESOLUTION],
}

impl Default for FlowField {
	fn default() -> Self {
		FlowField {
			field: [BITS_DEFAULT; FIELD_RESOLUTION * FIELD_RESOLUTION],
		}
	}
}

impl Field<u8> for FlowField {
	/// Get a reference to the field array
	fn get(&self) -> &[u8; FIELD_RESOLUTION * FIELD_RESOLUTION] {
		&self.field
	}
	/// Retrieve a field cell value
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u8 {
		self.field[field_cell.as_1d_index()]
	}
	/// Set a field cell to a value
	fn set_field_cell_value(&mut self, value: u8, field_cell: FieldCell) {
		self.field[field_cell.as_1d_index()] = value;
	}
}

impl FlowField {
	/// Init [FlowField] with default values and flags set for goal(s), walls and LOS
	pub fn new(
		route_step: &RouteStep,
		int_field: &IntegrationField,
		prev_int_field: Option<&IntegrationField>,
	) -> FlowField {
		let mut flowfield = FlowField::default();
		set_starting_flags(&mut flowfield, int_field);
		if let Some(window) = route_step.portal() {
			if let Some(neighbour_int) = prev_int_field {
				// read the previous int field leading to this flowfield,
				// align the portal cells of this to point at the best (cheapest)
				// cost in the neighbour
				optimise_portal_direction(&mut flowfield, neighbour_int, window);
			} else {
				// fallback//TODO remove this in a future release as it should never be called now
				set_portal_direction(&mut flowfield, window);
			}
		}

		flowfield
	}
	/// Iterate over each cost in the [IntegrationField] and calculate the flow
	/// value
	pub fn build(&mut self, int_field: &IntegrationField) {
		for (cell_index, flow_value) in self.field.iter_mut().enumerate() {
			calculate_flow_cell(cell_index, flow_value, int_field);
		}
	}
	/// Indicates that a cell is pathable
	pub fn is_pathable(&self, field_cell: &FieldCell) -> bool {
		self.field[field_cell.as_1d_index()] & BITS_PATHABLE == BITS_PATHABLE
	}
	/// Indicates that a cell is the target goal
	pub fn is_goal(&self, field_cell: &FieldCell) -> bool {
		self.field[field_cell.as_1d_index()] & BITS_GOAL == BITS_GOAL
			&& self.is_pathable(field_cell)
	}
	/// Indicates that a cell is a portal goal
	pub fn is_portal_goal(&self, field_cell: &FieldCell) -> bool {
		self.field[field_cell.as_1d_index()] & BITS_PORTAL_GOAL == BITS_PORTAL_GOAL
			&& self.is_pathable(field_cell)
	}
	/// Check if a [FieldCell] has Line-of-Sight to a goal. If so an actor can stop reading [FlowField] and path in a straight line to it
	pub fn has_los(&self, field_cell: &FieldCell) -> bool {
		self.field[field_cell.as_1d_index()] & BITS_HAS_LOS == BITS_HAS_LOS
			&& self.is_pathable(field_cell)
	}
	/// Read directional bits of a cell and get the direction vector
	#[cfg(feature = "2d")]
	pub fn get_2d_dir(&self, field_cell: &FieldCell) -> Option<Vec2> {
		let cell_value = self.field[field_cell.as_1d_index()];
		let bit_dir = cell_value & BITS_COST_FILTER;
		match bit_dir {
			BITS_NORTH => Some(Vec2::new(0.0, 1.0)),
			BITS_EAST => Some(Vec2::new(1.0, 0.0)),
			BITS_SOUTH => Some(Vec2::new(0.0, -1.0)),
			BITS_WEST => Some(Vec2::new(-1.0, 0.0)),
			BITS_NORTH_EAST => Some(Vec2::new(1.0, 1.0)),
			BITS_SOUTH_EAST => Some(Vec2::new(1.0, -1.0)),
			BITS_SOUTH_WEST => Some(Vec2::new(-1.0, -1.0)),
			BITS_NORTH_WEST => Some(Vec2::new(-1.0, 1.0)),
			BITS_DEFAULT => {
				// warn!("Flow cell has no calculation {}", field_cell);
				None
			}
			_ => {
				warn!(
					"First 4 bits of cell are not recognised directions: {}, bits {}",
					field_cell, bit_dir
				);
				None
			}
		}
	}
	/// Read directional bits of a cell and get the direction vector
	#[cfg(feature = "3d")]
	pub fn get_3d_dir(&self, field_cell: &FieldCell) -> Option<Vec3> {
		let cell_value = self.field[field_cell.as_1d_index()];
		let bit_dir = cell_value & BITS_COST_FILTER;
		match bit_dir {
			BITS_NORTH => Some(Vec3::new(0.0, 0.0, -1.0)),
			BITS_EAST => Some(Vec3::new(1.0, 0.0, 0.0)),
			BITS_SOUTH => Some(Vec3::new(0.0, 0.0, 1.0)),
			BITS_WEST => Some(Vec3::new(-1.0, 0.0, 0.0)),
			BITS_NORTH_EAST => Some(Vec3::new(1.0, 0.0, -1.0)),
			BITS_SOUTH_EAST => Some(Vec3::new(1.0, 0.0, 1.0)),
			BITS_SOUTH_WEST => Some(Vec3::new(-1.0, 0.0, 1.0)),
			BITS_NORTH_WEST => Some(Vec3::new(-1.0, 0.0, -1.0)),
			BITS_DEFAULT => {
				// warn!("Flow cell has no calculation {}", field_cell);
				None
			}
			_ => {
				debug!(
					"First 4 bits of cell are not recognised directions: {}, bits {}",
					field_cell, bit_dir
				);
				None
			}
		}
	}
}

/// Interpret flags set in the [IntegrationField] and establish [FlowField] flags for goal(s), impassable walls and Line-of-Sight (LOS)
fn set_starting_flags(flowfield: &mut FlowField, int_field: &IntegrationField) {
	for (i, value) in int_field.get().iter().enumerate() {
		if value & INT_BITS_GOAL == INT_BITS_GOAL {
			// set goal
			flowfield.field[i] |= BITS_GOAL;
			flowfield.field[i] |= BITS_PATHABLE;
			flowfield.field[i] |= BITS_HAS_LOS;
		} else if value & INT_BITS_PORTAL == INT_BITS_PORTAL {
			// set portal goal
			flowfield.field[i] |= BITS_PORTAL_GOAL;
			flowfield.field[i] |= BITS_PATHABLE;
		} else if value & INT_BITS_LOS == INT_BITS_LOS {
			// set los
			flowfield.field[i] |= BITS_HAS_LOS;
			flowfield.field[i] |= BITS_PATHABLE;
		} else if value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE {
			// set impassable
			flowfield.field[i] |= BITS_IMPASSABLE;
		}
	}
}

/// When building the [FlowField] that has portal based goals identify what
/// sector boundary they lie upon and set their directional bits to point into
/// the neighbouring sector
fn set_portal_direction(flowfield: &mut FlowField, window: &PortalWindow) {
	// based on window boundary find bit dir
	let this_boundary = window.get_boundary();
	let dir_bits = convert_compass_dir_to_bits_dir(this_boundary);
	// walk the window and set the dir bits
	for cell_index in window.get_all_window_cells().iter() {
		flowfield.field[*cell_index] |= dir_bits;
	}
}

/// When building the [FlowField] that has portal based goals, identify what
/// sector boundary they lie upon and set their directional bits to point into
/// the neighbouring sector. Use the previous [IntegrationField] to find the
/// cheapest neighbour to point towards
fn optimise_portal_direction(
	flowfield: &mut FlowField,
	prev_int_field: &IntegrationField,
	this_window: &PortalWindow,
) {
	// based on window boundary find bit dir
	let this_boundary = this_window.get_boundary();

	// note these cells run left-right for north and south boundaries, and
	// top-bottom for east-west
	let this_cells = this_window.get_all_window_cells();
	// unit length window so set dir into next sector as normal
	if this_cells.len() == 1 {
		let dir_bits = convert_compass_dir_to_bits_dir(this_boundary);
		flowfield.field[this_cells[0]] |= dir_bits;
		return;
	}
	// get integration cost in the neighbour along the window
	let adjacent_values = match this_boundary {
		CompassDir::North => {
			let mut values = vec![];
			for this_cell in this_cells.iter() {
				let fc = FieldCell::from_index(*this_cell);
				// adj is on south boundary so const row 9
				let adj_cell = FieldCell::new(fc.column, 9);
				values.push(prev_int_field.get_field_cell_value(adj_cell) & INT_FILTER_BITS_COST);
			}
			values
		}
		CompassDir::East => {
			let mut values = vec![];
			for this_cell in this_cells.iter() {
				let fc = FieldCell::from_index(*this_cell);
				// adj is on west boundary so const col 0
				let adj_cell = FieldCell::new(0, fc.row);
				values.push(prev_int_field.get_field_cell_value(adj_cell) & INT_FILTER_BITS_COST);
			}
			values
		}
		CompassDir::South => {
			let mut values = vec![];
			for this_cell in this_cells.iter() {
				let fc = FieldCell::from_index(*this_cell);
				// adj is on north boundary so const row 0
				let adj_cell = FieldCell::new(fc.column, 0);
				values.push(prev_int_field.get_field_cell_value(adj_cell) & INT_FILTER_BITS_COST);
			}
			values
		}
		CompassDir::West => {
			let mut values = vec![];
			for this_cell in this_cells.iter() {
				let fc = FieldCell::from_index(*this_cell);
				// adj is on east boundary so const column 9
				let adj_cell = FieldCell::new(9, fc.row);
				values.push(prev_int_field.get_field_cell_value(adj_cell) & INT_FILTER_BITS_COST);
			}
			values
		}
		_ => panic!(
			"Invalid compass dir {} used for optimising flow portal direction",
			this_boundary
		),
	};

	// work through the cells of the window and set the flow direction
	// towards the cheapest int cost
	match this_boundary {
		CompassDir::North => {
			for (i, this_cell) in this_cells.iter().enumerate() {
				// first one can only look at 2
				if i == 0 {
					// points north
					let n_value = adjacent_values[i];
					// points northeast
					let ne_value = adjacent_values[i + 1];

					if n_value <= ne_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::North);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::NorthEast);
						flowfield.field[*this_cell] |= dir_bits;
					}
				} else if i == this_cells.len() - 1 {
					// last one can only look at 2
					//
					// points north
					let n_value = adjacent_values[i];
					// points northwest
					let nw_value = adjacent_values[i - 1];

					if n_value <= nw_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::North);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::NorthWest);
						flowfield.field[*this_cell] |= dir_bits;
					}
				} else {
					// others can compare 3
					let nw_value = adjacent_values[i - 1];
					let n_value = adjacent_values[i];
					let ne_value = adjacent_values[i + 1];

					if n_value <= nw_value && n_value <= ne_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::North);
						flowfield.field[*this_cell] |= dir_bits;
					} else if nw_value <= n_value && nw_value <= ne_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::NorthWest);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::NorthEast);
						flowfield.field[*this_cell] |= dir_bits;
					}
				}
			}
		}
		CompassDir::East => {
			for (i, this_cell) in this_cells.iter().enumerate() {
				// first one can only look at 2
				if i == 0 {
					// points east
					let e_value = adjacent_values[i];
					// points southeast
					let se_value = adjacent_values[i + 1];

					if e_value <= se_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::East);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::SouthEast);
						flowfield.field[*this_cell] |= dir_bits;
					}
				} else if i == this_cells.len() - 1 {
					// last one can only look at 2
					//
					// points east
					let e_value = adjacent_values[i];
					// points northeast
					let ne_value = adjacent_values[i - 1];

					if e_value <= ne_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::East);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::NorthEast);
						flowfield.field[*this_cell] |= dir_bits;
					}
				} else {
					// others can compare 3
					let ne_value = adjacent_values[i - 1];
					let e_value = adjacent_values[i];
					let se_value = adjacent_values[i + 1];

					if e_value <= ne_value && e_value <= se_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::East);
						flowfield.field[*this_cell] |= dir_bits;
					} else if ne_value <= e_value && ne_value <= se_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::NorthEast);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::SouthEast);
						flowfield.field[*this_cell] |= dir_bits;
					}
				}
			}
		}
		CompassDir::South => {
			for (i, this_cell) in this_cells.iter().enumerate() {
				// first one can only look at 2
				if i == 0 {
					// points south
					let s_value = adjacent_values[i];
					// points southeast
					let se_value = adjacent_values[i + 1];

					if s_value <= se_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::South);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::SouthEast);
						flowfield.field[*this_cell] |= dir_bits;
					}
				} else if i == this_cells.len() - 1 {
					// last one can only look at 2
					//
					// points south
					let s_value = adjacent_values[i];
					// points southwest
					let sw_value = adjacent_values[i - 1];

					if s_value <= sw_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::South);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::SouthWest);
						flowfield.field[*this_cell] |= dir_bits;
					}
				} else {
					// others can compare 3
					let sw_value = adjacent_values[i - 1];
					let s_value = adjacent_values[i];
					let se_value = adjacent_values[i + 1];

					if s_value <= sw_value && s_value <= se_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::South);
						flowfield.field[*this_cell] |= dir_bits;
					} else if sw_value <= s_value && sw_value <= se_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::SouthWest);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::SouthEast);
						flowfield.field[*this_cell] |= dir_bits;
					}
				}
			}
		}
		CompassDir::West => {
			for (i, this_cell) in this_cells.iter().enumerate() {
				// first one can only look at 2
				if i == 0 {
					// points west
					let w_value = adjacent_values[i];
					// points southwest
					let sw_value = adjacent_values[i + 1];

					if w_value <= sw_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::West);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::SouthWest);
						flowfield.field[*this_cell] |= dir_bits;
					}
				} else if i == this_cells.len() - 1 {
					// last one can only look at 2
					//
					// points west
					let w_value = adjacent_values[i];
					// points northwest
					let nw_value = adjacent_values[i - 1];

					if w_value <= nw_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::West);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::NorthWest);
						flowfield.field[*this_cell] |= dir_bits;
					}
				} else {
					// others can compare 3
					let nw_value = adjacent_values[i - 1];
					let w_value = adjacent_values[i];
					let sw_value = adjacent_values[i + 1];

					if w_value <= nw_value && w_value <= sw_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::West);
						flowfield.field[*this_cell] |= dir_bits;
					} else if nw_value <= w_value && nw_value <= sw_value {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::NorthWest);
						flowfield.field[*this_cell] |= dir_bits;
					} else {
						let dir_bits = convert_compass_dir_to_bits_dir(&CompassDir::SouthWest);
						flowfield.field[*this_cell] |= dir_bits;
					}
				}
			}
		}
		_ => panic!(
			"Invalid compass dir {} used for optimising flow portal direction",
			this_boundary
		),
	}
}

/// Compare the neighbours of a cell in the [FlowField] and determine their bits
fn calculate_flow_cell(cell_index: usize, flow_value: &mut u8, int_field: &IntegrationField) {
	// skip if marked as wall or LOS or goal
	if *flow_value & BITS_IMPASSABLE == BITS_IMPASSABLE
		|| *flow_value & BITS_PATHABLE == BITS_PATHABLE
	{
		return;
	}
	// get up to 8 cells around cell_index
	// lookup their integrated-cost value and
	// find the cheapest. That direction defines the flow
	// dir bit to be set
	let this_cell = FieldCell::from_index(cell_index);
	let n_compass_dirs = CompassDir::get_all_cell_neighbours_with_compass_dir(this_cell);

	// if a direction points exactly at a goal just use that
	let mut goal_dir: Option<CompassDir> = None;
	// record best direction found
	let mut best_dir: Option<(CompassDir, u32)> = None;

	// go through all neighbours and find cheapest to set direction bits to
	for (compass_dir, n_cell) in n_compass_dirs.iter() {
		let n_int_value = int_field.get_field_cell_value(*n_cell);
		if n_int_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE {
			// cannot point into wall
			continue;
		}
		// if neighbour is diagonal inspect orthogonal to ensure it's a valid
		// direction. i.e it's not diagonal between two walls
		match compass_dir {
			CompassDir::NorthEast => {
				// check N and E for walls
				if let Some(north_this) = this_cell.get_in_compass_direction(&CompassDir::North, 1)
					&& let Some(east_this) =
						this_cell.get_in_compass_direction(&CompassDir::East, 1)
				{
					let north_this_value = int_field.get_field_cell_value(north_this);
					let east_this_value = int_field.get_field_cell_value(east_this);
					if north_this_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE
						&& east_this_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE
					{
						// diagonal through wall, skip
						continue;
					}
				}
			}
			CompassDir::SouthEast => {
				// check E and S for walls
				if let Some(south_this) = this_cell.get_in_compass_direction(&CompassDir::South, 1)
					&& let Some(east_this) =
						this_cell.get_in_compass_direction(&CompassDir::East, 1)
				{
					let south_this_value = int_field.get_field_cell_value(south_this);
					let east_this_value = int_field.get_field_cell_value(east_this);
					if south_this_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE
						&& east_this_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE
					{
						// diagonal through wall, skip
						continue;
					}
				}
			}
			CompassDir::SouthWest => {
				// check S and W for walls
				if let Some(south_this) = this_cell.get_in_compass_direction(&CompassDir::South, 1)
					&& let Some(west_this) =
						this_cell.get_in_compass_direction(&CompassDir::West, 1)
				{
					let south_this_value = int_field.get_field_cell_value(south_this);
					let west_this_value = int_field.get_field_cell_value(west_this);
					if south_this_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE
						&& west_this_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE
					{
						// diagonal through wall, skip
						continue;
					}
				}
			}
			CompassDir::NorthWest => {
				// check W and N for walls
				if let Some(north_this) = this_cell.get_in_compass_direction(&CompassDir::North, 1)
					&& let Some(west_this) =
						this_cell.get_in_compass_direction(&CompassDir::West, 1)
				{
					let north_this_value = int_field.get_field_cell_value(north_this);
					let west_this_value = int_field.get_field_cell_value(west_this);
					if north_this_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE
						&& west_this_value & INT_BITS_IMPASSABLE == INT_BITS_IMPASSABLE
					{
						// diagonal through wall, skip
						continue;
					}
				}
			}
			_ => {
				// carry on as normal
			}
		}

		if n_int_value & INT_BITS_GOAL == INT_BITS_GOAL
			|| n_int_value & INT_BITS_PORTAL == INT_BITS_PORTAL
		{
			// can point to goal/portal goal
			goal_dir = Some(*compass_dir);
			break;
		}

		// special case where the int value is the default. This means an
		// integrated-cost hasn't been calculated for the cell. This can
		// happen if there's an island, like a ring of walls around clear cells,
		// or if a wall has bisected a sector completely
		if n_int_value & INT_FILTER_BITS_COST == 65535 {
			// in this special case we mark the flow value with the
			// impassable flag
			*flow_value |= BITS_IMPASSABLE;
			break;
		}

		// if this far then just record best dir
		if let Some((o, v)) = &mut best_dir {
			if n_int_value & INT_FILTER_BITS_COST < *v {
				*v = n_int_value & INT_FILTER_BITS_COST;
				*o = *compass_dir;
			}
		} else {
			best_dir = Some((*compass_dir, n_int_value & INT_FILTER_BITS_COST));
		}
	}

	// set the bit direction based on best available
	if let Some(dir) = goal_dir {
		let bits = convert_compass_dir_to_bits_dir(&dir);
		*flow_value |= bits;
		*flow_value |= BITS_PATHABLE;
	} else if let Some((dir, _)) = best_dir {
		let bits = convert_compass_dir_to_bits_dir(&dir);
		*flow_value |= bits;
		*flow_value |= BITS_PATHABLE;
	} else {
		// special case where a cell is entirely encased on walls. It will
		// have no integrated-cost calculation, nothing will point towards
		// it and it can't point at anything. Treat it as wall
		*flow_value |= BITS_IMPASSABLE;
	}
}

/// Indicates that a cell is pathable
pub fn is_pathable(cell_value: u8) -> bool {
	cell_value & BITS_PATHABLE == BITS_PATHABLE
}

/// Indicates that a cell is the target goal
pub fn is_goal(cell_value: u8) -> bool {
	cell_value & BITS_GOAL == BITS_GOAL && is_pathable(cell_value)
}

/// Indicates that a cell is a portal goal
pub fn is_portal_goal(cell_value: u8) -> bool {
	cell_value & BITS_PORTAL_GOAL == BITS_PORTAL_GOAL && is_pathable(cell_value)
}

/// If a cell has direct vision to the goal then the [FlowField] should be
/// disregarded as the actor can move in a straight line to the goal
pub fn has_line_of_sight(cell_value: u8) -> bool {
	cell_value & BITS_HAS_LOS == BITS_HAS_LOS && is_pathable(cell_value)
}

/// Check is a cell value is marked as being an impassable wall
pub fn is_wall(cell_value: u8) -> bool {
	cell_value & BITS_IMPASSABLE == BITS_IMPASSABLE && !is_pathable(cell_value)
}

/// From a pathable [FlowField] cell get the directional [CompassDir] of movement
pub fn get_compass_dir_from_bits(cell_value: u8) -> CompassDir {
	let dir = cell_value & BITS_COST_FILTER;
	match dir {
		BITS_NORTH => CompassDir::North,
		BITS_EAST => CompassDir::East,
		BITS_SOUTH => CompassDir::South,
		BITS_WEST => CompassDir::West,
		BITS_NORTH_EAST => CompassDir::NorthEast,
		BITS_SOUTH_EAST => CompassDir::SouthEast,
		BITS_SOUTH_WEST => CompassDir::SouthWest,
		BITS_NORTH_WEST => CompassDir::NorthWest,
		BITS_DEFAULT => CompassDir::Zero,
		_ => CompassDir::Zero, // _ => panic!("First 4 bits of cell are not recognised directions"),
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use crate::flowfields::{fields::cost_field::CostField, sectors::SectorID};

	use super::*;
	#[test]
	fn default_init() {
		let flow_field = FlowField::default();
		let v = flow_field.get_field_cell_value(FieldCell::new(0, 0));
		assert_eq!(BITS_DEFAULT, v);
	}

	// g = goal
	// X = wall
	// L = LOS
	// b = wave blocked
	//TODO: see note of this diagram in integration field tests
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
	fn starting_flags() {
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

		let mut flowfield = FlowField::default();
		set_starting_flags(&mut flowfield, &int_field);

		let r_c = FieldCell::new(2, 9);
		let r = flowfield.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#010b}, cost: {:#010b}",
			r_c,
			r & BITS_FLAG_FILTER,
			r & BITS_COST_FILTER
		);
		assert!(r & BITS_IMPASSABLE == BITS_IMPASSABLE);

		let r_c = FieldCell::new(4, 8);
		let r = flowfield.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#010b}, cost: {:#010b}",
			r_c,
			r & BITS_FLAG_FILTER,
			r & BITS_COST_FILTER
		);
		assert!(r & BITS_GOAL == BITS_GOAL);

		let r_c = FieldCell::new(4, 7);
		let r = flowfield.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#010b}, cost: {:#010b}",
			r_c,
			r & BITS_FLAG_FILTER,
			r & BITS_COST_FILTER
		);
		assert!(r & BITS_HAS_LOS == BITS_HAS_LOS);
	}

	#[test]
	fn portal_dir_north() {
		let prev_costfield = CostField::default();
		let prev_sector = SectorID::new(1, 0);
		let prev_goal = 25;
		let prev_portal = None;
		let prev_route_step = RouteStep::new(&prev_sector, prev_goal, prev_portal);
		let mut prev_int_field = IntegrationField::init(&prev_costfield, &prev_route_step);
		prev_int_field.build(&prev_costfield);

		let costfield = CostField::default();
		let sector = SectorID::new(1, 1);
		let goal = 0;
		let portal = Some(PortalWindow::new(
			FieldCell::new(0, 0),
			FieldCell::new(0, 0),
			CompassDir::North,
		));
		let route_step = RouteStep::new(&sector, goal, portal);
		let int_field = IntegrationField::init(&costfield, &route_step);

		let flowfield = FlowField::new(&route_step, &int_field, Some(&prev_int_field));

		let r_c = flowfield.field[0];
		assert!(r_c & BITS_NORTH == BITS_NORTH)
	}
	// g = goal
	// X = wall
	// L = LOS
	// b = wave blocked
	//TODO: see note of this diagram in integration field tests
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
	fn flags_after_build() {
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

		let mut flowfield = FlowField::new(&route_step, &int_field, None);
		flowfield.build(&int_field);

		let r_c = FieldCell::new(2, 9);
		let r = flowfield.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#010b}, cost: {:#010b}",
			r_c,
			r & BITS_FLAG_FILTER,
			r & BITS_COST_FILTER
		);
		assert!(r & BITS_IMPASSABLE == BITS_IMPASSABLE);

		let r_c = FieldCell::new(3, 7);
		let r = flowfield.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#010b}, cost: {:#010b}",
			r_c,
			r & BITS_FLAG_FILTER,
			r & BITS_COST_FILTER
		);
		assert!(r & BITS_PATHABLE == BITS_PATHABLE);

		let r_c = FieldCell::new(4, 8);
		let r = flowfield.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#010b}, cost: {:#010b}",
			r_c,
			r & BITS_FLAG_FILTER,
			r & BITS_COST_FILTER
		);
		assert!(r & BITS_GOAL == BITS_GOAL);

		let r_c = FieldCell::new(4, 7);
		let r = flowfield.get_field_cell_value(r_c);
		println!(
			"{} :: flags: {:#010b}, cost: {:#010b}",
			r_c,
			r & BITS_FLAG_FILTER,
			r & BITS_COST_FILTER
		);
		assert!(r & BITS_HAS_LOS == BITS_HAS_LOS);
	}
}
