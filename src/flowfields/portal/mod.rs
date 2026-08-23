//! Each Sector has up to 4 boundaries with neighbouring Sectors (fewer when
//! the sector is in a corner or along the edge of the game world). Each
//! boundary can contain Portals which indicate a navigable point from the
//! current Sector to a neighbour.
//!
//! A Portal is generated at the midpoint of a boundary - in situations where
//! the `CostField` contains `255` costs along the edge then multiple Portals
//! may be generated at the midpoint of each valid pathable segment along the
//! boundary.
//!
//! Example, Portals exist in pairs on either side of a sector boundary, where
//! a wall causes the same boundary to have multiple Portals
//! ```txt
//!  ____________ ____________ ____________ ____________
//! |            |            |            |            |
//! |            |            |            |            |
//! |         p<-|->p<---->p<-|->p         |            |
//! |            |            |            |            |
//! |____________|____________|____________|____________|
//! |            |            |            |            |
//! |         p<-|->p         |            |            |
//! |            |X           |            |            |
//! |         p<-|->p         |            |            |
//! |____________|____________|____________|____________|
//! ```

use std::collections::BTreeMap;

use bevy::prelude::*;
use petgraph::{Directed, Undirected, graph::NodeIndex, stable_graph::StableGraph};

use crate::flowfields::{
	fields::{Field, FieldCell, cost_field::CostField},
	route::RouteStep,
	sectors::{SectorID, sector_cost::SectorCostFields},
	utilities::CompassDir,
};

/// Describes the start and end indices of a portal
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Debug, Clone, Reflect, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct PortalWindow {
	/// Where the window starts
	start: FieldCell,
	/// Where the window ends
	end: FieldCell,
	/// The side of the sector the window sits on. This is required to distinguish
	/// between portals where `start == end` and it sits in a corner
	boundary: CompassDir,
}

impl PortalWindow {
	/// Init [PortalWindow]
	pub fn new(start: FieldCell, end: FieldCell, compass_dir: CompassDir) -> Self {
		PortalWindow {
			start,
			end,
			boundary: compass_dir,
		}
	}
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
	/// Get the indices of every cell along the window
	pub fn get_all_window_cells(&self) -> Vec<usize> {
		let mut cells = vec![];
		if self.start == self.end {
			cells.push(self.start.as_1d_index());
		} else {
			// based on column being either the same or different walk along the window
			if self.start.get_column() != self.end.get_column() {
				// columns are different so the window runs
				// left to right, either along the northern boundary
				// or the southern one
				for col in self.start.get_column()..=self.end.get_column() {
					cells.push(FieldCell::new(col, self.start.get_row()).as_1d_index());
				}
			} else {
				// columns are the same so the window runs
				// top to bottom, either along the eastern ot western boundary
				for row in self.start.get_row()..=self.end.get_row() {
					cells.push(FieldCell::new(self.start.get_column(), row).as_1d_index());
				}
			}
		}

		cells
	}
	/// Get the side of the sector that the window sits upon
	pub fn get_boundary(&self) -> &CompassDir {
		&self.boundary
	}
}

