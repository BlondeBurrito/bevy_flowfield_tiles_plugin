//! Flowfields are a means of handling pathfinding for a crowd of actors.
//!
//! [Fixing Pathfinding Once and For All](https://web.archive.org/web/20150905073624/http://www.ai-blog.net/archives/000152.html)
//!
//! [SupCom2- Elijah Emerson](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter23_Crowd_Pathfinding_and_Steering_Using_Flow_Field_Tiles.pdf)
//!
//! [jdxdev](https://www.jdxdev.com/blog/2020/05/03/flowfields/)
//!
//! [leifnode](https://leifnode.com/2013/12/flow-field-pathfinding/)
//!
//! A map is divided into a series of Sectors with Portals indicating a pathable point from
//! one Sector to a neighbour. A Sector is made up of fields which the algorithm uses to calculate a
//! path from a starting position to a goal position.
//!
//! Sectors are positioned from the top-left corner of the map, i.e (-x, -z) direction. the fields of
//! a sector are indexed from the top-left corner of the sector.
//!
//! Definitions:
//!
//! * Sector - a grid area of `MxN` dimensions containing three 2D arrays of `MxN` used for calcualting paths. These arrays are called 'cost fields', 'integration fields' and 'flow fields'
//!
//! ```text
//!  _____________________________
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! |__|__|__|__|__|__|__|__|__|__|
//! ```
//!
//! * Portal - a pathable window from one Sector to another
//! * Cost field - 8-bit field where a value of 255 represents impassable terrain and range 1 - 254
//! represents the cost of traversing that grid location, 1 being the default and easiest. You could define
//! a value of 56 for instance as being a slope or swamp and in such a case pathfinding will try to avoid it
//! * Integration field - uses the cost field as input and stores the calculated cost-to-goal (cost to path to the eventual location you want to end up at).
//! * Flow field - 8-bit field used by actors to flow from one area of space to another. The first 4 bits
//! of the field represent directions of movement and the second 4 bits are flags to indicate whether a
//! field cell is pathable or provides a straight line route to the target/goal (which menas you don't
//! need to spend time calculating any cells, the actor can just move straight towards it)
//!

use std::collections::BTreeMap;

use bevy::prelude::*;

use self::{
	portal::portal_graph::PortalGraph,
	sectors::{SectorCostFields, SectorPortals},
};
/// Determines the number of Sectors by dividing the map length and depth by this value
const SECTOR_RESOLUTION: usize = 10;
/// Defines the dimenions of all field arrays
const FIELD_RESOLUTION: usize = 10;

pub mod cost_field;
pub mod flow_field;
pub mod integration_field;
pub mod plugin;
pub mod portal;
pub mod sectors;

/// Convenience way of accessing the 4 sides of a sector in [portal::portals::Portals], the 4 sides of a grid cell in [integration_field::IntegrationField] and the 8 directions
/// of movement in [flow_field::FlowField]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, PartialEq)]
pub enum Ordinal {
	North,
	East,
	South,
	West,
	NorthEast,
	SouthEast,
	SouthWest,
	NorthWest,
}

