//! A Portal indicates a pathable area from one Sector to another.
//!

pub mod portal_graph;
pub mod portals;

use std::collections::BTreeMap;

use bevy::prelude::*;
use petgraph::{Directed, graph::NodeIndex, stable_graph::StableGraph};

use crate::v2::flowfields::{
	fields::{Field, FieldCell, cost_field::CostField},
	sectors::{SectorID, sector_cost::SectorCostFields},
	utilities::Ordinal,
};

/// Describes the start and end indices of a portal
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Debug, Clone, Reflect, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct PortalWindow {
	start: FieldCell,
	end: FieldCell,
}

impl PortalWindow {
	/// Get the middle [FieldCell] of the window
	fn get_midpoint(&self) -> FieldCell {
		if self.start == self.end {
			self.start
		} else if self.start.get_column() != self.end.get_column() {
			let mid_col = (self.start.get_column() + self.end.get_column()) / 2;
			FieldCell::new(mid_col, self.start.get_row())
		} else {
			let mid_row = (self.start.get_row() + self.end.get_row()) / 2;
			FieldCell::new(self.start.get_column(), mid_row)
		}
	}
}

/// Hold the [PortalWindow] for each boundary
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Debug, Clone, Reflect)]
struct Windows {
	north: Vec<PortalWindow>,
	east: Vec<PortalWindow>,
	south: Vec<PortalWindow>,
	west: Vec<PortalWindow>,
}
impl Windows {
	/// Based on [Ordinal] get the list of windows for that boundary
	fn get_windows_for_ordinal(&self, ordinal: &Ordinal) -> &Vec<PortalWindow> {
		match ordinal {
			Ordinal::North => &self.north,
			Ordinal::East => &self.east,
			Ordinal::South => &self.south,
			Ordinal::West => &self.west,
			_ => panic!("Ordinal {} cannot be used for looking up windows", ordinal),
		}
	}
	/// Based on [Ordinal] add a window
	fn add_window(&mut self, window: PortalWindow, ordinal: &Ordinal) {
		match ordinal {
			Ordinal::North => self.north.push(window),
			Ordinal::East => self.east.push(window),
			Ordinal::South => self.south.push(window),
			Ordinal::West => self.west.push(window),
			_ => panic!(
				"Ordinal {} cannot be used for recording portal windows",
				ordinal
			),
		}
	}
	// /// Get all [PortalWindow]
	// fn get_all(&self) -> Vec<PortalWindow> {
	// 	let mut portal_windows: Vec<PortalWindow> = vec![];
	// 	portal_windows.extend(&self.north);
	// 	portal_windows.extend(&self.east);
	// 	portal_windows.extend(&self.south);
	// 	portal_windows.extend(&self.west);

	// 	portal_windows
	// }
	/// Remove all [PortalWindow] and return them
	fn remove_all(&mut self) -> Vec<PortalWindow> {
		let mut portal_windows = vec![];
		for _ in 0..self.north.len() {
			portal_windows.push(self.north.pop().unwrap());
		}
		for _ in 0..self.east.len() {
			portal_windows.push(self.east.pop().unwrap());
		}
		for _ in 0..self.south.len() {
			portal_windows.push(self.south.pop().unwrap());
		}
		for _ in 0..self.west.len() {
			portal_windows.push(self.west.pop().unwrap());
		}

		portal_windows
	}
}

/// Represents a node in the portal graph
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Debug, Clone, Reflect, Ord, PartialOrd, Eq, Copy, PartialEq)]
struct PortalNode {
	sector: SectorID,
	window: PortalWindow,
}

/// Describes pathing between sectors
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Debug, Clone, Reflect)]
pub struct Portals {
	portals: BTreeMap<SectorID, Windows>,
	#[reflect(ignore)] //TODO
	nodes: BTreeMap<PortalNode, NodeIndex<u32>>,
	#[reflect(ignore)] //TODO
	graph: StableGraph<i32, i32, Directed, u32>,
}