/// Hold the [PortalWindow] for each boundary
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Debug, Clone, Reflect)]
pub struct Windows {
	/// Windows along sector boundary
	north: Vec<PortalWindow>,
	/// Windows along sector boundary
	east: Vec<PortalWindow>,
	/// Windows along sector boundary
	south: Vec<PortalWindow>,
	/// Windows along sector boundary
	west: Vec<PortalWindow>,
}
impl Windows {
	/// Based on [CompassDir] get the list of windows for that boundary
	fn get_windows_for_compass_dir(&self, compass_dir: &CompassDir) -> &Vec<PortalWindow> {
		match compass_dir {
			CompassDir::North => &self.north,
			CompassDir::East => &self.east,
			CompassDir::South => &self.south,
			CompassDir::West => &self.west,
			_ => panic!(
				"CompassDir {} cannot be used for looking up windows",
				compass_dir
			),
		}
	}
	/// Based on [CompassDir] add a window
	fn add_window(&mut self, window: PortalWindow, compass_dir: &CompassDir) {
		match compass_dir {
			CompassDir::North => self.north.push(window),
			CompassDir::East => self.east.push(window),
			CompassDir::South => self.south.push(window),
			CompassDir::West => self.west.push(window),
			_ => panic!(
				"CompassDir {} cannot be used for recording portal windows",
				compass_dir
			),
		}
	}
	/// Get all [PortalWindow]
	pub fn get_all(&self) -> Vec<PortalWindow> {
		let mut portal_windows: Vec<PortalWindow> = vec![];
		portal_windows.extend(&self.north);
		portal_windows.extend(&self.east);
		portal_windows.extend(&self.south);
		portal_windows.extend(&self.west);

		portal_windows
	}
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
pub struct PortalNode {
	/// The sector the node refers to
	sector: SectorID,
	/// The location of the portal
	window: PortalWindow,
}

/// Describes pathing between sectors
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Debug, Clone, Reflect)]
pub struct Portals {
	/// Records all [PortalWindow] across sectors
	portals: BTreeMap<SectorID, Windows>,
	/// Lookup between [PortalNode] and graph type
	#[reflect(ignore)] //TODO
	nodes: BTreeMap<PortalNode, NodeIndex<u32>>,
	/// Describes which [PortalNode]s can reach each other
	#[reflect(ignore)] //TODO
	graph: StableGraph<i32, i32, Undirected, u32>,
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
	pub fn update_portals(&mut self, sector: &SectorID, sector_costs: &SectorCostFields) {
		// identify sectors around sector
		let mut sectors = vec![*sector];
		for s in sector.get_surrounding_sectors() {
			if sector_costs.get_scaled_costs().contains_key(&s) {
				sectors.push(s);
			}
		}
		// remove nodes and recalculate portals
		let portals = &mut self.portals;
		let nodes = &mut self.nodes;
		let portal_graph = &mut self.graph;
		let scaled_costs = sector_costs.get_scaled_costs();
		let cost_graphs = sector_costs.get_graphs();
		for sector in sectors.iter() {
			// remove windows and nodes
			remove_sector_nodes_and_windows(sector, portals, nodes, portal_graph);
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
		for sector in sectors.iter() {
			let windows = portals.get(sector).unwrap();
			generate_sector_external_edges(sector, windows, nodes, portal_graph, portals);
		}
	}
	/// Attempt to find a portal-portal path from one sector to another
	pub fn find_path(
		&self,
		source_sector: &SectorID,
		source_cell: &FieldCell,
		goal_sector: &SectorID,
		goal_cell: &FieldCell,
		sector_costs: &SectorCostFields,
	) -> Option<Vec<RouteStep>> {
		// handle source and goal sectors being the same
		if *source_sector == *goal_sector {
			// two scenarios
			// 1) actor can clearly path to the goal
			let cost_graph = sector_costs.get_graphs().get(source_sector).unwrap();
			let start = source_cell.as_1d_index() as u16;
			let end = goal_cell.as_1d_index() as u16;
			if petgraph::algo::astar(
				cost_graph,
				start.into(),
				|f| f == end.into(),
				|e| *e.weight(),
				|_| 0,
			)
			.is_some()
			{
				return Some(vec![RouteStep::new(
					source_sector,
					goal_cell.as_1d_index(),
					None,
				)]);
			}
			// 2) wall across sector, meaning actor needs to path into a different
			// sector, "go around" and come back in
			// proceed as normal
		}
		//
		let cost_graphs = sector_costs.get_graphs();
		let portal_graph = &self.graph;

		// find which source portals are accessible from the source_cell
		let source_portals = self.portals.get(source_sector).unwrap().get_all();
		// record nodes and the cost to reach it
		let mut source_nodes = vec![];
		for window in source_portals.iter() {
			let midpoint = window.get_midpoint();
			let cost_graph = cost_graphs.get(source_sector).unwrap();
			// search for a cell route in the costs graph
			let start = source_cell.as_1d_index() as u16;
			let end = midpoint.as_1d_index() as u16;
			if petgraph::algo::astar(
				cost_graph,
				start.into(),
				|f| f == end.into(),
				|e| *e.weight(),
				|_| 0,
			)
			.is_some()
			{
				// lookup the graph node of the portal that's reachable and record it
				let portal_node = PortalNode {
					sector: *source_sector,
					window: *window,
				};
				if let Some(graph_node) = self.nodes.get(&portal_node) {
					source_nodes.push(graph_node);
				}
			}
		}

		// find which portals in the goal sector can reach the target_cell
		let target_portals = self.portals.get(goal_sector).unwrap().get_all();
		// record nodes and the cost to reach it, we use this cost to help fine tune
		// the best portal in the goal sector
		let mut target_nodes = vec![];
		for window in target_portals.iter() {
			let midpoint = window.get_midpoint();
			let cost_graph = cost_graphs.get(goal_sector).unwrap();
			// search for a cell route in the costs graph
			let start = midpoint.as_1d_index() as u16;
			let end = goal_cell.as_1d_index() as u16;
			if let Some((total_cost, _)) = petgraph::algo::astar(
				cost_graph,
				start.into(),
				|f| f == end.into(),
				|e| *e.weight(),
				|_| 0,
			) {
				// lookup the graph node of the portal that's reachable and record it
				let portal_node = PortalNode {
					sector: *goal_sector,
					window: *window,
				};
				if let Some(graph_node) = self.nodes.get(&portal_node) {
					target_nodes.push((graph_node, total_cost as i32));
				}
			}
		}

		// with source and goal PortalNode now attempt to find the best portal-portal
		// path
		//
		// make a record of the best path if it exists
		let mut best_path: Option<(i32, Vec<PortalNode>)> = None;
		for start in source_nodes {
			for (end, cost_from_portal) in target_nodes.iter() {
				if let Some((cost, steps)) = petgraph::algo::astar(
					portal_graph,
					*start,
					|f| f == **end,
					|e| *e.weight(),
					|_| 0,
				) {
					if let Some((best_cost, best_route)) = best_path.as_mut() {
						if (cost + cost_from_portal) < *best_cost {
							*best_cost = cost + cost_from_portal;

							let mut p_node_route = vec![];
							for s in steps {
								for (pn, pi) in self.nodes.iter() {
									if *pi == s {
										p_node_route.push(*pn);
									}
								}
							}
							*best_route = p_node_route;
						}
					} else {
						let mut p_node_route = vec![];
						for s in steps {
							for (pn, pi) in self.nodes.iter() {
								if *pi == s {
									p_node_route.push(*pn);
								}
							}
						}
						best_path = Some((cost + cost_from_portal, p_node_route));
					}
				}
			}
		}
		if let Some((_, node_path)) = best_path {
			// record the path but only make records of steps with exit portals and the
			// final goal.
			// Most steps are effectively pairs describing the entry portal and exit
			// portal. We only care about the very first step, the last step and steps
			// that contain portals exiting a sector
			let mut final_route = vec![];
			for (i, segment) in node_path.iter().enumerate() {
				if i == node_path.len() - 1 {
					// the goal sector uses true goal, not portal
					final_route.push(RouteStep::new(
						&segment.sector,
						goal_cell.as_1d_index(),
						None,
					));
				} else if i == 0 {
					// start sector is just the exit portals
					final_route.push(RouteStep::new(
						&segment.sector,
						segment.window.get_midpoint().as_1d_index(),
						Some(segment.window),
					));
				} else if i % 2 == 0 {
					// other than start and end only record even steps, those are the exit
					// portal ones
					final_route.push(RouteStep::new(
						&segment.sector,
						segment.window.get_midpoint().as_1d_index(),
						Some(segment.window),
					));
				}
			}
			Some(final_route)
		} else {
			None
		}
	}
	/// Get a reference to the map of portal windows
	pub fn get_portals(&self) -> &BTreeMap<SectorID, Windows> {
		&self.portals
	}
}

/// Walk through each boundary of sector and compute the [PortalWindow]s
fn generate_sector_portals(
	portals: &mut BTreeMap<SectorID, Windows>,
	scaled_costs: &BTreeMap<SectorID, CostField>,
	origin_sector: &SectorID,
	origin_field: &CostField,
) {
	let compass_dirs = [
		CompassDir::North,
		CompassDir::East,
		CompassDir::South,
		CompassDir::West,
	];
	for compass_dir in compass_dirs.iter() {
		if let Some(windows) =
			walk_sector_boundary(origin_sector, origin_field, scaled_costs, compass_dir)
		{
			for window in windows.iter() {
				let value = portals.get_mut(origin_sector).unwrap();
				value.add_window(*window, compass_dir);
			}
		}
	}
}

/// Walk along the [CompassDir] boundary of a sector determine where portal windows are
fn walk_sector_boundary(
	origin_sector: &SectorID,
	origin_field: &CostField,
	scaled_costs: &BTreeMap<SectorID, CostField>,
	compass_dir: &CompassDir,
) -> Option<Vec<PortalWindow>> {
	let adjacent_sector = origin_sector.get_in_compass_direction(compass_dir, 1);
	// proceed if sector exists
	if let Some(adjacent_field) = scaled_costs.get(&adjacent_sector) {
		// walk along the boundary of the origin sector
		// we need the FieldCells along origin boundary and the FieldCells along the inverse boundary of the adjacent sector
		let (origin_cells, adjacent_cells) = boundary_field_cells(compass_dir);
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
						boundary: *compass_dir,
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
					boundary: *compass_dir,
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
	portal_graph: &mut StableGraph<i32, i32, Undirected, u32>,
	nodes: &mut BTreeMap<PortalNode, NodeIndex<u32>>,
) {
	let compass_dirs = [
		CompassDir::North,
		CompassDir::East,
		CompassDir::South,
		CompassDir::West,
	];
	for compass_dir in compass_dirs.iter() {
		for window in windows.get_windows_for_compass_dir(compass_dir).iter() {
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
	portal_graph: &mut StableGraph<i32, i32, Undirected, u32>,
) {
	// create a list of all the PortalWindow
	let mut window_list: Vec<PortalWindow> = vec![];
	window_list.extend(&windows.north);
	window_list.extend(&windows.east);
	window_list.extend(&windows.south);
	window_list.extend(&windows.west);
	// we need to cost graph of this sector to verify if two portal windows can
	// actually path to one another
	let cost_graph = cost_graphs.get(sector).expect("Sector is missing a graph");
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
				// edges are undirected so only add an edge if it doesn't exist
				//TODO: do this check early to avoid unnecessary astar?
				if !portal_graph.contains_edge(*this_node_index, *other_node_index) {
					portal_graph.add_edge(*this_node_index, *other_node_index, path_cost as i32);
				}
			}
		}
	}
}

/// Using each [CompassDir] boundary of a sector lookup neighbouring sector
/// [PortalWindow] and create edges between them
fn generate_sector_external_edges(
	sector: &SectorID,
	windows: &Windows,
	nodes: &BTreeMap<PortalNode, NodeIndex<u32>>,
	portal_graph: &mut StableGraph<i32, i32, Undirected, u32>,
	portals: &BTreeMap<SectorID, Windows>,
) {
	let compass_dirs = [
		CompassDir::North,
		CompassDir::East,
		CompassDir::South,
		CompassDir::West,
	];
	for compass_dir in compass_dirs.iter() {
		// get the portal windows of this sector for a boundary
		let this_portal_windows = windows.get_windows_for_compass_dir(compass_dir);
		// find the adjacent sector so that its windows can be found
		let adjacent_sector = sector.get_in_compass_direction(compass_dir, 1);
		// get the mirrored compass dir in the adjacent sector
		let adjacent_compass = compass_dir.inverse();
		if let Some(adjacent_windows) = portals.get(&adjacent_sector) {
			let adjacent_portal_windows =
				adjacent_windows.get_windows_for_compass_dir(&adjacent_compass);
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
				if let Some(this_node_index) = nodes.get(&this_portal_node)
					&& let Some(adjacent_node_index) = nodes.get(&adjacent_portal_node)
				{
					// edges are undirected, only add if one doesn't exist
					if !portal_graph.contains_edge(*this_node_index, *adjacent_node_index) {
						// create the undirected edge from this to adjacent
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
	portal_graph: &mut StableGraph<i32, i32, Undirected, u32>,
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

/// Get [FieldCell] sets along an [CompassDir] boundary. The first set are the cells along the [CompassDir]. The second set are the corresponding [FieldCell] adjacent in a sector in the [CompassDir] direction (i.e if `compass_dir` is `North` then the second set are the [FieldCell] of the adjacent sectors `South` boundary)
fn boundary_field_cells(compass_dir: &CompassDir) -> ([FieldCell; 10], [FieldCell; 10]) {
	match compass_dir {
		CompassDir::North => (
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
		CompassDir::East => (
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
		CompassDir::South => (
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
		CompassDir::West => (
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
		_ => panic!(
			"CompassDir {} cannot be used for boundary walking",
			compass_dir
		),
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use crate::flowfields::dimensions::Dimensions;

	use super::*;

	#[test]
	fn window_midpoint1() {
		let window = PortalWindow {
			start: FieldCell::new(0, 3),
			end: FieldCell::new(0, 9),
			boundary: CompassDir::West,
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
			boundary: CompassDir::East,
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
			boundary: CompassDir::West,
		};

		let actual = FieldCell::new(0, 3);
		let result = window.get_midpoint();
		assert_eq!(actual, result);
	}

	#[test]
	fn windows_compass_dir() {
		let north = vec![PortalWindow {
			start: FieldCell::new(0, 0),
			end: FieldCell::new(5, 0),
			boundary: CompassDir::North,
		}];
		let east = vec![PortalWindow {
			start: FieldCell::new(9, 0),
			end: FieldCell::new(9, 4),
			boundary: CompassDir::East,
		}];
		let south = vec![PortalWindow {
			start: FieldCell::new(2, 9),
			end: FieldCell::new(7, 9),
			boundary: CompassDir::South,
		}];
		let west = vec![PortalWindow {
			start: FieldCell::new(0, 4),
			end: FieldCell::new(0, 8),
			boundary: CompassDir::West,
		}];
		let windows = Windows {
			north: north.clone(),
			east: east.clone(),
			south: south.clone(),
			west: west.clone(),
		};

		assert_eq!(
			north,
			*windows.get_windows_for_compass_dir(&CompassDir::North)
		);
		assert_eq!(
			east,
			*windows.get_windows_for_compass_dir(&CompassDir::East)
		);
		assert_eq!(
			south,
			*windows.get_windows_for_compass_dir(&CompassDir::South)
		);
		assert_eq!(
			west,
			*windows.get_windows_for_compass_dir(&CompassDir::West)
		);
	}

	#[test]
	fn window_add() {
		let mut windows = Windows {
			north: vec![PortalWindow {
				start: FieldCell::new(0, 0),
				end: FieldCell::new(5, 0),
				boundary: CompassDir::North,
			}],
			east: vec![],
			south: vec![],
			west: vec![],
		};
		let new = PortalWindow {
			start: FieldCell::new(7, 0),
			end: FieldCell::new(9, 0),
			boundary: CompassDir::North,
		};
		windows.add_window(new, &CompassDir::North);

		let actual = vec![
			PortalWindow {
				start: FieldCell::new(0, 0),
				end: FieldCell::new(5, 0),
				boundary: CompassDir::North,
			},
			PortalWindow {
				start: FieldCell::new(7, 0),
				end: FieldCell::new(9, 0),
				boundary: CompassDir::North,
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
				boundary: CompassDir::North,
			}],
			east: vec![PortalWindow {
				start: FieldCell::new(9, 0),
				end: FieldCell::new(9, 4),
				boundary: CompassDir::East,
			}],
			south: vec![PortalWindow {
				start: FieldCell::new(2, 9),
				end: FieldCell::new(7, 9),
				boundary: CompassDir::South,
			}],
			west: vec![PortalWindow {
				start: FieldCell::new(0, 4),
				end: FieldCell::new(0, 8),
				boundary: CompassDir::West,
			}],
		};
		let result = windows.remove_all();
		let actual = vec![
			PortalWindow {
				start: FieldCell::new(0, 0),
				end: FieldCell::new(5, 0),
				boundary: CompassDir::North,
			},
			PortalWindow {
				start: FieldCell::new(9, 0),
				end: FieldCell::new(9, 4),
				boundary: CompassDir::East,
			},
			PortalWindow {
				start: FieldCell::new(2, 9),
				end: FieldCell::new(7, 9),
				boundary: CompassDir::South,
			},
			PortalWindow {
				start: FieldCell::new(0, 4),
				end: FieldCell::new(0, 8),
				boundary: CompassDir::West,
			},
		];
		assert_eq!(actual, result);
	}

	#[test]
	fn walk() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_radius = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let sector_costs = SectorCostFields::new(&dimensions);

		let origin_sector = SectorID::new(0, 0);
		let origin_field = sector_costs.get_scaled_costs().get(&origin_sector).unwrap();
		let scaled_costs = sector_costs.get_scaled_costs();

		let compass_dir = &CompassDir::North;
		let north = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, compass_dir);
		assert!(north.is_none());

		let compass_dir = &CompassDir::East;
		let east = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, compass_dir);
		assert!(1 == east.unwrap().len());

		let compass_dir = &CompassDir::South;
		let south = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, compass_dir);
		assert!(1 == south.unwrap().len());

		let compass_dir = &CompassDir::West;
		let west = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, compass_dir);
		assert!(west.is_none());
	}

	// walk a boundary where the CostField has an initial wall on the boundary, this
	// should prove that multiple portals are initialised
	#[test]
	fn walk_initial_wall() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_radius = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
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

		let compass_dir = &CompassDir::East;
		let east = walk_sector_boundary(&origin_sector, origin_field, scaled_costs, compass_dir);
		assert!(2 == east.unwrap().len());

		// ensure the neighbour without a wall registers
		// the same number of portals
		let n_sector = SectorID::new(1, 0);
		let n_field = sector_costs.get_scaled_costs().get(&n_sector).unwrap();
		let scaled_costs = sector_costs.get_scaled_costs();

		let n_compass_dir = &CompassDir::West;
		let west = walk_sector_boundary(&n_sector, n_field, scaled_costs, n_compass_dir);
		assert!(2 == west.unwrap().len());
	}

	#[test]
	fn sector_portals() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_radius = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let sector_costs = SectorCostFields::new(&dimensions);
		let mut portals = Portals::default();

		portals.generate_all_portals(sector_costs.get_scaled_costs());

		// check for PortalWindow existence on boundaries
		let sector1 = SectorID::new(0, 0);
		let s1_north = &portals.portals.get(&sector1).unwrap().north;
		assert!(s1_north.is_empty());
		let s1_east = &portals.portals.get(&sector1).unwrap().east;
		assert!(1 == s1_east.len());
		let s1_south = &portals.portals.get(&sector1).unwrap().south;
		assert!(1 == s1_south.len());
		let s1_west = &portals.portals.get(&sector1).unwrap().west;
		assert!(s1_west.is_empty());
	}

	//  ____________ ____________
	// |            |            |
	// |            |            |
	// |           p|p           |
	// |     p      |     p      |
	// |____________|____________|
	// |     p      |     p      |
	// |            |            |
	// |           p|p           |
	// |            |            |
	// |____________|____________|
	#[test]
	fn default_node_edge_count() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_radius = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let sector_costs = SectorCostFields::new(&dimensions);
		let portals = Portals::new(&sector_costs);

		let actual_node_count = 8;
		let result_node_count = portals.graph.node_count();
		assert_eq!(actual_node_count, result_node_count);

		let actual_edge_count = 8;
		let result_edge_count = portals.graph.edge_count();
		assert_eq!(actual_edge_count, result_edge_count);
	}

	// check nodes and edges when the SectorCost begins with a wall
	//
	//  ____________ ____________
	// |            |            |
	// |            |            |
	// |           p|p           |
	// |     p      |     p      |
	// |____________|____________|
	// |     p      |     p      |
	// |           p|p           |
	// |            |X           |
	// |           p|p           |
	// |____________|____________|
	#[test]
	fn graph_counts_initial_wall() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_radius = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
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

		let actual_edge_count = 13;
		let result_edge_count = portals.graph.edge_count();
		assert_eq!(actual_edge_count, result_edge_count);
	}

	//  ____________ ____________
	// |            |            |
	// |            |            |
	// |            |p           |
	// |            |     p      |
	// |____________|____________|
	// |     p      |     p      |
	// |            |            |
	// |           p|p           |
	// |            |            |
	// |____________|____________|
	#[test]
	fn remove_nodes() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_radius = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let sector_costs = SectorCostFields::new(&dimensions);
		let mut portals = Portals::new(&sector_costs);

		let nodes = &mut portals.nodes;
		let portal_graph = &mut portals.graph;
		remove_sector_nodes_and_windows(
			&SectorID::new(0, 0),
			&mut portals.portals,
			nodes,
			portal_graph,
		);

		assert!(6 == portals.graph.node_count());
		assert!(5 == portals.graph.edge_count());
	}

