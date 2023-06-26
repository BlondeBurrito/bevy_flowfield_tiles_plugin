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

pub mod cost_fields;
pub mod integration_fields;
pub mod plugin;
pub mod portal;
pub mod sectors;

/// Concenience way of accessing the 4 sides of a sector in [portal::Portals] and the 8 directions
/// of movement in [flowfield::Flowfields]
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
/// The length, `x` and depth, `z`, of the map
#[derive(Component)]
pub struct MapDimensions(u32, u32);

impl MapDimensions {
	pub fn new(x_length: u32, z_depth: u32) -> Self {
		//TODO some kind of check to ensure map isn;t too small, must be 3x3 sectors at least
		let x_sector_count = x_length / SECTOR_RESOLUTION as u32 - 1;
		let z_sector_count = z_depth / SECTOR_RESOLUTION as u32 - 1;
		MapDimensions(x_length, z_depth)
	}
	pub fn get_column(&self) -> u32 {
		self.0
	}
	pub fn get_row(&self) -> u32 {
		self.1
	}
}

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
		let portals = SectorPortals::new(map_length, map_depth);
		let mut graph = PortalGraph::default();
		graph
			.build_graph_nodes(&portals)
			.build_edges_within_each_sector(&portals)
			.build_edges_between_sectors(&portals, map_length, map_depth);
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
mod tests {}
