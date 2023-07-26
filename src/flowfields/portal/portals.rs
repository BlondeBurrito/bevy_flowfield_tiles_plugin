//! A Portal indicates a pathable area from one Sector to another. Each side of a Sector can
//! have multiple portals if a side is 'split' due to an impassable value in the
//! [CostField]. A side that sits along the edge of the map
//! itself cannot have a portal. For example here is a representation of a
//! [CostField] in the top-left corner of a map with the portal [FieldCell]
//! positions labeled with a `P`:
//!
//! ```text
//!  ___________________________________________________________
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  |  1  P
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
//! |  1  |  1  |  1  |  1  | 255 |  1  |  1  |  1  |  1  |  1  P
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  | 255 |  1  |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  | 255 | 255 |  1  |  1  |  1  |  1  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  1  |  1  |  1  |  1  | 255 | 255 | 255 |  1  |  1  |  1  |
//! |_____|__P__|_____|_____|_____|_____|_____|_____|__P__|_____|
//! ```
//!
//! When an agent needs to path somewhere it is initially given a path based on moving from one portal
//! to another portal/end sector. This ensures responsiveness so when a player issues a movement order
//! the agent can immediately starts pathing. In the background the other components of the Flowfields can
//! calcualte a perfect path which can then supersede using portals to path when it's ready

use crate::prelude::*;

/// Portals contains an array of length 4 (one element for each side of a sector) where the values are lists of the portals. The elements correspond to Ordinals in a strict ordering of `0..=3 == North, East,
/// South, West`
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Debug, Clone)]
pub struct Portals([Vec<FieldCell>; 4]);