	// generate portals, then update SectorCosts, then update portals
	// internal_edges:top(1+3+3+1), bottom(1+3+3+1),S[3,1](3)...6,10,6
	// external_edges:18
	//  ____________ ____________ ____________ ____________
	// |            |            |            |            |
	// |            |            |            |            |
	// |           p|p          p|p          p|p           |
	// |     p      |     p      |     p      |     p      |
	// |____________|____________|____________|____________|
	// |     p      |     p      |     p      |     p      |
	// |           p|p           |            |            |
	// |            |X          p|p          p|p           |
	// |     p     p|p    p      |     p      |     p      |
	// |____________|____________|____________|____________|
	// |     p      |     p      |     p      |     p      |
	// |            |            |            |            |
	// |           p|p          p|p          p|p           |
	// |            |            |            |            |
	// |____________|____________|____________|____________|
	#[test]
	fn graph_updated() {
		let origin = (0.0, 0.0);
		let size = (40.0, 30.0);
		let world_unit_size = 1.0;
		let actor_radius = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let mut sector_costs = SectorCostFields::new(&dimensions);
		let mut portals = Portals::new(&sector_costs);

		sector_costs.set_field_cost(
			&SectorID::new(1, 1),
			&FieldCell::new(0, 5),
			255,
			&dimensions,
		);

		portals.update_portals(&SectorID::new(1, 1), &sector_costs);

		let actual_node_count = 36;
		let result_node_count = portals.graph.node_count();
		assert_eq!(actual_node_count, result_node_count);

		let actual_edge_count = 59;
		let result_edge_count = portals.graph.edge_count();
		assert_eq!(actual_edge_count, result_edge_count);
	}

	#[test]
	fn best_path() {
		let origin = (0.0, 0.0);
		let size = (40.0, 30.0);
		let world_unit_size = 1.0;
		let actor_radius = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
		let sector_costs = SectorCostFields::new(&dimensions);
		let portals = Portals::new(&sector_costs);

		let source_sector = SectorID::new(0, 0);
		let source_cell = FieldCell::new(3, 4);
		let goal_sector = SectorID::new(3, 0);
		let goal_cell = FieldCell::new(5, 5);
		let path = portals.find_path(
			&source_sector,
			&source_cell,
			&goal_sector,
			&goal_cell,
			&sector_costs,
		);

		let actual_len = 4;
		assert_eq!(actual_len, path.unwrap().len());
	}
}
