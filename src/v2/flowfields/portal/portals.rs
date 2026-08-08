// //! A Portal indicates a pathable area from one Sector to another.
// //!

// use std::collections::BTreeMap;

// use bevy::prelude::*;
// use petgraph::{Directed, graph::NodeIndex, stable_graph::StableGraph};

// use crate::v2::flowfields::{
// 	fields::{Field, FieldCell, cost_field::CostField},
// 	sectors::{SectorID, sector_cost::SectorCostFields},
// 	utilities::Ordinal,
// };

// /// Describes the start and end indices of a portal
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Default, Debug, Clone, Reflect, Copy, Ord, PartialOrd, Eq, PartialEq)]
// struct PortalWindow {
// 	start: FieldCell,
// 	end: FieldCell,
// }

// impl PortalWindow {
// 	/// Get the middle [FieldCell] of the window
// 	fn get_midpoint(&self) -> FieldCell {
// 		if self.start == self.end {
// 			self.start
// 		} else if self.start.get_column() != self.end.get_column() {
// 			let mid_col = (self.start.get_column() + self.end.get_column()) / 2;
// 			FieldCell::new(mid_col, self.start.get_row())
// 		} else {
// 			let mid_row = (self.start.get_row() + self.end.get_row()) / 2;
// 			FieldCell::new(self.start.get_column(), mid_row)
// 		}
// 	}
// }

// /// Hold the [PortalWindow] for each boundary
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Default, Debug, Clone, Reflect)]
// struct Windows {
// 	north: Vec<PortalWindow>,
// 	east: Vec<PortalWindow>,
// 	south: Vec<PortalWindow>,
// 	west: Vec<PortalWindow>,
// }
// impl Windows {
// 	/// Based on [Ordinal] get the list of windows for that boundary
// 	fn get_windows_for_ordinal(&self, ordinal: &Ordinal) -> &Vec<PortalWindow> {
// 		match ordinal {
// 			Ordinal::North => &self.north,
// 			Ordinal::East => &self.east,
// 			Ordinal::South => &self.south,
// 			Ordinal::West => &self.west,
// 			_ => panic!("Ordinal {} cannot be used for looking up windows", ordinal),
// 		}
// 	}
// 	/// Based on [Ordinal] add a window
// 	fn add_window(&mut self, window: PortalWindow, ordinal: &Ordinal) {
// 		match ordinal {
// 			Ordinal::North => self.north.push(window),
// 			Ordinal::East => self.east.push(window),
// 			Ordinal::South => self.south.push(window),
// 			Ordinal::West => self.west.push(window),
// 			_ => panic!(
// 				"Ordinal {} cannot be used for recording portal windows",
// 				ordinal
// 			),
// 		}
// 	}
// }

// /// Represents a node in the portal graph
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Default, Debug, Clone, Reflect, Ord, PartialOrd, Eq, Copy, PartialEq)]
// struct PortalNode {
// 	sector: SectorID,
// 	window: PortalWindow,
// }

// /// Describes pathing between sectors
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Default, Debug, Clone, Reflect)]
// struct Portals {
// 	portals: BTreeMap<SectorID, Windows>,
// 	#[reflect(ignore)] //TODO
// 	nodes: BTreeMap<PortalNode, NodeIndex<u32>>,
// 	#[reflect(ignore)] //TODO
// 	graph: StableGraph<i32, i32, Directed, u32>,
// }

// impl Portals {
// 	pub fn new(sector_cost_fields: &SectorCostFields) -> Self {
// 		let scaled_costs = sector_cost_fields.get_scaled_costs();
// 		let graphs = sector_cost_fields.get_graphs();
// 		let mut portals = Portals::default();
// 		// calculate the portals
// 		portals.generate_all_portals(scaled_costs);
// 		// build the nodes
// 		portals.generate_all_nodes();
// 		// build the edges inside each sector
// 		portals.generate_all_internal_edges(graphs);
// 		// build the edges between each sector
// 		portals.generate_all_external_edges();