impl Portals {
	/// Get a reference to the array list of [FieldCell]
	#[cfg(not(tarpaulin_include))]
	pub fn get(&self) -> &[Vec<FieldCell>; 4] {
		&self.0
	}
	/// Get a mutable reference to the array list of [FieldCell]
	#[cfg(not(tarpaulin_include))]
	pub fn get_mut(&mut self) -> &mut [Vec<FieldCell>; 4] {
		&mut self.0
	}
	/// Get a reference to the list of [FieldCell]s along the side of a sector defined by the [Ordinal]
	#[cfg(not(tarpaulin_include))]
	pub fn get_portals_for_side(&self, ordinal: &Ordinal) -> &Vec<FieldCell> {
		match ordinal {
			Ordinal::North => &self.0[0],
			Ordinal::East => &self.0[1],
			Ordinal::South => &self.0[2],
			Ordinal::West => &self.0[3],
			_ => panic!(
				"Portals ordinal can only be North, East, South or West. Asked for {:?}",
				ordinal
			),
		}
	}
	/// Get a mutable reference to the list of [FieldCell]s along the side of a sector defined
	/// by the [Ordinal]
	pub fn get_portals_for_side_mut(&mut self, ordinal: &Ordinal) -> &mut Vec<FieldCell> {
		match ordinal {
			Ordinal::North => &mut self.0[0],
			Ordinal::East => &mut self.0[1],
			Ordinal::South => &mut self.0[2],
			Ordinal::West => &mut self.0[3],
			_ => panic!(
				"Portals ordinal can only be North, East, South or West. Asked for {:?}",
				ordinal
			),
		}
	}
	/// Remove all [FieldCell] elements from the lists of [FieldCell]
	fn clear(&mut self) {
		for vec in self.0.iter_mut() {
			vec.clear();
		}
	}
	/// When a sectors [CostField] is updated the portal [FieldCell]s of the sector and
	/// its neighbours may no longer be valid so they should be recalculated.
	///
	/// Special care must be taken when a neighbouring [CostField] has
	/// impassable values of `255` along the boundary edge as these will create multiple [Portals]
	/// within the neighbour. When building [Portals] we inspect the sector neighbours to ensure that
	/// the [Portals] between neighbours are positioned equally and each sector has the same number of
	/// them alonog the boundary.
	///
	/// In this example the left sector has two cost fields denoted `x` meaning impassable 255. When
	/// calculating the [Portals] of the middle sector rather than creating one
	/// central portal [FieldCell]
	/// along that boundary we instead need to create two portal [FieldCell]s to match the layout of the
	/// adjacent sector
	/// ```text
	/// _______________________________
	/// |         P         |         |
	/// |        x|         |         |
	/// |        x|         |         |
	/// |         P         |         |
	/// |_________|_________|_________|
	/// ```
	pub fn recalculate_portals(
		&mut self,
		sector_cost_fields: &SectorCostFields,
		sector_id: &SectorID,
		map_dimensions: &MapDimensions
	) {
		self.clear();
		// there are up to 4 lists of [FieldCell]s for a given sector, in case this sector being
		// updated is on a boundary we need to determine the valid elements of [Portals] that
		// should be updated
		let valid_ordinals_for_this_sector: Vec<(Ordinal, SectorID)> =
		map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(
				sector_id,
			);
		// moving in a clockwise fashion around the valid ordinals of the boundary sector movement
		// we inspect the [CostField] values to calculate the portals along each valid sector side
		let cost_field = sector_cost_fields
			.get()
			.get(sector_id)
			.expect("Invalid sector id");
		for (ord, adjoining_sector_id) in valid_ordinals_for_this_sector.iter() {
			match ord {
				Ordinal::North => {
					let portal_nodes = self.get_portals_for_side_mut(ord);
					let column_range = 0..FIELD_RESOLUTION;
					let fixed_row = 0;
					let adjoining_cost_field =
						sector_cost_fields.get().get(adjoining_sector_id).unwrap();
					// walk along the side of the field
					let mut neighbouring_pathable = Vec::new();
					for i in column_range {
						let field_cost =
							cost_field.get_field_cell_value(FieldCell::new(i, fixed_row));
						let adjacent_field_cost = adjoining_cost_field
							.get_field_cell_value(FieldCell::new(i, FIELD_RESOLUTION - 1));
						if field_cost != 255 && adjacent_field_cost != 255 {
							// a pathable point along the edge so we record it to be
							// published later as a FieldCell
							neighbouring_pathable.push((i, fixed_row));
						} else {
							// if a length along the edge was previously calculated then publish
							// it as FieldCell
							if !neighbouring_pathable.is_empty() {
								// find the most centre like cell for this portal window
								let mut column_index_sum = 0;
								for (m, _) in neighbouring_pathable.iter() {
									column_index_sum += m;
								}
								let portal_midpoint_column =
									column_index_sum / neighbouring_pathable.len();
								portal_nodes
									.push(FieldCell::new(portal_midpoint_column, fixed_row));
								// clear the recording list so any other portals along the side can be built
								neighbouring_pathable.clear();
							}
						}
					}
					// if the side doesn't end with a cost field of 255 then there's one more portal window that needs to be published after iterating over the side
					if !neighbouring_pathable.is_empty() {
						// find the most centre like cell for this portal window
						let mut column_index_sum = 0;
						for (i, _) in neighbouring_pathable.iter() {
							column_index_sum += i;
						}
						let portal_midpoint_column = column_index_sum / neighbouring_pathable.len();
						portal_nodes.push(FieldCell::new(portal_midpoint_column, fixed_row));
						// clear the recording list so any other portals along the side can be built
						neighbouring_pathable.clear();
					}
				}
				Ordinal::East => {
					let portal_nodes = self.get_portals_for_side_mut(ord);
					let fixed_column = FIELD_RESOLUTION - 1;
					let row_range = 0..FIELD_RESOLUTION;
					let adjoining_cost_field =
						sector_cost_fields.get().get(adjoining_sector_id).unwrap();
					// walk along the side of the field
					let mut neighbouring_pathable = Vec::new();
					for j in row_range {
						let field_cost =
							cost_field.get_field_cell_value(FieldCell::new(fixed_column, j));
						let adjacent_field_cost =
							adjoining_cost_field.get_field_cell_value(FieldCell::new(0, j));
						if field_cost != 255 && adjacent_field_cost != 255 {
							// a pathable point along the edge so we record it to be
							// published later as a FieldCell
							neighbouring_pathable.push((fixed_column, j));
						} else {
							// if a length along the edge was previously calculated then publish
							// it as FieldCell
							if !neighbouring_pathable.is_empty() {
								// find the most centre like cell for this portal window
								let mut row_index_sum = 0;
								for (_, n) in neighbouring_pathable.iter() {
									row_index_sum += n;
								}
								let portal_midpoint_row =
									row_index_sum / neighbouring_pathable.len();
								portal_nodes
									.push(FieldCell::new(fixed_column, portal_midpoint_row));
								// clear the recording list so any other portals along the side can be built
								neighbouring_pathable.clear();
							}
						}
					}
					// if the side doesn't end with a cost field of 255 then there's one more portal window that needs to be published after iterating over the side
					if !neighbouring_pathable.is_empty() {
						// find the most centre like cell for this portal window
						let mut row_index_sum = 0;
						for (_, n) in neighbouring_pathable.iter() {
							row_index_sum += n;
						}
						let portal_midpoint_row = row_index_sum / neighbouring_pathable.len();
						portal_nodes.push(FieldCell::new(fixed_column, portal_midpoint_row));
						// clear the recording list so any other portals along the side can be built
						neighbouring_pathable.clear();
					}
				}
				Ordinal::South => {
					let portal_nodes = self.get_portals_for_side_mut(ord);
					let column_range = 0..FIELD_RESOLUTION;
					let fixed_row = FIELD_RESOLUTION - 1;
					let adjoining_cost_field =
						sector_cost_fields.get().get(adjoining_sector_id).unwrap();
					// walk along the side of the field
					let mut neighbouring_pathable = Vec::new();
					for i in column_range {
						let field_cost =
							cost_field.get_field_cell_value(FieldCell::new(i, fixed_row));
						let adjacent_field_cost =
							adjoining_cost_field.get_field_cell_value(FieldCell::new(i, 0));
						if field_cost != 255 && adjacent_field_cost != 255 {
							// a pathable point along the edge so we record it to be
							// published later as a FieldCell
							neighbouring_pathable.push((i, fixed_row));
						} else {
							// if a length along the edge was previously calculated then publish
							// it as FieldCell
							if !neighbouring_pathable.is_empty() {
								// find the most centre like cell for this portal window
								let mut column_index_sum = 0;
								for (m, _) in neighbouring_pathable.iter() {
									column_index_sum += m;
								}
								let portal_midpoint_column =
									column_index_sum / neighbouring_pathable.len();
								portal_nodes
									.push(FieldCell::new(portal_midpoint_column, fixed_row));
								// clear the recording list so any other portals along the side can be built
								neighbouring_pathable.clear();
							}
						}
					}
					// if the side doesn't end with a cost field of 255 then there's one more portal window that needs to be published after iterating over the side
					if !neighbouring_pathable.is_empty() {
						// find the most centre like cell for this portal window
						let mut column_index_sum = 0;
						for (i, _) in neighbouring_pathable.iter() {
							column_index_sum += i;
						}
						let portal_midpoint_column = column_index_sum / neighbouring_pathable.len();
						portal_nodes.push(FieldCell::new(portal_midpoint_column, fixed_row));
						// clear the recording list so any other portals along the side can be built
						neighbouring_pathable.clear();
					}
				}
				Ordinal::West => {
					let portal_nodes = self.get_portals_for_side_mut(ord);
					let fixed_column = 0;
					let row_range = 0..FIELD_RESOLUTION;
					let adjoining_cost_field =
						sector_cost_fields.get().get(adjoining_sector_id).unwrap();
					// walk along the side of the field
					let mut neighbouring_pathable = Vec::new();
					for j in row_range {
						let field_cost =
							cost_field.get_field_cell_value(FieldCell::new(fixed_column, j));
						let adjacent_field_cost = adjoining_cost_field
							.get_field_cell_value(FieldCell::new(FIELD_RESOLUTION - 1, j));
						if field_cost != 255 && adjacent_field_cost != 255 {
							// a pathable point along the edge so we record it to be
							// published later as a FieldCell
							neighbouring_pathable.push((fixed_column, j));
						} else {
							// if a length along the edge was previously calculated then publish
							// it as FieldCell
							if !neighbouring_pathable.is_empty() {
								// find the most centre like cell for this portal window
								let mut row_index_sum = 0;
								for (_, n) in neighbouring_pathable.iter() {
									row_index_sum += n;
								}
								let portal_midpoint_row =
									row_index_sum / neighbouring_pathable.len();
								portal_nodes
									.push(FieldCell::new(fixed_column, portal_midpoint_row));
								// clear the recording list so any other portals along the side can be built
								neighbouring_pathable.clear();
							}
						}
					}
					// if the side doesn't end with a cost field of 255 then there's one more portal window that needs to be published after iterating over the side
					if !neighbouring_pathable.is_empty() {
						// find the most centre like cell for this portal window
						let mut row_index_sum = 0;
						for (_, n) in neighbouring_pathable.iter() {
							row_index_sum += n;
						}
						let portal_midpoint_row = row_index_sum / neighbouring_pathable.len();
						portal_nodes.push(FieldCell::new(fixed_column, portal_midpoint_row));
						// clear the recording list so any other portals along the side can be built
						neighbouring_pathable.clear();
					}
				}
				_ => panic!(
					"Portal ordinals can only be North, East, South or West. Asked for {:?}",
					ord
				),
			};
		}
	}
	/// A [FieldCell] represents the midpoint of a segment along a boundary, for smooth pathfinding any field cell along the segemnt should be a viable goal node when calculating an [IntegrationField]. This takes inspects the `portal_id` within the given `sector_id` and build a list of field cells which comprise the true dimension of the portal
	pub fn expand_portal_into_goals(
		&self,
		sector_cost_fields: &SectorCostFields,
		sector_id: &SectorID,
		portal_id: &FieldCell,
		neighbour_sector_id: &SectorID,
		map_dimensions: &MapDimensions
	) -> Vec<FieldCell> {
		// find the bounudary the portal sit along
		let mut boundary_ordinals = portal_id.get_boundary_ordinal_from_field_cell();
		// if it's in a corner then it could apply to two boundaries, narrow it down so we know which boundary to walk
		if boundary_ordinals.len() > 1 {
			let valid_ordinals_for_this_sector: Vec<(Ordinal, SectorID)> =
			map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(
					sector_id,
				);
			'outer: for (ordinal, id) in valid_ordinals_for_this_sector.iter() {
				if id == neighbour_sector_id {
					boundary_ordinals.retain(|o| o == ordinal);
					break 'outer;
				}
			}
			if boundary_ordinals.len() > 1 {
				panic!("Sector {:?} does not have a neighbour at {:?} while inspecting portal {:?}. This suggests that a portal exists on a sector boundary where it shouldn't, i.e this sector is along an edge of the world and the portal is on a boundary leading to nowhere", sector_id, neighbour_sector_id, portal_id);
			}
		}
		let boundary_ordinal = boundary_ordinals.first().unwrap();
		let mut goals: Vec<FieldCell> = Vec::new();
		// the portal itself is a goal
		goals.push(*portal_id);
		// from the portal walk either left/right or up/down depending on the ordinal
		// until an impassable cost field value is found
		let this_cost_field = sector_cost_fields.get().get(sector_id).unwrap();
		let adjoining_cost_field = sector_cost_fields.get().get(neighbour_sector_id).unwrap();
		match boundary_ordinal {
			Ordinal::North => {
				// walk left from the portal
				let mut step = 1;
				'left: while portal_id.get_column().checked_sub(step).is_some() {
					let left = FieldCell::new(portal_id.get_column() - step, portal_id.get_row());
					// check whether cell or adjoining cell is impassable
					let left_cost = this_cost_field
						.get_field_cell_value(FieldCell::new(left.get_column(), left.get_row()));
					let neighbour_cost = adjoining_cost_field.get_field_cell_value(FieldCell::new(
						left.get_column(),
						FIELD_RESOLUTION - 1,
					));
					if left_cost != 255 && neighbour_cost != 255 {
						goals.push(left);
						step += 1;
					} else {
						// portal length cannot go any further
						break 'left;
					}
				}
				// walk right from the portal
				let mut step = 1;
				'right: while portal_id.get_column() + step < FIELD_RESOLUTION {
					let right = FieldCell::new(portal_id.get_column() + step, portal_id.get_row());
					// check whether cell or adjoining cell is impassable
					let right_cost = this_cost_field.get_field_cell_value(right);
					let neighbour_cost = adjoining_cost_field.get_field_cell_value(FieldCell::new(
						right.get_column(),
						FIELD_RESOLUTION - 1,
					));
					if right_cost != 255 && neighbour_cost != 255 {
						goals.push(right);
						step += 1;
					} else {
						// portal length cannot go any further
						break 'right;
					}
				}
			}
			Ordinal::East => {
				// walk up from the portal
				let mut step = 1;
				'up: while portal_id.get_row().checked_sub(step).is_some() {
					let up = FieldCell::new(portal_id.get_column(), portal_id.get_row() - step);
					// check whether cell or adjoining cell is impassable
					let up_cost = this_cost_field.get_field_cell_value(up);
					let neighbour_cost =
						adjoining_cost_field.get_field_cell_value(FieldCell::new(0, up.get_row()));
					if up_cost != 255 && neighbour_cost != 255 {
						goals.push(up);
						step += 1;
					} else {
						// portal length cannot go any further
						break 'up;
					}
				}
				// walk down from the portal
				let mut step = 1;
				'down: while portal_id.get_row() + step < FIELD_RESOLUTION {
					let down = FieldCell::new(portal_id.get_column(), portal_id.get_row() + step);
					// check whether cell or adjoining cell is impassable
					let right_cost = this_cost_field.get_field_cell_value(down);
					let neighbour_cost = adjoining_cost_field
						.get_field_cell_value(FieldCell::new(0, down.get_row()));
					if right_cost != 255 && neighbour_cost != 255 {
						goals.push(down);
						step += 1;
					} else {
						// portal length cannot go any further
						break 'down;
					}
				}
			}
			Ordinal::South => {
				// walk left from the portal
				let mut step = 1;
				'left: while portal_id.get_column().checked_sub(step).is_some() {
					let left = FieldCell::new(portal_id.get_column() - step, portal_id.get_row());
					// check whether cell or adjoining cell is impassable
					let left_cost = this_cost_field.get_field_cell_value(left);
					let neighbour_cost = adjoining_cost_field
						.get_field_cell_value(FieldCell::new(left.get_column(), 0));
					if left_cost != 255 && neighbour_cost != 255 {
						goals.push(left);
						step += 1;
					} else {
						// portal length cannot go any further
						break 'left;
					}
				}
				// walk right from the portal
				let mut step = 1;
				'right: while portal_id.get_column() + step < FIELD_RESOLUTION {
					let right = FieldCell::new(portal_id.get_column() + step, portal_id.get_row());
					// check whether cell or adjoining cell is impassable
					let right_cost = this_cost_field.get_field_cell_value(right);
					let neighbour_cost = adjoining_cost_field
						.get_field_cell_value(FieldCell::new(right.get_column(), 0));
					if right_cost != 255 && neighbour_cost != 255 {
						goals.push(right);
						step += 1;
					} else {
						// portal length cannot go any further
						break 'right;
					}
				}
			}
			Ordinal::West => {
				// walk up from the portal
				let mut step = 1;
				'up: while portal_id.get_row().checked_sub(step).is_some() {
					let up = FieldCell::new(portal_id.get_column(), portal_id.get_row() - step);
					// check whether cell or adjoining cell is impassable
					let up_cost = this_cost_field.get_field_cell_value(up);
					let neighbour_cost = adjoining_cost_field
						.get_field_cell_value(FieldCell::new(FIELD_RESOLUTION - 1, up.get_row()));
					if up_cost != 255 && neighbour_cost != 255 {
						goals.push(up);
						step += 1;
					} else {
						// portal length cannot go any further
						break 'up;
					}
				}
				// walk down from the portal
				let mut step = 1;
				'down: while portal_id.get_row() + step < FIELD_RESOLUTION {
					let down = FieldCell::new(portal_id.get_column(), portal_id.get_row() + step);
					// check whether cell or adjoining cell is impassable
					let right_cost = this_cost_field.get_field_cell_value(down);
					let neighbour_cost = adjoining_cost_field
						.get_field_cell_value(FieldCell::new(FIELD_RESOLUTION - 1, down.get_row()));
					if right_cost != 255 && neighbour_cost != 255 {
						goals.push(down);
						step += 1;
					} else {
						// portal length cannot go any further
						break 'down;
					}
				}
			}
			_ => panic!(
				"Invalid Ordinal {:?} for boundary walking",
				boundary_ordinal
			),
		}
		goals
	}
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use crate::flowfields::sectors::sector_portals::SectorPortals;