impl Portals {
	pub fn new(sector_cost_fields: &SectorCostFields) -> Self {
		let scaled_costs = sector_cost_fields.get_scaled_costs();
		let graphs = sector_cost_fields.get_graphs();
		let mut portals = Portals::default();
		// calculate the portals
		portals.generate_all_portals(scaled_costs);
		// build the nodes
		portals.generate_all_nodes();
		// build the edges inside each sector
		portals.generate_all_internal_edges(graphs);
		// build the edges between each sector
		portals.generate_all_external_edges();

		portals
	}
	/// Iterate over all [CostField] and identify [PortalWindow] in every sector
	fn generate_all_portals(&mut self, scaled_costs: &BTreeMap<SectorID, CostField>) {
		// step through each sector and compare boundaries with adjacent sectors
		// determining portals
		for (origin_sector, origin_field) in scaled_costs.iter() {
			// set up sector in portals map
			if !self.portals.contains_key(origin_sector) {
				self.portals.insert(*origin_sector, Windows::default());
			}
			generate_sector_portals(&mut self.portals, scaled_costs, origin_sector, origin_field);
		}
	}
	/// Iterate over all [PortalWindow] and add graph nodes
	fn generate_all_nodes(&mut self) {
		let portal_graph = &mut self.graph;
		let nodes = &mut self.nodes;
		for (sector_id, windows) in self.portals.iter() {
			generate_sector_nodes(sector_id, windows, portal_graph, nodes);
		}
	}
	/// Iterate through all sectors and establish graph edges between internal portal windows
	fn generate_all_internal_edges(
		&mut self,
		cost_graphs: &BTreeMap<SectorID, StableGraph<u8, u8, Directed, u16>>,
	) {
		let portals = &self.portals;
		let nodes = &self.nodes;
		let portal_graph = &mut self.graph;
		for (sector, windows) in portals.iter() {
			generate_sector_internal_edges(sector, windows, nodes, cost_graphs, portal_graph);
		}
	}
	/// Iterate through all sectors and establish edges between neighbouring portal windows
	fn generate_all_external_edges(&mut self) {
		let portals = &self.portals;
		let nodes = &self.nodes;
		let portal_graph = &mut self.graph;
		for (sector, windows) in portals.iter() {
			generate_sector_external_edges(sector, windows, nodes, portal_graph, portals);
		}
	}
	/// When [CostField]s are changed the modified sector and its neighbours
	/// should have their portals recalculated
	pub fn update_portals(&mut self, sectors: &[SectorID], sector_costs: &SectorCostFields) {
		// remove nodes and recalculate portals
		let portals = &mut self.portals;
		let nodes = &mut self.nodes;
		let portal_graph = &mut self.graph;
		let scaled_costs = sector_costs.get_scaled_costs();
		let cost_graphs = sector_costs.get_graphs();
		for sector in sectors {
			// remove windows and nodes
			remove_sector_nodes_and_windows(sector, portals, nodes, portal_graph);
			// }
			// for sector in sectors {
			// recalculate portals
			let origin_field = scaled_costs.get(sector).unwrap();
			generate_sector_portals(portals, scaled_costs, sector, origin_field);
			// create orphaned nodes (without edges)
			let windows = portals.get_mut(sector).unwrap();
			generate_sector_nodes(sector, windows, portal_graph, nodes);
			// rebuild internal edges
			generate_sector_internal_edges(sector, windows, nodes, cost_graphs, portal_graph);
		}
		// once effected sectors have portals regenerated and internal edges created,
		// reestablish external edges
		for sector in sectors {
			let windows = portals.get(sector).unwrap();
			generate_sector_external_edges(sector, windows, nodes, portal_graph, portals);
		}
	}
}