// 		portals
// 	}
// 	/// Iterate over all [CostField] and identify [PortalWindow] in every sector
// 	fn generate_all_portals(&mut self, scaled_costs: &BTreeMap<SectorID, CostField>) {
// 		// step through each sector and compare boundaries with adjacent sectors
// 		// determining portals
// 		for (origin_sector, origin_field) in scaled_costs.iter() {
// 			// set up sector in portals map
// 			if !self.portals.contains_key(origin_sector) {
// 				self.portals.insert(*origin_sector, Windows::default());
// 			}
// 			self.generate_sector_portals(scaled_costs, origin_sector, origin_field);
// 		}
// 	}
// 	/// Walk through each boundary of sector and compute the [`PortalWindow`s]
// 	fn generate_sector_portals(
// 		&mut self,
// 		scaled_costs: &BTreeMap<SectorID, CostField>,
// 		origin_sector: &SectorID,
// 		origin_field: &CostField,
// 	) {
// 		let ordinals = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
// 		for ordinal in ordinals.iter() {
// 			if let Some(windows) =
// 				walk_sector_boundary(origin_sector, origin_field, scaled_costs, ordinal)
// 			{
// 				for window in windows.iter() {
// 					let value = self.portals.get_mut(origin_sector).unwrap();
// 					value.add_window(*window, ordinal);
// 				}
// 			}
// 		}
// 	}
// 	/// Iterate over all [CostField] and identify [Node] across sector boundaries
// 	fn generate_all_nodes(&mut self) {
// 		for (sector_id, windows) in self.portals.iter() {
// 			for window in windows.north.iter() {
// 				let portal_node = PortalNode {
// 					sector: *sector_id,
// 					window: *window,
// 				};
// 				let node = self.graph.add_node(1);
// 				self.nodes.insert(portal_node, node);
// 			}
// 			for window in windows.east.iter() {
// 				let portal_node = PortalNode {
// 					sector: *sector_id,
// 					window: *window,
// 				};
// 				let node = self.graph.add_node(1);
// 				self.nodes.insert(portal_node, node);
// 			}
// 			for window in windows.south.iter() {
// 				let portal_node = PortalNode {
// 					sector: *sector_id,
// 					window: *window,
// 				};
// 				let node = self.graph.add_node(1);
// 				self.nodes.insert(portal_node, node);
// 			}
// 			for window in windows.west.iter() {
// 				let portal_node = PortalNode {
// 					sector: *sector_id,
// 					window: *window,
// 				};
// 				let node = self.graph.add_node(1);
// 				self.nodes.insert(portal_node, node);
// 			}
// 		}
// 	}
// 	/// Iterate through all sectors and establish graph edges between internal portal windows
// 	fn generate_all_internal_edges(
// 		&mut self,
// 		cost_graphs: &BTreeMap<SectorID, StableGraph<u8, u8, Directed, u8>>,
// 	) {
// 		let portals = &self.portals;
// 		let nodes = &self.nodes;
// 		let portal_graph = &mut self.graph;
// 		for (sector, windows) in portals.iter() {
// 			generate_sector_internal_edges(sector, windows, nodes, cost_graphs, portal_graph);
// 		}
// 	}
// 	/// Iterate through all sectors and establish edges between neighbouring portal windows
// 	fn generate_all_external_edges(&mut self) {
// 		let portals = &self.portals;
// 		let nodes = &self.nodes;
// 		let portal_graph = &mut self.graph;
// 		for (sector, windows) in portals.iter() {
// 			generate_sector_external_edges(sector, windows, nodes, portal_graph, portals);
// 		}
// 	}
// }

// fn walk_sector_boundary(
// 	origin_sector: &SectorID,
// 	origin_field: &CostField,
// 	scaled_costs: &BTreeMap<SectorID, CostField>,
// 	ordinal: &Ordinal,
// ) -> Option<Vec<PortalWindow>> {
// 	let adjacent_sector = origin_sector.get_in_ordinal_direction(ordinal, 1);
// 	// proceed if sector exists
// 	if let Some(adjacent_field) = scaled_costs.get(&adjacent_sector) {
// 		// walk along the boundary of the origin sector
// 		// we need the FieldCells along origin boundary and the FieldCells along the inverse boundary of the adjacent sector
// 		let (origin_cells, adjacent_cells) = boundary_field_cells(ordinal);
// 		// identify any windows
// 		let mut windows = vec![];
// 		let mut current_window = vec![];
// 		for (i, origin_cell) in origin_cells.iter().enumerate() {
// 			let origin_cost = origin_field.get_field_cell_value(*origin_cell);
// 			let adjacent_cost = adjacent_field.get_field_cell_value(adjacent_cells[i]);
// 			// if costs are < 255 we record then as part of a portal window
// 			// if we encounter either cost as 255 then we have found the limit of the window
// 			if origin_cost != 255 && adjacent_cost != 255 {
// 				current_window.push(origin_cell);
// 			} else {
// 				if !current_window.is_empty() {
// 					let start = current_window.first().unwrap();
// 					let end = current_window.last().unwrap();
// 					let portal_window = PortalWindow {
// 						start: **start,
// 						end: **end,
// 					};
// 					windows.push(portal_window);
// 					current_window.clear();
// 				}
// 			}
// 		}
// 		Some(windows)
// 	} else {
// 		None
// 	}
// }