impl Ordinal {
	/// Based on a grid cells `(column, row)` position find its neighbours based on FIELD_RESOLUTION limits (up to 4)
	pub fn get_cell_neighbours(cell_id: (usize, usize)) -> Vec<(usize, usize)> {
		let mut neighbours = Vec::new();
		if cell_id.1 > 0 {
			neighbours.push((cell_id.0, cell_id.1 - 1)); // northern cell coords
		}
		if cell_id.0 < FIELD_RESOLUTION - 1 {
			neighbours.push((cell_id.0 + 1, cell_id.1)); // eastern cell coords
		}
		if cell_id.1 < FIELD_RESOLUTION - 1 {
			neighbours.push((cell_id.0, cell_id.1 + 1)); // southern cell coords
		}
		if cell_id.0 > 0 {
			neighbours.push((cell_id.0 - 1, cell_id.1)); // western cell coords
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find its neighbours based on map size limits (up to 4)
	pub fn get_sector_neighbours(
		sector_id: &(u32, u32),
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> Vec<(u32, u32)> {
		let mut neighbours = Vec::new();
		let sector_x_column_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
		let sector_z_row_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;
		if sector_id.1 > 0 {
			neighbours.push((sector_id.0, sector_id.1 - 1)); // northern sector coords
		}
		if sector_id.0 < sector_x_column_limit {
			neighbours.push((sector_id.0 + 1, sector_id.1)); // eastern sector coords
		}
		if sector_id.1 < sector_z_row_limit {
			neighbours.push((sector_id.0, sector_id.1 + 1)); // southern sector coords
		}
		if sector_id.0 > 0 {
			neighbours.push((sector_id.0 - 1, sector_id.1)); // western sector coords
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find the [Ordinal] directions for its boundaries that can support [portal::portals::Portals]
	pub fn get_sector_portal_ordinals(
		sector_id: &(u32, u32),
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> Vec<Ordinal> {
		let mut neighbours = Vec::new();
		let sector_x_column_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
		let sector_z_row_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;
		if sector_id.1 > 0 {
			neighbours.push(Ordinal::North); // northern sector coords
		}
		if sector_id.0 < sector_x_column_limit {
			neighbours.push(Ordinal::East); // eastern sector coords
		}
		if sector_id.1 < sector_z_row_limit {
			neighbours.push(Ordinal::South); // southern sector coords
		}
		if sector_id.0 > 0 {
			neighbours.push(Ordinal::West); // western sector coords
		}
		neighbours
	}
	/// Based on a sectors `(column, row)` position find its neighbours based on map size limits (up to 4) and include the [Ordinal] direction in the result
	pub fn get_sector_neighbours_with_ordinal(
		sector_id: &(u32, u32),
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> Vec<(Ordinal, (u32, u32))> {
		let mut neighbours = Vec::new();
		let sector_x_column_limit = map_x_dimension / SECTOR_RESOLUTION as u32 - 1;
		let sector_z_row_limit = map_z_dimension / SECTOR_RESOLUTION as u32 - 1;
		if sector_id.1 > 0 {
			neighbours.push((Ordinal::North, (sector_id.0, sector_id.1 - 1))); // northern sector coords
		}
		if sector_id.0 < sector_x_column_limit {
			neighbours.push((Ordinal::East, (sector_id.0 + 1, sector_id.1))); // eastern sector coords
		}
		if sector_id.1 < sector_z_row_limit {
			neighbours.push((Ordinal::South, (sector_id.0, sector_id.1 + 1))); // southern sector coords
		}
		if sector_id.0 > 0 {
			neighbours.push((Ordinal::West, (sector_id.0 - 1, sector_id.1))); // western sector coords
		}
		neighbours
	}
	/// Returns the opposite [Ordinal] of the current
	pub fn inverse(&self) -> Ordinal {
		match self {
			Ordinal::North => Ordinal::South,
			Ordinal::East => Ordinal::West,
			Ordinal::South => Ordinal::North,
			Ordinal::West => Ordinal::East,
			Ordinal::NorthEast => Ordinal::SouthWest,
			Ordinal::SouthEast => Ordinal::NorthWest,
			Ordinal::SouthWest => Ordinal::NorthEast,
			Ordinal::NorthWest => Ordinal::SouthEast,
		}
	}
}

/// The length `x` and depth `z` of the map
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Default)]
pub struct MapDimensions(u32, u32);

impl MapDimensions {
	pub fn new(x_length: u32, z_depth: u32) -> Self {
		//TODO some kind of check to ensure map isn;t too small, must be 3x3? sectors at least
		let x_sector_count = (x_length / SECTOR_RESOLUTION as u32).checked_sub(1);
		let z_sector_count = (z_depth / SECTOR_RESOLUTION as u32).checked_sub(1);
		if x_sector_count.is_none() || z_sector_count.is_none() {
			panic!(
				"Map dimensions `({}, {})` cannot support sectors, try larger values",
				x_length, z_depth
			);
		}
		MapDimensions(x_length, z_depth)
	}
	pub fn get_column(&self) -> u32 {
		self.0
	}
	pub fn get_row(&self) -> u32 {
		self.1
	}
}
//TODO #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Bundle)]
pub struct FlowfieldTilesBundle {
	sector_cost_fields: SectorCostFields,
	sector_portals: SectorPortals,
	portal_graph: PortalGraph,
	map_dimensions: MapDimensions,
}

impl FlowfieldTilesBundle {
	pub fn new(map_length: u32, map_depth: u32) -> Self {
		let map_dimensions = MapDimensions::new(map_length, map_depth);
		let cost_fields = SectorCostFields::new(map_length, map_depth);
		let mut portals = SectorPortals::new(map_length, map_depth);
		// update default portals for cost fields
		for (sector_id, _v) in cost_fields.get() {
			portals.update_portals(
				*sector_id,
				&cost_fields,
				map_dimensions.get_column(),
				map_dimensions.get_row(),
			);
		}
		let graph = PortalGraph::new(
			&portals,
			&cost_fields,
			map_dimensions.get_column(),
			map_dimensions.get_row(),
		);
		FlowfieldTilesBundle {
			sector_cost_fields: cost_fields,
			sector_portals: portals,
			portal_graph: graph,
			map_dimensions,
		}
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn ordinal_grid_cell_neighbours() {
		let cell_id = (0, 0);
		let result = Ordinal::get_cell_neighbours(cell_id);
		let actual = vec![(1, 0), (0, 1)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_grid_cell_neighbours2() {
		let cell_id = (9, 9);
		let result = Ordinal::get_cell_neighbours(cell_id);
		let actual = vec![(9, 8), (8, 9)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_grid_cell_neighbours3() {
		let cell_id = (4, 4);
		let result = Ordinal::get_cell_neighbours(cell_id);
		let actual = vec![(4, 3), (5, 4), (4, 5), (3, 4)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_grid_cell_neighbours4() {
		let cell_id = (5, 0);
		let result = Ordinal::get_cell_neighbours(cell_id);
		let actual = vec![(6, 0), (5, 1), (4, 0)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours() {
		let sector_id = (0, 0);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let result = Ordinal::get_sector_neighbours(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(1, 0), (0, 1)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours2() {
		let sector_id = (29, 54);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let result = Ordinal::get_sector_neighbours(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(29, 53), (28, 54)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours3() {
		let sector_id = (14, 31);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let result = Ordinal::get_sector_neighbours(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(14, 30), (15, 31), (14, 32), (13, 31)];
		assert_eq!(actual, result);
	}
	#[test]
	fn ordinal_sector_neighbours4() {
		let sector_id = (0, 13);
		let map_x_dimension = 300;
		let map_z_dimension = 550;
		let result = Ordinal::get_sector_neighbours(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(0, 12), (1, 13), (0, 14)];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_northern_oridnals() {
		let sector_id = (3, 0);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::East, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_eastern_oridnals() {
		let sector_id = (19, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_southern_oridnals() {
		let sector_id = (4, 19);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::West];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_western_oridnals() {
		let sector_id = (0, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::South];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_centre_oridnals() {
		let sector_id = (4, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result =
			Ordinal::get_sector_portal_ordinals(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
		assert_eq!(actual, result);
	}
}