/// Walk through each boundary of sector and compute the [`PortalWindow`s]
fn generate_sector_portals(
	portals: &mut BTreeMap<SectorID, Windows>,
	scaled_costs: &BTreeMap<SectorID, CostField>,
	origin_sector: &SectorID,
	origin_field: &CostField,
) {
	let ordinals = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
	for ordinal in ordinals.iter() {
		if let Some(windows) =
			walk_sector_boundary(origin_sector, origin_field, scaled_costs, ordinal)
		{
			for window in windows.iter() {
				let value = portals.get_mut(origin_sector).unwrap();
				value.add_window(*window, ordinal);
			}
		}
	}
}

fn walk_sector_boundary(
	origin_sector: &SectorID,
	origin_field: &CostField,
	scaled_costs: &BTreeMap<SectorID, CostField>,
	ordinal: &Ordinal,
) -> Option<Vec<PortalWindow>> {
	let adjacent_sector = origin_sector.get_in_ordinal_direction(ordinal, 1);
	// proceed if sector exists
	if let Some(adjacent_field) = scaled_costs.get(&adjacent_sector) {
		// walk along the boundary of the origin sector
		// we need the FieldCells along origin boundary and the FieldCells along the inverse boundary of the adjacent sector
		let (origin_cells, adjacent_cells) = boundary_field_cells(ordinal);
		// identify any windows
		let mut windows = vec![];
		let mut current_window = vec![];
		for (i, origin_cell) in origin_cells.iter().enumerate() {
			let origin_cost = origin_field.get_field_cell_value(*origin_cell);
			let adjacent_cost = adjacent_field.get_field_cell_value(adjacent_cells[i]);
			// if costs are < 255 we record then as part of a portal window
			// if we encounter either cost as 255 then we have found the limit of the window
			if origin_cost != 255 && adjacent_cost != 255 {
				current_window.push(origin_cell);
			} else {
				if !current_window.is_empty() {
					let start = current_window.first().unwrap();
					let end = current_window.last().unwrap();
					let portal_window = PortalWindow {
						start: **start,
						end: **end,
					};
					windows.push(portal_window);
					current_window.clear();
					continue;
				}
			}
			// case where boundary is walked to the far end, we
			// need to publish the current window as there's nothing
			// left to walk/compare against
			if i == origin_cells.len() - 1 && !current_window.is_empty() {
				let start = current_window.first().unwrap();
				let end = current_window.last().unwrap();
				let portal_window = PortalWindow {
					start: **start,
					end: **end,
				};
				windows.push(portal_window);
				current_window.clear();
			}
		}
		Some(windows)
	} else {
		None
	}
}

/// Create [PortalNode] and graph nodes for a given [SectorID]
fn generate_sector_nodes(
	sector_id: &SectorID,
	windows: &Windows,
	portal_graph: &mut StableGraph<i32, i32, Directed, u32>,
	nodes: &mut BTreeMap<PortalNode, NodeIndex<u32>>,
) {
	let ordinals = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
	for ordinal in ordinals.iter() {
		for window in windows.get_windows_for_ordinal(ordinal).iter() {
			let portal_node = PortalNode {
				sector: *sector_id,
				window: *window,
			};
			let node = portal_graph.add_node(1);
			nodes.insert(portal_node, node);
		}
	}
}