// /// Visit each boundary of a sector and create graph edges between each
// /// [PortalNode] that has a path to another one
// fn generate_sector_internal_edges(
// 	sector: &SectorID,
// 	windows: &Windows,
// 	nodes: &BTreeMap<PortalNode, NodeIndex<u32>>,
// 	cost_graphs: &BTreeMap<SectorID, StableGraph<u8, u8, Directed, u8>>,
// 	portal_graph: &mut StableGraph<i32, i32, Directed, u32>,
// ) {
// 	// create a list of all the PortalWindow
// 	let mut window_list: Vec<PortalWindow> = vec![];
// 	window_list.extend(&windows.north);
// 	window_list.extend(&windows.east);
// 	window_list.extend(&windows.south);
// 	window_list.extend(&windows.west);
// 	// we need to cost graph of this sector to verify if two portal windows can
// 	// actually path to one another
// 	let cost_graph = cost_graphs.get(sector).unwrap();
// 	// iter over all the windows doubly and establish edges for windows that can see each other
// 	for this_window in window_list.iter() {
// 		for other_window in window_list.iter() {
// 			// don't create an edge for a window going to itself
// 			if this_window == other_window {
// 				continue;
// 			}
// 			// find the midpoint of the two windows
// 			let this_midpoint = this_window.get_midpoint();
// 			let other_midpoint = other_window.get_midpoint();
// 			// check if they pathable via the cost graph
// 			let start = this_midpoint.as_1d_index() as u8;
// 			let is_goal = other_midpoint.as_1d_index() as u8;
// 			let estimate_cost = |_| 0;
// 			if let Some((path_cost, _)) = petgraph::algo::astar::astar(
// 				cost_graph,
// 				start.into(),
// 				|finish| finish == is_goal.into(),
// 				|edge| *edge.weight(),
// 				estimate_cost,
// 			) {
// 				// windows can see each other so create an edge in the portal graph,
// 				// use the cost of this path as the edge weight
// 				let this_node = PortalNode {
// 					sector: *sector,
// 					window: *this_window,
// 				};
// 				let other_node = PortalNode {
// 					sector: *sector,
// 					window: *other_window,
// 				};
// 				let this_node_index = nodes.get(&this_node).unwrap();
// 				let other_node_index = nodes.get(&other_node).unwrap();
// 				portal_graph.add_edge(*this_node_index, *other_node_index, path_cost as i32);
// 			}
// 		}
// 	}
// }

// /// Using each [Ordinal] boundary of a sector lookup neighbouring sector
// /// [PortalWindow] and create edges between them
// fn generate_sector_external_edges(
// 	sector: &SectorID,
// 	windows: &Windows,
// 	nodes: &BTreeMap<PortalNode, NodeIndex<u32>>,
// 	portal_graph: &mut StableGraph<i32, i32, Directed, u32>,
// 	portals: &BTreeMap<SectorID, Windows>,
// ) {
// 	let ordinals = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
// 	for ordinal in ordinals.iter() {
// 		// get the portal windows of this sector for a boundary
// 		let this_portal_windows = windows.get_windows_for_ordinal(ordinal);
// 		// find the adjacent sector so that its windows can be found
// 		let adjacent_sector = sector.get_in_ordinal_direction(ordinal, 1);
// 		// get the mirrored ordinal in the adjacent sector
// 		let adjacent_ordinal = match ordinal {
// 			Ordinal::North => Ordinal::South,
// 			Ordinal::East => Ordinal::West,
// 			Ordinal::South => Ordinal::North,
// 			Ordinal::West => Ordinal::East,
// 			_ => panic!("This should never panic"),
// 		};
// 		if let Some(adjacent_windows) = portals.get(&adjacent_sector) {
// 			let adjacent_portal_windows =
// 				adjacent_windows.get_windows_for_ordinal(&adjacent_ordinal);
// 			for (i, this_portal_window) in this_portal_windows.iter().enumerate() {
// 				let adjacent_portal_window = adjacent_portal_windows.get(i).unwrap();
// 				// create [PortalNode] for each window for looking up the graph indices
// 				let this_portal_node = PortalNode {
// 					sector: *sector,
// 					window: *this_portal_window,
// 				};
// 				let adjacent_portal_node = PortalNode {
// 					sector: adjacent_sector,
// 					window: *adjacent_portal_window,
// 				};
// 				// extract the graph indices
// 				if let Some(this_node_index) = nodes.get(&this_portal_node) {
// 					if let Some(adjacent_node_index) = nodes.get(&adjacent_portal_node) {
// 						// create the directed edge from this to adjacent
// 						portal_graph.add_edge(*this_node_index, *adjacent_node_index, 1);
// 					}
// 				}
// 			}
// 		}
// 	}
// }

