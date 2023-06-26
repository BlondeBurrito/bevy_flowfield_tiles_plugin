//! A Portal indicates a pathable area from one [Sector] to another. Each side of a [Sector] can
//! have multiple portals if a side is 'split' due to an impassable value in the
//! [crate::flowfields::cost_fields::CostFields]. A side that sits along the edge of the map
//! itself cannot have a portal. For example here is a representation of a
//! [crate::flowfields::cost_fields::CostFields] in the top-left corner of a map with the [PortalNode]
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
//! the agent immediately starts pathing. In the background the other components of the Flowfields can
//! calcualte a perfect path which can then supersede using portals to path when it's ready

use bevy::prelude::*;

use crate::flowfields::{
	sectors::{get_ordinal_and_ids_of_neighbouring_sectors, SectorCostFields},
	Ordinal, FIELD_RESOLUTION, SECTOR_RESOLUTION,
};

/// A PortalNode indicates the `(column, row)`position in its local sector that acts as a window
/// into a neighbouring sector
#[derive(Clone, Debug, PartialEq)]
pub struct PortalNode((u32, u32));

impl PortalNode {
	pub fn new(column: u32, row: u32) -> Self {
		PortalNode((column, row))
	}
	/// Get the local `(column, row)` position of a portal in the associated sector
	pub fn get_column_row(&self) -> (u32, u32) {
		self.0
	}
}

/// Portals contains an array of length 4 (one element for each side of a sector) where the values are lists of the portals. The elements correspond to Ordinals in a strict ordering of `0..=3 == North, East,
/// South, West`
#[derive(Default, Debug, Clone)]
pub struct Portals([Vec<PortalNode>; 4]);