/// Visit each boundary of a sector and create graph edges between each
/// [PortalNode] that has a path to another one
fn generate_sector_internal_edges(
	sector: &SectorID,
	windows: &Windows,
	nodes: &BTreeMap<PortalNode, NodeIndex<u32>>,
	cost_graphs: &BTreeMap<SectorID, StableGraph<u8, u8, Directed, u16>>,
	portal_graph: &mut StableGraph<i32, i32, Directed, u32>,
) {
	// create a list of all the PortalWindow
	let mut window_list: Vec<PortalWindow> = vec![];
	window_list.extend(&windows.north);
	window_list.extend(&windows.east);
	window_list.extend(&windows.south);
	window_list.extend(&windows.west);
	// we need to cost graph of this sector to verify if two portal windows can
	// actually path to one another
	let cost_graph = cost_graphs.get(sector).unwrap();
	// iter over all the windows doubly and establish edges for windows that can see each other
	for this_window in window_list.iter() {
		for other_window in window_list.iter() {
			// don't create an edge for a window going to itself
			if this_window == other_window {
				continue;
			}
			// find the midpoint of the two windows
			let this_midpoint = this_window.get_midpoint();
			let other_midpoint = other_window.get_midpoint();
			// check if they pathable via the cost graph
			let start = this_midpoint.as_1d_index() as u16;
			let is_goal = other_midpoint.as_1d_index() as u16;
			let estimate_cost = |_| 0;
			if let Some((path_cost, _)) = petgraph::algo::astar::astar(
				cost_graph,
				start.into(),
				|finish| finish == is_goal.into(),
				|edge| *edge.weight(),
				estimate_cost,
			) {
				// windows can see each other so create an edge in the portal graph,
				// use the cost of this path as the edge weight
				let this_node = PortalNode {
					sector: *sector,
					window: *this_window,
				};
				let other_node = PortalNode {
					sector: *sector,
					window: *other_window,
				};
				let this_node_index = nodes.get(&this_node).unwrap();
				let other_node_index = nodes.get(&other_node).unwrap();
				portal_graph.add_edge(*this_node_index, *other_node_index, path_cost as i32);
			}
		}
	}
}

/// Using each [Ordinal] boundary of a sector lookup neighbouring sector
/// [PortalWindow] and create edges between them
fn generate_sector_external_edges(
	sector: &SectorID,
	windows: &Windows,
	nodes: &BTreeMap<PortalNode, NodeIndex<u32>>,
	portal_graph: &mut StableGraph<i32, i32, Directed, u32>,
	portals: &BTreeMap<SectorID, Windows>,
) {
	let ordinals = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
	for ordinal in ordinals.iter() {
		// get the portal windows of this sector for a boundary
		let this_portal_windows = windows.get_windows_for_ordinal(ordinal);
		// find the adjacent sector so that its windows can be found
		let adjacent_sector = sector.get_in_ordinal_direction(ordinal, 1);
		// get the mirrored ordinal in the adjacent sector
		let adjacent_ordinal = match ordinal {
			Ordinal::North => Ordinal::South,
			Ordinal::East => Ordinal::West,
			Ordinal::South => Ordinal::North,
			Ordinal::West => Ordinal::East,
			_ => panic!("This should never panic"),
		};
		if let Some(adjacent_windows) = portals.get(&adjacent_sector) {
			let adjacent_portal_windows =
				adjacent_windows.get_windows_for_ordinal(&adjacent_ordinal);
			for (i, this_portal_window) in this_portal_windows.iter().enumerate() {
				let adjacent_portal_window = adjacent_portal_windows.get(i).unwrap();
				// create [PortalNode] for each window for looking up the graph indices
				let this_portal_node = PortalNode {
					sector: *sector,
					window: *this_portal_window,
				};
				let adjacent_portal_node = PortalNode {
					sector: adjacent_sector,
					window: *adjacent_portal_window,
				};
				// extract the graph indices
				if let Some(this_node_index) = nodes.get(&this_portal_node) {
					if let Some(adjacent_node_index) = nodes.get(&adjacent_portal_node) {
						// create the directed edge from this to adjacent
						portal_graph.add_edge(*this_node_index, *adjacent_node_index, 1);
					}
				}
			}
		}
	}
}

/// For a given sector remove the [PortalNode]s and graph nodes
fn remove_sector_nodes_and_windows(
	sector: &SectorID,
	portals: &mut BTreeMap<SectorID, Windows>,
	nodes: &mut BTreeMap<PortalNode, NodeIndex<u32>>,
	portal_graph: &mut StableGraph<i32, i32, Directed, u32>,
) {
	let windows = portals.get_mut(sector).unwrap();
	let portal_windows = windows.remove_all();
	for pw in portal_windows.iter() {
		let portal_node = PortalNode {
			sector: *sector,
			window: *pw,
		};
		if let Some(graph_node) = nodes.remove(&portal_node) {
			portal_graph.remove_node(graph_node);
		}
	}
}