// /// Get [FieldCell] sets along an [Ordinal] boundary. The first set are the cells along the [Ordinal]. The second set are the corresponding [FieldCell] adjacent in a sector in the [Ordinal] direction (i.e if `ordinal` is `North` then the second set are the [FieldCell] of the adjacent sectors `South` boundary)
// fn boundary_field_cells(ordinal: &Ordinal) -> ([FieldCell; 10], [FieldCell; 10]) {
// 	match ordinal {
// 		Ordinal::North => (
// 			[
// 				FieldCell::new(0, 0),
// 				FieldCell::new(1, 0),
// 				FieldCell::new(2, 0),
// 				FieldCell::new(3, 0),
// 				FieldCell::new(4, 0),
// 				FieldCell::new(5, 0),
// 				FieldCell::new(6, 0),
// 				FieldCell::new(7, 0),
// 				FieldCell::new(8, 0),
// 				FieldCell::new(8, 0),
// 			],
// 			[
// 				FieldCell::new(0, 9),
// 				FieldCell::new(1, 9),
// 				FieldCell::new(2, 9),
// 				FieldCell::new(3, 9),
// 				FieldCell::new(4, 9),
// 				FieldCell::new(5, 9),
// 				FieldCell::new(6, 9),
// 				FieldCell::new(7, 9),
// 				FieldCell::new(8, 9),
// 				FieldCell::new(9, 9),
// 			],
// 		),
// 		Ordinal::East => (
// 			[
// 				FieldCell::new(9, 0),
// 				FieldCell::new(9, 1),
// 				FieldCell::new(9, 2),
// 				FieldCell::new(9, 3),
// 				FieldCell::new(9, 4),
// 				FieldCell::new(9, 5),
// 				FieldCell::new(9, 6),
// 				FieldCell::new(9, 7),
// 				FieldCell::new(9, 8),
// 				FieldCell::new(9, 9),
// 			],
// 			[
// 				FieldCell::new(0, 0),
// 				FieldCell::new(0, 1),
// 				FieldCell::new(0, 2),
// 				FieldCell::new(0, 3),
// 				FieldCell::new(0, 4),
// 				FieldCell::new(0, 5),
// 				FieldCell::new(0, 6),
// 				FieldCell::new(0, 7),
// 				FieldCell::new(0, 8),
// 				FieldCell::new(0, 9),
// 			],
// 		),
// 		Ordinal::South => (
// 			[
// 				FieldCell::new(0, 9),
// 				FieldCell::new(1, 9),
// 				FieldCell::new(2, 9),
// 				FieldCell::new(3, 9),
// 				FieldCell::new(4, 9),
// 				FieldCell::new(5, 9),
// 				FieldCell::new(6, 9),
// 				FieldCell::new(7, 9),
// 				FieldCell::new(8, 9),
// 				FieldCell::new(9, 9),
// 			],
// 			[
// 				FieldCell::new(0, 0),
// 				FieldCell::new(1, 0),
// 				FieldCell::new(2, 0),
// 				FieldCell::new(3, 0),
// 				FieldCell::new(4, 0),
// 				FieldCell::new(5, 0),
// 				FieldCell::new(6, 0),
// 				FieldCell::new(7, 0),
// 				FieldCell::new(8, 0),
// 				FieldCell::new(8, 0),
// 			],
// 		),
// 		Ordinal::West => (
// 			[
// 				FieldCell::new(0, 0),
// 				FieldCell::new(0, 1),
// 				FieldCell::new(0, 2),
// 				FieldCell::new(0, 3),
// 				FieldCell::new(0, 4),
// 				FieldCell::new(0, 5),
// 				FieldCell::new(0, 6),
// 				FieldCell::new(0, 7),
// 				FieldCell::new(0, 8),
// 				FieldCell::new(0, 9),
// 			],
// 			[
// 				FieldCell::new(9, 0),
// 				FieldCell::new(9, 1),
// 				FieldCell::new(9, 2),
// 				FieldCell::new(9, 3),
// 				FieldCell::new(9, 4),
// 				FieldCell::new(9, 5),
// 				FieldCell::new(9, 6),
// 				FieldCell::new(9, 7),
// 				FieldCell::new(9, 8),
// 				FieldCell::new(9, 9),
// 			],
// 		),
// 		_ => panic!("Ordinal {} cannot be used for boundary walking", ordinal),
// 	}
// }