use super::*;
	#[test]
	fn portals_top_left_sector() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let mut sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let sector_id = SectorID::new(0, 0);
		let cost_field = sector_cost_fields.get_mut().get_mut(&sector_id).unwrap();
		// switch some fields to impassable
		cost_field.set_field_cell_value(255, FieldCell::new(9, 5));
		cost_field.set_field_cell_value(255, FieldCell::new(0, 9));
		let mut portals = Portals::default();
		portals.recalculate_portals(&sector_cost_fields, &sector_id, &map_dimensions);
		let northern_side_portal_count = 0;
		let eastern_side_portal_count = 2;
		let southern_side_portal_count = 1;
		let western_side_portal_count = 0;
		assert_eq!(northern_side_portal_count, portals.0[0].len());
		assert_eq!(eastern_side_portal_count, portals.0[1].len());
		assert_eq!(southern_side_portal_count, portals.0[2].len());
		assert_eq!(western_side_portal_count, portals.0[3].len());
	}
	#[test]
	fn portals_top_middle_sector() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let mut sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let sector_id = SectorID::new(1, 0);
		let cost_field = sector_cost_fields.get_mut().get_mut(&sector_id).unwrap();
		// switch some fields to impassable
		cost_field.set_field_cell_value(255, FieldCell::new(9, 5));
		cost_field.set_field_cell_value(255, FieldCell::new(0, 9));
		let mut portals = Portals::default();
		portals.recalculate_portals(&sector_cost_fields, &sector_id, &map_dimensions);
		let northern_side_portal_count = 0;
		let eastern_side_portal_count = 2;
		let southern_side_portal_count = 1;
		let western_side_portal_count = 1;
		assert_eq!(northern_side_portal_count, portals.0[0].len());
		assert_eq!(eastern_side_portal_count, portals.0[1].len());
		assert_eq!(southern_side_portal_count, portals.0[2].len());
		assert_eq!(western_side_portal_count, portals.0[3].len());
	}
	#[test]
	fn portals_centre_sector() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let mut sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let sector_id = SectorID::new(1, 1);
		let cost_field = sector_cost_fields.get_mut().get_mut(&sector_id).unwrap();
		// switch some fields to impassable
		cost_field.set_field_cell_value(255, FieldCell::new(9, 5));
		cost_field.set_field_cell_value(255, FieldCell::new(0, 9));
		let mut portals = Portals::default();
		portals.recalculate_portals(&sector_cost_fields, &sector_id, &map_dimensions);
		let northern_side_portal_count = 1;
		let eastern_side_portal_count = 2;
		let southern_side_portal_count = 1;
		let western_side_portal_count = 1;
		assert_eq!(northern_side_portal_count, portals.0[0].len());
		assert_eq!(eastern_side_portal_count, portals.0[1].len());
		assert_eq!(southern_side_portal_count, portals.0[2].len());
		assert_eq!(western_side_portal_count, portals.0[3].len());
	}
	#[test]
	fn portals_bottom_middle_sector() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let mut sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let sector_id = SectorID::new(1, 2);
		let cost_field = sector_cost_fields.get_mut().get_mut(&sector_id).unwrap();
		// switch some fields to impassable
		cost_field.set_field_cell_value(255, FieldCell::new(4, 0));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 0));
		cost_field.set_field_cell_value(255, FieldCell::new(9, 5));
		cost_field.set_field_cell_value(255, FieldCell::new(0, 9));
		let mut portals = Portals::default();
		portals.recalculate_portals(&sector_cost_fields, &sector_id, &map_dimensions);
		let northern_side_portal_count = 3;
		let eastern_side_portal_count = 2;
		let southern_side_portal_count = 0;
		let western_side_portal_count = 1;
		assert_eq!(northern_side_portal_count, portals.0[0].len());
		assert_eq!(eastern_side_portal_count, portals.0[1].len());
		assert_eq!(southern_side_portal_count, portals.0[2].len());
		assert_eq!(western_side_portal_count, portals.0[3].len());
	}
	#[test]
	fn verify_rebuilding() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let mut sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, &map_dimensions)
		}
		
		// the current portals
		// _______________________________
		// |         |         |         |
		// |         |         |         |
		// |         P         P         |
		// |         |         |         |
		// |____P____|____P____|____P____|
		// |         |         |         |
		// |         |         |         |
		// |         P         P         |
		// |         |         |         |
		// |____P____|____P____|____P____|
		// |         |         |         |
		// |         |         |         |
		// |         P         P         |
		// |         |         |         |
		// |_________|_________|_________|
		let pre_first = sector_portals.get_mut().get_mut(&SectorID::new(0, 0)).unwrap().clone();
		let pre_second = sector_portals.get_mut().get_mut(&SectorID::new(0, 1)).unwrap().clone();
		println!("Pre update portals {:?}", pre_first);
		println!("Pre update portals {:?}", pre_second);
		// update the top-left CostFields and calculate new portals
		let mutated_sector_id = SectorID::new(0, 0);
		let field = sector_cost_fields.get_mut().get_mut(&mutated_sector_id).unwrap();
		field.set_field_cell_value(255, FieldCell::new(4, 9));
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, &map_dimensions);

		let post_first = sector_portals.get_mut().get_mut(&mutated_sector_id).unwrap().clone();
		let post_second = sector_portals.get_mut().get_mut(&SectorID::new(0, 1)).unwrap().clone();
		println!("Updated portals {:?}", post_first);
		println!("Updated portals {:?}", post_second);
		// This produces a new representation with an extra portal, `x` denotes the impassable point
		// just inserted
		// _______________________________
		// |         |         |         |
		// |         |         |         |
		// |         P         P         |
		// |         |         |         |
		// |_P__x_P__|____P____|____P____|
		// |         |         |         |
		// |         |         |         |
		// |         P         P         |
		// |         |         |         |
		// |____P____|____P____|____P____|
		// |         |         |         |
		// |         |         |         |
		// |         P         P         |
		// |         |         |         |
		// |_________|_________|_________|

		// ensure new portals are correct
		let actual_first = [
			vec![],
			vec![FieldCell::new(9, 4)],
			vec![FieldCell::new(1, 9), FieldCell::new(7, 9)],
			vec![]
			];
		let actual_second = [
			vec![FieldCell::new(1, 0), FieldCell::new(7, 0)],
			vec![FieldCell::new(9, 4)],
			vec![FieldCell::new(4, 9)],
			vec![]
			];
		assert_eq!(actual_first[2], post_first.get()[2]);
		assert_eq!(actual_second[0], post_second.get()[0]);
	}
	#[test]
	fn expand_portal_goals_north() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, &map_dimensions)
		}
		let sector_id = SectorID::new(1, 1);
		let portal_id = FieldCell::new(4, 0);
		let neighbour_sector_id = SectorID::new(1, 0);
		let goals = sector_portals.get().get(&sector_id).unwrap().expand_portal_into_goals(&sector_cost_fields, &sector_id, &portal_id, &neighbour_sector_id, &map_dimensions);

		let actual = vec![
			FieldCell::new(4, 0), FieldCell::new(3, 0), FieldCell::new(2, 0), FieldCell::new(1, 0), FieldCell::new(0, 0), FieldCell::new(5, 0), FieldCell::new(6, 0), FieldCell::new(7, 0), FieldCell::new(8, 0), FieldCell::new(9, 0)
			];
		assert_eq!(actual, goals);
	}
	#[test]
	fn expand_portal_goals_east() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, &map_dimensions)
		}
		let sector_id = SectorID::new(1, 1);
		let portal_id = FieldCell::new(9, 4);
		let neighbour_sector_id = SectorID::new(2, 1);
		let goals = sector_portals.get().get(&sector_id).unwrap().expand_portal_into_goals(&sector_cost_fields, &sector_id, &portal_id, &neighbour_sector_id, &map_dimensions);

		let actual = vec![
			FieldCell::new(9, 4), FieldCell::new(9, 3), FieldCell::new(9, 2), FieldCell::new(9, 1), FieldCell::new(9, 0), FieldCell::new(9, 5), FieldCell::new(9, 6),FieldCell::new (9, 7), FieldCell::new(9, 8), FieldCell::new(9, 9)
			];
		assert_eq!(actual, goals);
	}
	#[test]
	fn expand_portal_goals_south() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, &map_dimensions)
		}
		let sector_id = SectorID::new(1, 1);
		let portal_id = FieldCell::new(4, 9);
		let neighbour_sector_id = SectorID::new(1, 2);
		let goals = sector_portals.get().get(&sector_id).unwrap().expand_portal_into_goals(&sector_cost_fields, &sector_id, &portal_id, &neighbour_sector_id, &map_dimensions);

		let actual = vec![
			FieldCell::new(4, 9), FieldCell::new(3, 9), FieldCell::new(2, 9), FieldCell::new(1, 9), FieldCell::new(0, 9), FieldCell::new(5, 9), FieldCell::new(6, 9), FieldCell::new(7, 9), FieldCell::new(8, 9), FieldCell::new(9, 9)
			];
		assert_eq!(actual, goals);
	}
	#[test]
	fn expand_portal_goals_west() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, &map_dimensions)
		}
		let sector_id = SectorID::new(1, 1);
		let portal_id = FieldCell::new(0, 4);
		let neighbour_sector_id = SectorID::new(0, 1);
		let goals = sector_portals.get().get(&sector_id).unwrap().expand_portal_into_goals(&sector_cost_fields, &sector_id, &portal_id, &neighbour_sector_id, &map_dimensions);

		let actual = vec![
			FieldCell::new(0, 4), FieldCell::new(0, 3), FieldCell::new(0, 2), FieldCell::new(0, 1), FieldCell::new(0, 0), FieldCell::new(0, 5), FieldCell::new(0, 6), FieldCell::new(0, 7), FieldCell::new(0, 8), FieldCell::new(0, 9)
			];
		assert_eq!(actual, goals);
	}
	#[test]
	fn expand_portal_goals_short_local() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let sector_id = SectorID::new(1, 1);
		let neighbour_sector_id = SectorID::new(1, 0);
		let mut sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		sector_cost_fields.get_mut().get_mut(&sector_id).unwrap().set_field_cell_value(255, FieldCell::new(3, 0));

		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, &map_dimensions)
		}
		
		let portal_id = FieldCell::new(1, 0);
		let goals = sector_portals.get().get(&sector_id).unwrap().expand_portal_into_goals(&sector_cost_fields, &sector_id, &portal_id, &neighbour_sector_id, &map_dimensions);

		let actual = vec![
			FieldCell::new(1, 0), FieldCell::new(0, 0), FieldCell::new(2, 0)
			];
		assert_eq!(actual, goals);
	}
	#[test]
	fn expand_portal_goals_short_adjacent() {
		let map_dimensions = MapDimensions::new(30, 30, 10);
		let sector_id = SectorID::new(1, 1);
		let neighbour_sector_id = SectorID::new(1, 0);
		let mut sector_cost_fields = SectorCostFields::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		sector_cost_fields.get_mut().get_mut(&neighbour_sector_id).unwrap().set_field_cell_value(255, FieldCell::new(3, 9));

		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, &map_dimensions)
		}
		
		let portal_id = FieldCell::new(1, 0);
		let goals = sector_portals.get().get(&sector_id).unwrap().expand_portal_into_goals(&sector_cost_fields, &sector_id, &portal_id, &neighbour_sector_id, &map_dimensions);

		let actual = vec![
			FieldCell::new(1, 0), FieldCell::new(0, 0), FieldCell::new(2, 0)
			];
		assert_eq!(actual, goals);
	}
}