/// Get [FieldCell] sets along an [Ordinal] boundary. The first set are the cells along the [Ordinal]. The second set are the corresponding [FieldCell] adjacent in a sector in the [Ordinal] direction (i.e if `ordinal` is `North` then the second set are the [FieldCell] of the adjacent sectors `South` boundary)
fn boundary_field_cells(ordinal: &Ordinal) -> ([FieldCell; 10], [FieldCell; 10]) {
	match ordinal {
		Ordinal::North => (
			[
				FieldCell::new(0, 0),
				FieldCell::new(1, 0),
				FieldCell::new(2, 0),
				FieldCell::new(3, 0),
				FieldCell::new(4, 0),
				FieldCell::new(5, 0),
				FieldCell::new(6, 0),
				FieldCell::new(7, 0),
				FieldCell::new(8, 0),
				FieldCell::new(9, 0),
			],
			[
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
			],
		),
		Ordinal::East => (
			[
				FieldCell::new(9, 0),
				FieldCell::new(9, 1),
				FieldCell::new(9, 2),
				FieldCell::new(9, 3),
				FieldCell::new(9, 4),
				FieldCell::new(9, 5),
				FieldCell::new(9, 6),
				FieldCell::new(9, 7),
				FieldCell::new(9, 8),
				FieldCell::new(9, 9),
			],
			[
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
			],
		),
		Ordinal::South => (
			[
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
			],
			[
				FieldCell::new(0, 0),
				FieldCell::new(1, 0),
				FieldCell::new(2, 0),
				FieldCell::new(3, 0),
				FieldCell::new(4, 0),
				FieldCell::new(5, 0),
				FieldCell::new(6, 0),
				FieldCell::new(7, 0),
				FieldCell::new(8, 0),
				FieldCell::new(9, 0),
			],
		),
		Ordinal::West => (
			[
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
			],
			[
				FieldCell::new(9, 0),
				FieldCell::new(9, 1),
				FieldCell::new(9, 2),
				FieldCell::new(9, 3),
				FieldCell::new(9, 4),
				FieldCell::new(9, 5),
				FieldCell::new(9, 6),
				FieldCell::new(9, 7),
				FieldCell::new(9, 8),
				FieldCell::new(9, 9),
			],
		),
		_ => panic!("Ordinal {} cannot be used for boundary walking", ordinal),
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use crate::v2::flowfields::dimensions::Dimensions;

	use super::*;

	#[test]
	fn window_midpoint1() {
		let window = PortalWindow {
			start: FieldCell::new(0, 3),
			end: FieldCell::new(0, 9),
		};

		let actual = FieldCell::new(0, 6);
		let result = window.get_midpoint();
		assert_eq!(actual, result);
	}

	#[test]
	fn window_midpoint2() {
		let window = PortalWindow {
			start: FieldCell::new(9, 5),
			end: FieldCell::new(9, 7),
		};

		let actual = FieldCell::new(9, 6);
		let result = window.get_midpoint();
		assert_eq!(actual, result);
	}

	#[test]
	fn window_midpoint3() {
		let window = PortalWindow {
			start: FieldCell::new(0, 3),
			end: FieldCell::new(0, 3),
		};

		let actual = FieldCell::new(0, 3);
		let result = window.get_midpoint();
		assert_eq!(actual, result);
	}

	#[test]
	fn windows_ordinal() {
		let north = vec![PortalWindow {
			start: FieldCell::new(0, 0),
			end: FieldCell::new(5, 0),
		}];
		let east = vec![PortalWindow {
			start: FieldCell::new(9, 0),
			end: FieldCell::new(9, 4),
		}];
		let south = vec![PortalWindow {
			start: FieldCell::new(2, 9),
			end: FieldCell::new(7, 9),
		}];
		let west = vec![PortalWindow {
			start: FieldCell::new(0, 4),
			end: FieldCell::new(0, 8),
		}];
		let windows = Windows {
			north: north.clone(),
			east: east.clone(),
			south: south.clone(),
			west: west.clone(),
		};

		assert_eq!(north, *windows.get_windows_for_ordinal(&Ordinal::North));
		assert_eq!(east, *windows.get_windows_for_ordinal(&Ordinal::East));
		assert_eq!(south, *windows.get_windows_for_ordinal(&Ordinal::South));
		assert_eq!(west, *windows.get_windows_for_ordinal(&Ordinal::West));
	}

	#[test]
	fn window_add() {
		let mut windows = Windows {
			north: vec![PortalWindow {
				start: FieldCell::new(0, 0),
				end: FieldCell::new(5, 0),
			}],
			east: vec![],
			south: vec![],
			west: vec![],
		};
		let new = PortalWindow {
			start: FieldCell::new(7, 0),
			end: FieldCell::new(9, 0),
		};
		windows.add_window(new, &Ordinal::North);

		let actual = vec![
			PortalWindow {
				start: FieldCell::new(0, 0),
				end: FieldCell::new(5, 0),
			},
			PortalWindow {
				start: FieldCell::new(7, 0),
				end: FieldCell::new(9, 0),
			},
		];
		let result = windows.north;
		assert_eq!(actual, result);
	}

	#[test]
	fn windows_removal() {
		let mut windows = Windows {
			north: vec![PortalWindow {
				start: FieldCell::new(0, 0),
				end: FieldCell::new(5, 0),
			}],
			east: vec![PortalWindow {
				start: FieldCell::new(9, 0),
				end: FieldCell::new(9, 4),
			}],
			south: vec![PortalWindow {
				start: FieldCell::new(2, 9),
				end: FieldCell::new(7, 9),
			}],
			west: vec![PortalWindow {
				start: FieldCell::new(0, 4),
				end: FieldCell::new(0, 8),
			}],
		};
		let result = windows.remove_all();
		let actual = vec![
			PortalWindow {
				start: FieldCell::new(0, 0),
				end: FieldCell::new(5, 0),
			},
			PortalWindow {
				start: FieldCell::new(9, 0),
				end: FieldCell::new(9, 4),
			},
			PortalWindow {
				start: FieldCell::new(2, 9),
				end: FieldCell::new(7, 9),
			},
			PortalWindow {
				start: FieldCell::new(0, 4),
				end: FieldCell::new(0, 8),
			},
		];
		assert_eq!(actual, result);
	}

	#[test]
	fn walk() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let sector_costs = SectorCostFields::new(&dimensions);

		let origin_sector = SectorID::new(0, 0);
		let origin_field = sector_costs.get_scaled_costs().get(&origin_sector).unwrap();
		let scaled_costs = sector_costs.get_scaled_costs();

		let ordinal = &Ordinal::North;
		let north = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, ordinal);
		assert!(north.is_none());

		let ordinal = &Ordinal::East;
		let east = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, ordinal);
		assert!(1 == east.unwrap().len());

		let ordinal = &Ordinal::South;
		let south = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, ordinal);
		assert!(1 == south.unwrap().len());

		let ordinal = &Ordinal::West;
		let west = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, ordinal);
		assert!(west.is_none());
	}

	// walk a boundary where the CostField has an initial wall on the boundary, this
	// should prove that multiple portals are initialised
	#[test]
	fn walk_initial_wall() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let mut sector_costs = SectorCostFields::new(&dimensions);
		sector_costs.set_field_cost(
			&SectorID::new(0, 0),
			&FieldCell::new(9, 3),
			255,
			&dimensions,
		);

		let origin_sector = SectorID::new(0, 0);
		let origin_field = sector_costs.get_scaled_costs().get(&origin_sector).unwrap();
		let scaled_costs = sector_costs.get_scaled_costs();

		let ordinal = &Ordinal::East;
		let east = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, ordinal);
		assert!(2 == east.unwrap().len());

		// ensure the neighbour without a wall registers
		// the same number of portals
		let n_sector = SectorID::new(1, 0);
		let n_field = sector_costs.get_scaled_costs().get(&n_sector).unwrap();
		let scaled_costs = sector_costs.get_scaled_costs();

		let n_ordinal = &Ordinal::West;
		let west = walk_sector_boundary(&n_sector, n_field, scaled_costs, n_ordinal);
		assert!(2 == west.unwrap().len());
	}

	#[test]
	fn sector_portals() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let sector_costs = SectorCostFields::new(&dimensions);
		let mut portals = Portals::default();

		portals.generate_all_portals(sector_costs.get_scaled_costs());

		// check for PortalWindow existence on boundaries
		let sector1 = SectorID::new(0, 0);
		let s1_north = &portals.portals.get(&sector1).unwrap().north;
		assert!(0 == s1_north.len());
		let s1_east = &portals.portals.get(&sector1).unwrap().east;
		assert!(1 == s1_east.len());
		let s1_south = &portals.portals.get(&sector1).unwrap().south;
		assert!(1 == s1_south.len());
		let s1_west = &portals.portals.get(&sector1).unwrap().west;
		assert!(0 == s1_west.len());
	}

	#[test]
	fn default_node_edge_count() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let sector_costs = SectorCostFields::new(&dimensions);
		let portals = Portals::new(&sector_costs);

		let actual_node_count = 8;
		let result_node_count = portals.graph.node_count();
		assert_eq!(actual_node_count, result_node_count);

		let actual_edge_count = 16;
		let result_edge_count = portals.graph.edge_count();
		assert_eq!(actual_edge_count, result_edge_count);
	}

	// check nodes and edges when the SectorCost begins with a wall
	#[test]
	fn graph_counts_initial_wall() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let mut sector_costs = SectorCostFields::new(&dimensions);
		sector_costs.set_field_cost(
			&SectorID::new(1, 1),
			&FieldCell::new(0, 5),
			255,
			&dimensions,
		);

		let portals = Portals::new(&sector_costs);

		let actual_node_count = 10;
		let result_node_count = portals.graph.node_count();
		assert_eq!(actual_node_count, result_node_count);

		let actual_edge_count = 26;
		let result_edge_count = portals.graph.edge_count();
		assert_eq!(actual_edge_count, result_edge_count);
	}

	#[test]
	fn remove_nodes() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let sector_costs = SectorCostFields::new(&dimensions);
		let mut portals = Portals::new(&sector_costs);

		let nodes = &mut portals.nodes;
		let portal_graph = &mut portals.graph;
		for sector in &[SectorID::new(0, 0)] {
			remove_sector_nodes_and_windows(sector, &mut portals.portals, nodes, portal_graph);
		}

		assert!(6 == portals.graph.node_count());
		assert!(10 == portals.graph.edge_count());
	}

	// generate portals, then update SectorCosts, then update portals
	#[test]
	fn graph_updated() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let mut sector_costs = SectorCostFields::new(&dimensions);
		let mut portals = Portals::new(&sector_costs);

		sector_costs.set_field_cost(
			&SectorID::new(1, 1),
			&FieldCell::new(0, 5),
			255,
			&dimensions,
		);

		portals.update_portals(
			&[
				SectorID::new(1, 1),
				SectorID::new(1, 0),
				SectorID::new(0, 1),
			],
			&sector_costs,
		);

		println!("Windows {:?}", portals.portals);
		println!("Nodes: {:?}", portals.nodes);
		println!("Edges {:?}", portals.graph.edge_indices());

		let actual_node_count = 10;
		let result_node_count = portals.graph.node_count();
		assert_eq!(actual_node_count, result_node_count);

		let actual_edge_count = 26;
		let result_edge_count = portals.graph.edge_count();
		assert_eq!(actual_edge_count, result_edge_count);
	}
}