impl Portals {
	/// Get a reference to the array list of [PortalNode]
	pub fn get(&self) -> &[Vec<PortalNode>; 4] {
		&self.0
	}
	/// Get a mutable reference to the array list of [PortalNode]
	pub fn get_mut(&mut self) -> &mut [Vec<PortalNode>; 4] {
		&mut self.0
	}
	/// Get a reference to the list of [PortalNode]s along the side of a sector defined by the [Ordinal]
	pub fn get_portals_for_side(&self, ordinal: &Ordinal) -> &Vec<PortalNode> {
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
	/// Get a mutable reference to the list of [PortalNode]s along the side of a sector defined
	/// by the [Ordinal]
	pub fn get_portals_for_side_mut(&mut self, ordinal: &Ordinal) -> &mut Vec<PortalNode> {
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
	/// Remove all [PortalNode] elements from the lists of [PortalNode]
	fn clear(&mut self) {
		for vec in self.0.iter_mut() {
			vec.clear();
		}
	}
	/// When a sectors [crate::flowfields::cost_fields::CostFields] is updated the [PortalNode]s of the sector and
	/// its neighbours may no longer be valid so they should be recalculated.
	///
	/// Special care must be taken when a neighbouring [crate::flowfields::cost_fields::CostFields] has
	/// impassable values of `255` along the boundary edge as these will create multiple [Portals]
	/// within the neighbour. When building [Portals] we inspect the sector neighbours to ensure that
	/// the [Portals] between neighbours are positioned equally and each sector has the same number of
	/// them alonog the boundary.
	///
	/// In this example the left sector has two cost fields denoted `x` meaning impassable 255. When
	/// calculating the [Portals] of the middle sector rather than creating one central [PortalNode]
	/// along that boundary we instead need to create two [PortalNode]s to match the layout of the
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
		sector_id: &(u32, u32),
		map_x_dimension: u32,
		map_z_dimension: u32,
	) {
		self.clear();
		// there are up to 4 lists of [PortalNode]s for a given sector, in case this sector being
		// updated is on a boundary we need to determine the valid elements of [Portals] that
		// should be updated
		let valid_ordinals_for_this_sector: Vec<(Ordinal, (u32, u32))> =
			get_ordinal_and_ids_of_neighbouring_sectors(
				&sector_id,
				map_x_dimension,
				map_z_dimension,
			);
		// moving in a clockwise fashion around the valid ordinals of the boundary sector movement
		// we inspect the [crate::flowfields::cost_fields::CostFields] values to calculate the portals along each valid sector side
		let cost_fields = sector_cost_fields
			.get()
			.get(&sector_id)
			.expect("Invalid sector id");
		for (ord, adjoining_sector_id) in valid_ordinals_for_this_sector.iter() {
			match ord {
				Ordinal::North => {
					let portal_nodes = self.get_portals_for_side_mut(ord);
					let column_range = 0..FIELD_RESOLUTION;
					let fixed_row = 0;
					let adjoining_cost_fields =
						sector_cost_fields.get().get(&adjoining_sector_id).unwrap();
					// walk along the side of the field
					let mut neighbouring_pathable = Vec::new();
					for i in column_range {
						let field_cost = cost_fields.get_grid_value(i, fixed_row);
						let adjacent_field_cost =
							adjoining_cost_fields.get_grid_value(i, FIELD_RESOLUTION - 1);
						if field_cost != 255 && adjacent_field_cost != 255 {
							// a pathable point along the edge so we record it to be
							// published later as a PortalNode
							neighbouring_pathable.push((i, fixed_row));
						} else {
							// if a length along the edge was previously calculated then publish
							// it as PortalNode
							if !neighbouring_pathable.is_empty() {
								// find the most centre like cell for this portal window
								let mut column_index_sum = 0;
								for (m, _) in neighbouring_pathable.iter() {
									column_index_sum += m;
								}
								let portal_midpoint_column =
									column_index_sum / neighbouring_pathable.len();
								portal_nodes.push(PortalNode::new(
									portal_midpoint_column as u32,
									fixed_row as u32,
								));
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
						portal_nodes.push(PortalNode::new(
							portal_midpoint_column as u32,
							fixed_row as u32,
						));
						// clear the recording list so any other portals along the side can be built
						neighbouring_pathable.clear();
					}
				}
				Ordinal::East => {
					let portal_nodes = self.get_portals_for_side_mut(ord);
					let fixed_column = FIELD_RESOLUTION - 1;
					let row_range = 0..FIELD_RESOLUTION;
					let adjoining_cost_fields =
						sector_cost_fields.get().get(&adjoining_sector_id).unwrap();
					// walk along the side of the field
					let mut neighbouring_pathable = Vec::new();
					for j in row_range {
						let field_cost = cost_fields.get_grid_value(fixed_column, j);
						let adjacent_field_cost = adjoining_cost_fields.get_grid_value(0, j);
						if field_cost != 255 && adjacent_field_cost != 255 {
							// a pathable point along the edge so we record it to be
							// published later as a PortalNode
							neighbouring_pathable.push((fixed_column, j));
						} else {
							// if a length along the edge was previously calculated then publish
							// it as PortalNode
							if !neighbouring_pathable.is_empty() {
								// find the most centre like cell for this portal window
								let mut row_index_sum = 0;
								for (_, n) in neighbouring_pathable.iter() {
									row_index_sum += n;
								}
								let portal_midpoint_row =
									row_index_sum / neighbouring_pathable.len();
								portal_nodes.push(PortalNode::new(
									fixed_column as u32,
									portal_midpoint_row as u32,
								));
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
						portal_nodes.push(PortalNode::new(
							fixed_column as u32,
							portal_midpoint_row as u32,
						));
						// clear the recording list so any other portals along the side can be built
						neighbouring_pathable.clear();
					}
				}
				Ordinal::South => {
					let portal_nodes = self.get_portals_for_side_mut(ord);
					let column_range = 0..FIELD_RESOLUTION;
					let fixed_row = FIELD_RESOLUTION - 1;
					let adjoining_cost_fields =
						sector_cost_fields.get().get(&adjoining_sector_id).unwrap();
					// walk along the side of the field
					let mut neighbouring_pathable = Vec::new();
					for i in column_range {
						let field_cost = cost_fields.get_grid_value(i, fixed_row);
						let adjacent_field_cost = adjoining_cost_fields.get_grid_value(i, 0);
						if field_cost != 255 && adjacent_field_cost != 255 {
							// a pathable point along the edge so we record it to be
							// published later as a PortalNode
							neighbouring_pathable.push((i, fixed_row));
						} else {
							// if a length along the edge was previously calculated then publish
							// it as PortalNode
							if !neighbouring_pathable.is_empty() {
								// find the most centre like cell for this portal window
								let mut column_index_sum = 0;
								for (m, _) in neighbouring_pathable.iter() {
									column_index_sum += m;
								}
								let portal_midpoint_column =
									column_index_sum / neighbouring_pathable.len();
								portal_nodes.push(PortalNode::new(
									portal_midpoint_column as u32,
									fixed_row as u32,
								));
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
						portal_nodes.push(PortalNode::new(
							portal_midpoint_column as u32,
							fixed_row as u32,
						));
						// clear the recording list so any other portals along the side can be built
						neighbouring_pathable.clear();
					}
				}
				Ordinal::West => {
					let portal_nodes = self.get_portals_for_side_mut(ord);
					let fixed_column = 0;
					let row_range = 0..FIELD_RESOLUTION;
					let adjoining_cost_fields =
						sector_cost_fields.get().get(&adjoining_sector_id).unwrap();
					// walk along the side of the field
					let mut neighbouring_pathable = Vec::new();
					for j in row_range {
						let field_cost = cost_fields.get_grid_value(fixed_column, j);
						let adjacent_field_cost =
							adjoining_cost_fields.get_grid_value(FIELD_RESOLUTION - 1, j);
						if field_cost != 255 && adjacent_field_cost != 255 {
							// a pathable point along the edge so we record it to be
							// published later as a PortalNode
							neighbouring_pathable.push((fixed_column, j));
						} else {
							// if a length along the edge was previously calculated then publish
							// it as PortalNode
							if !neighbouring_pathable.is_empty() {
								// find the most centre like cell for this portal window
								let mut row_index_sum = 0;
								for (_, n) in neighbouring_pathable.iter() {
									row_index_sum += n;
								}
								let portal_midpoint_row =
									row_index_sum / neighbouring_pathable.len();
								portal_nodes.push(PortalNode::new(
									fixed_column as u32,
									portal_midpoint_row as u32,
								));
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
						portal_nodes.push(PortalNode::new(
							fixed_column as u32,
							portal_midpoint_row as u32,
						));
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
}
/// A sector has up to four sides which can have portals, sectors around the boundary of the map
/// have less than 4. Based on the ID of the sector and the dimensions of the map retrieve the
/// ordinals of the sector which can support portals
fn get_sector_portal_ordinals(
	sector_id: (u32, u32),
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> Vec<Ordinal> {
	let sector_x_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
	let sector_z_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;

	if sector_id.0 == 0 && sector_id.1 == 0 {
		//top left sector only has 2 valid sides for portals
		// ___________
		// | x       |
		// |x        |
		// |         |
		// |         |
		// |_________|
		vec![Ordinal::East, Ordinal::South]
	} else if sector_id.0 == sector_x_limit && sector_id.1 == 0 {
		// top right sector has only two valid sides for portals
		// ___________
		// |       x |
		// |        x|
		// |         |
		// |         |
		// |_________|
		vec![Ordinal::South, Ordinal::West]
	} else if sector_id.0 == sector_x_limit && sector_id.1 == sector_z_limit {
		// bottom right sector only has two valid sides for portals
		// ___________
		// |         |
		// |         |
		// |         |
		// |        x|
		// |_______x_|
		vec![Ordinal::North, Ordinal::West]
	} else if sector_id.0 == 0 && sector_id.1 == sector_z_limit {
		// bottom left sector only has two valid sides for portals
		// ___________
		// |         |
		// |         |
		// |         |
		// |x        |
		// |_x_______|
		vec![Ordinal::North, Ordinal::East]
	} else if sector_id.0 > 0 && sector_id.0 < sector_x_limit && sector_id.1 == 0 {
		// northern row minus the corners sectors have three valid sides for portals
		// ___________
		// | xxxxxxx |
		// |         |
		// |         |
		// |         |
		// |_________|
		vec![Ordinal::East, Ordinal::South, Ordinal::West]
	} else if sector_id.0 == sector_x_limit && sector_id.1 > 0 && sector_id.1 < sector_z_limit {
		// eastern column minus the corners have three sectors of valid sides for portals
		// ___________
		// |         |
		// |        x|
		// |        x|
		// |        x|
		// |_________|
		vec![Ordinal::North, Ordinal::South, Ordinal::West]
	} else if sector_id.0 > 0 && sector_id.0 < sector_x_limit && sector_id.1 == sector_z_limit {
		// southern row minus corners have three sectors of valid sides for portals
		// ___________
		// |         |
		// |         |
		// |         |
		// |         |
		// |_xxxxxxx_|
		vec![Ordinal::North, Ordinal::East, Ordinal::West]
	} else if sector_id.0 == 0 && sector_id.1 > 0 && sector_id.1 < sector_z_limit {
		// western column minus corners have three sectors of valid sides for portals
		// ___________
		// |         |
		// |x        |
		// |x        |
		// |x        |
		// |_________|
		vec![Ordinal::North, Ordinal::East, Ordinal::South]
	} else if sector_id.0 > 0
		&& sector_id.0 < sector_x_limit
		&& sector_id.1 > 0
		&& sector_id.1 < sector_z_limit
	{
		// all other sectors not along an edge of the map have four valid sectors for portals
		// ___________
		// |         |
		// |    x    |
		// |   x x   |
		// |    x    |
		// |_________|
		vec![Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West]
	} else {
		// // special case that occurs when the map is so small that there's only
		error!(
			"Sector ID {:?} does not fit within map dimensions, there are only `{}x{}` sectors",
			sector_id,
			map_x_dimension / SECTOR_RESOLUTION as u32,
			map_z_dimension / SECTOR_RESOLUTION as u32
		);
		vec![]
	}
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use crate::flowfields::sectors::SectorPortals;

use super::*;
	#[test]
	fn get_northern_oridnals() {
		let sector_id = (3, 0);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_sector_portal_ordinals(sector_id, map_x_dimension, map_z_dimension);
		let actual =  vec![Ordinal::East, Ordinal::South, Ordinal::West];
		assert_eq!(actual,result);
	}
	#[test]
	fn get_eastern_oridnals() {
		let sector_id = (19, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_sector_portal_ordinals(sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::South, Ordinal::West];
		assert_eq!(actual,result);
	}
	#[test]
	fn get_southern_oridnals() {
		let sector_id = (4, 19);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_sector_portal_ordinals(sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::West];
		assert_eq!(actual,result);
	}
	#[test]
	fn get_western_oridnals() {
		let sector_id = (0, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_sector_portal_ordinals(sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::South];
		assert_eq!(actual,result);
	}
	#[test]
	fn get_centre_oridnals() {
		let sector_id = (4, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_sector_portal_ordinals(sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
		assert_eq!(actual,result);
	}
	#[test]
	fn portals_top_left_sector() {
		let mut sector_cost_fields = SectorCostFields::new(30, 30);
		let cost_fields = sector_cost_fields.get_mut().get_mut(&(0, 0)).unwrap();
		// switch some fields to impassable
		cost_fields.set_grid_value(255, 9, 5);
		cost_fields.set_grid_value(255, 0, 9);
		let mut portals = Portals::default();
		portals.recalculate_portals(&sector_cost_fields, &(0, 0), 30, 30);
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
		let mut sector_cost_fields = SectorCostFields::new(30, 30);
		let cost_fields = sector_cost_fields.get_mut().get_mut(&(1, 0)).unwrap();
		// switch some fields to impassable
		cost_fields.set_grid_value(255, 9, 5);
		cost_fields.set_grid_value(255, 0, 9);
		let mut portals = Portals::default();
		portals.recalculate_portals(&sector_cost_fields, &(1, 0), 30, 30);
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
		let mut sector_cost_fields = SectorCostFields::new(30, 30);
		let cost_fields = sector_cost_fields.get_mut().get_mut(&(1, 1)).unwrap();
		// switch some fields to impassable
		cost_fields.set_grid_value(255, 9, 5);
		cost_fields.set_grid_value(255, 0, 9);
		let mut portals = Portals::default();
		portals.recalculate_portals(&sector_cost_fields, &(1, 1), 30, 30);
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
		let mut sector_cost_fields = SectorCostFields::new(30, 30);
		let cost_fields = sector_cost_fields.get_mut().get_mut(&(1, 2)).unwrap();
		// switch some fields to impassable
		cost_fields.set_grid_value(255, 4, 0);
		cost_fields.set_grid_value(255, 6, 0);
		cost_fields.set_grid_value(255, 9, 5);
		cost_fields.set_grid_value(255, 0, 9);
		let mut portals = Portals::default();
		portals.recalculate_portals(&sector_cost_fields, &(1, 2), 30, 30);
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
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let mut sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
		let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, map_x_dimension, map_z_dimension)
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
		let pre_first = sector_portals.get_mut().get_mut(&(0, 0)).unwrap().clone();
		let pre_second = sector_portals.get_mut().get_mut(&(0, 1)).unwrap().clone();
		println!("Pre update portals {:?}", pre_first);
		println!("Pre update portals {:?}", pre_second);
		// update the top-left CostFields and calculate new portals
		let mutated_sector_id = (0, 0);
		let field = sector_cost_fields.get_mut().get_mut(&mutated_sector_id).unwrap();
		field.set_grid_value(255, 4, 9);
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, map_x_dimension, map_z_dimension);

		let post_first = sector_portals.get_mut().get_mut(&mutated_sector_id).unwrap().clone();
		let post_second = sector_portals.get_mut().get_mut(&(0, 1)).unwrap().clone();
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
			vec![PortalNode((9, 4))],
			vec![PortalNode((1, 9)), PortalNode((7, 9))],
			vec![]
			];
		let actual_second = [
			vec![PortalNode((1, 0)), PortalNode((7, 0))],
			vec![PortalNode((9, 4))],
			vec![PortalNode((4, 9))],
			vec![]
			];
		assert_eq!(actual_first[2], post_first.get()[2]);
		assert_eq!(actual_second[0], post_second.get()[0]);
	}
}
