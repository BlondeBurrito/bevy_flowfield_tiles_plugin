//! When an agent needs to path somewhere it is initially given a path based on moving from one portal
//! to another portal/end sector. The path is calculated from the [PortalGraph] which records the
//! points of navigation (`nodes`), the the paths bewteen them (`edges`).
//!
//! This ensures responsiveness so when a player issues a movement order
//! the agent immediately starts pathing. In the background the other components of the Flowfields can
//! calcualte a perfect path which can then supersede using portals to path when it's ready

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::portals::Portals;
use crate::prelude::*;
use bevy::prelude::*;

const SECTOR_BOUNDARY_PORTAL_PORTAL_DISTANCE: i32 = 5;

/// A graph contains a series of [PortalNode] which denotes the Sector and FieldCell of a portal
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Reflect, Clone, Copy, Debug, Hash)]
struct PortalNode {
	sector_id: SectorID,
	portal_cell: FieldCell,
	weight: i32,
}

impl PortalNode {
	fn new(sector_id: SectorID, portal_cell: FieldCell, weight: i32) -> Self {
		PortalNode {
			sector_id,
			portal_cell,
			weight
		}
	}
	/// Get the [SectorID]
	fn get_sector(&self) -> &SectorID {
		&self.sector_id
	}
	/// Get the portals [FieldCell] as a referance
	fn get_portal_cell(&self) -> &FieldCell {
		&self.portal_cell
	}
	/// Get the weight (how expensive) of traversing to the nodes
	fn get_weight(&self) -> i32 {
		self.weight
	}
}

impl Ord for PortalNode {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(self.sector_id, self.portal_cell).cmp(&(other.sector_id, other.portal_cell))
	}
}

impl PartialOrd for PortalNode {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for PortalNode {
	fn eq(&self, other: &Self) -> bool {
		self.sector_id == other.sector_id && self.portal_cell == other.portal_cell
	}
}
impl Eq for PortalNode {}

/// An edge between [PortalNode] s comes in two varieties.
/// 
/// Internal means it's an edge to another Portal within the same sector, External means it is a Portal to a neighbouring sector Portal
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Reflect, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Copy)]
enum Direction {
	#[default]
	Internal,
	External,
}

impl Direction {
	fn flip(self) -> Direction{
		if self == Direction::Internal {
			Direction::External
		} else {
			Direction::Internal
		}
	}
}

/// An edge indicates a link between two [PortalNode]s
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Reflect, Clone, Debug)]
struct PortalEdge {
	source_node: PortalNode,
	target_node: PortalNode,
	distance: i32,
	direction: Direction,
}

impl Ord for PortalEdge {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(self.source_node, self.target_node).cmp(&(other.source_node, other.target_node))
	}
}

impl PartialOrd for PortalEdge {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for PortalEdge {
	fn eq(&self, other: &Self) -> bool {
		self.source_node == other.source_node && self.target_node == other.target_node && self.direction == other.direction
	}
}
impl Eq for PortalEdge {}

impl PortalEdge {
	/// Create a new [PortalEdge] with target portal `node` and a navigation weighting
	fn new(source_node: PortalNode, target_node: PortalNode, distance: i32, direction: Direction) -> Self {
		PortalEdge { source_node, target_node, distance, direction}
	}
	/// Get the node at the source of this edge
	fn get_source_node(&self) -> &PortalNode {
		&self.source_node
	}
	/// Get the node this edge points towards
	fn get_target_node(&self) -> &PortalNode {
		&self.target_node
	}
	/// Get the distance ([FieldCell] count) to this edge
	fn get_distance(&self) -> i32 {
		self.distance
	}
	/// Get the [Direction] of this edge
	fn get_direction(&self) -> &Direction {
		&self.direction
	}
	fn contains_source(&self, source_node: PortalNode) -> bool {
		*self.get_source_node() == source_node
	}
	fn contains_target(&self, target_node: PortalNode) -> bool {
		*self.get_target_node() == target_node
	}
}
//TODO? map of map? outer map = SectorID, inner map FieldCell/(the portal) => Vec
//TODO reflect
///
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Default, Clone, Debug)]
// #[reflect(Component)]
pub struct PortalGraph {
	nodes: BTreeSet<PortalNode>,
	edges: BTreeSet<PortalEdge>,
}

impl PortalGraph {
	/// Get a reference to the set of nodes in the graph
	fn get_nodes(&self) -> &BTreeSet<PortalNode> {
		&self.nodes
	}
	/// Get a mutable reference to the set of nodes in the graph
	fn get_nodes_mut(&mut self) -> &mut BTreeSet<PortalNode> {
		&mut self.nodes
	}
	/// From a given Sector and FielCell obtain the PortalNode representation of it
	fn get_node(&self, sector: &SectorID, cell: &FieldCell) -> Option<PortalNode> {
		for n in self.get_nodes().iter() {
			if n.get_sector() == sector && n.get_portal_cell() == cell {
				return Some(*n)
			}
		}
		None
	}
	/// Get a reference to the edges of the graph
	fn get_edges(&self) -> &BTreeSet<PortalEdge> {
		&self.edges
	}
	/// Get a mutable reference to the edges of the graph
	fn get_edges_mut(&mut self) -> &mut BTreeSet<PortalEdge> {
		&mut self.edges
	}
	/// For the given `sector_id` identify any nodes in the graph that correspond to it
	fn get_nodes_containg_sector(&self, sector_id: &SectorID) -> Vec<&PortalNode> {
		let mut nodes = vec![];
		for node in self.get_nodes().iter() {
			if *sector_id == *node.get_sector() {
				nodes.push(node);
			}
		}
		nodes
	}
	/// For the given `sector_id` identify any nodes in the graph that correspond to it
	fn get_nodes_containg_sector_mut(&mut self, sector_id: &SectorID) -> Vec<&PortalNode> {
		let mut nodes = vec![];
		for node in self.get_nodes_mut().iter() {
			if *sector_id == *node.get_sector() {
				nodes.push(node);
			}
		}
		nodes
	}
	fn add_edge(&mut self, edge: PortalEdge) {
		self.edges.insert(edge);
		// //TODO shortcut used by update_graph in reaplce_old_neighbours, expensive?
		// if !self.edges.contains(&edge) {
		// 	edges.insert(edge);
		// }
	}
	/// Remove a node from the graph and any edges that reference it
	fn remove_node(&mut self, node: &PortalNode) {
		let mut edges_to_remove = vec![];
		let edges = self.get_edges_mut().clone();
		for edge in edges.iter() {
			if edge.contains_source(*node)|| edge.contains_target(*node) {
				edges_to_remove.push(edge);
			}
		}
		for edge in edges_to_remove.iter() {
			self.remove_edge(edge);
		}
		self.nodes.remove(node);
	}
	/// Remove an edge from the graph
	fn remove_edge(&mut self, edge: &PortalEdge) {
		self.edges.remove(edge);
	}
	/// Create a new instance of [PortalGraph] with inital nodes and edges built
	pub fn new(
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) -> Self {
		let mut graph = PortalGraph::default();
		graph.insert_all_portal_nodes(sector_portals, sector_cost_fields);
		graph.build_all_internal_sector_edges(sector_portals, sector_cost_fields);
		graph.build_all_external_sector_edges(sector_portals, sector_cost_fields, map_dimensions);
		graph
	}
	/// Iterate over the calcualted portals and insert a [PortalNode] for each
	fn insert_all_portal_nodes(&mut self, sector_portals: &SectorPortals, sector_cost_fields: &SectorCostFields) {
		let portals_map = sector_portals.get();
		for (sector_id, portals) in portals_map {
			for p in portals.get().iter() {
				for cell in p {
					let weight = sector_cost_fields.get_scaled().get(sector_id).unwrap().get_field_cell_value(*cell) as i32;
					let portal_node = PortalNode::new(*sector_id, *cell, weight);
					// info!("Inserting {:?}", portal_node);
					// info!("Current graph {:?}", self);
					self.insert_portal_node(portal_node);
				}
			}
		}
	}
	fn insert_portal_node(&mut self, node: PortalNode){
		// should never contian one already?
		if self.get_nodes_mut().contains(&node) {
			// TODO diagonal case?
		} else {
			self.get_nodes_mut().insert(node);
		}
		// if let Some(v) = self.get_graph_mut().insert(node, vec![]) {
		// 	panic!("Graph already contains {:?} with value {:?}", node, v);
		// }
	}
	/// Create [PortalEdge]s between portals within all sectors
	fn build_all_internal_sector_edges(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
	) {
		for (sector_id, portals) in sector_portals.get() {
			// get the cost field for this sector
			let cost_field = sector_cost_fields.get_scaled().get(sector_id).unwrap();
			// create a combined list of portal points which can be iterated over to link a portal
			// to all portals in the sector
			let all_sector_portals = portals.get_all();
			// create edges between portals that can see each other
			self.build_sector_internal_edges(sector_id, cost_field, &all_sector_portals);
		}
	}
	/// Create [PortalEdge]s between the [Portals] of the supplied `sector_id`
	fn build_sector_internal_edges(
		&mut self,
		sector_id: &SectorID,
		cost_field: &CostField,
		portals: &Vec<FieldCell>,
	) {
		// create edges between portals that can see each other
		for (i, source) in portals.iter().enumerate() {
			for (j, target) in portals.iter().enumerate() {
				if i == j {
					continue;
				} else {
					let is_visible =
						cost_field.can_internal_portal_pair_see_each_other(*source, *target);
					if is_visible.0 {
						// create the edge
						let s_weight = cost_field.get_field_cell_value(*source) as i32;
						let source_node = PortalNode::new(*sector_id, *source, s_weight);
						let t_weight = cost_field.get_field_cell_value(*target) as i32;
						let target_node = PortalNode::new(*sector_id, *target, t_weight);
						let distance = is_visible.1;
						let edge = PortalEdge::new(source_node, target_node, distance, Direction::Internal);
						self.add_edge(edge);
					}
				}
			}
		}
	}
	/// Create [PortalEdge]s at the portal crossing/boundary [FieldCell]s for each neighbouring sector
	fn build_all_external_sector_edges(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) {
		for (sector_id, portals) in sector_portals.get() {
			// sectors bordering this one
			let sector_neighbours =
				map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(sector_id);
			self.build_sector_external_edges(sector_portals, sector_cost_fields, sector_id, portals, sector_neighbours);
		}
	}
	/// Create [PortalEdge]s from the `portals` of this `sector_id` to its neighbour portals
	fn build_sector_external_edges(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		sector_id: &SectorID,
		portals: &Portals,
		sector_neighbours: Vec<(Ordinal, SectorID)>,
	) {
		for (ordinal, neighbour_id) in sector_neighbours.iter() {
			let cost_field_source = sector_cost_fields.get_scaled().get(sector_id).unwrap();
			let cost_field_target = sector_cost_fields.get_scaled().get(neighbour_id).unwrap();
			// get portals along boundary of current sector being worked on
			let boundary_portals = portals.get_portals_for_side(ordinal);
			// get inverse ordinal portals along boundary of the neighbour
			let neighbour_portals = sector_portals.get().get(neighbour_id).unwrap();
			let neighbour_boundary_portals =
				neighbour_portals.get_portals_for_side(&ordinal.inverse());
			// create edges between the portals
			for (i, cell) in boundary_portals.iter().enumerate() {
				// source of the edge
				let weight = cost_field_source.get_field_cell_value(*cell) as i32;
				let source_node = PortalNode::new(*sector_id, *cell, weight);
				// target of the edge
				// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
				let neighbour_portal = neighbour_boundary_portals[i];
				let weight = cost_field_target.get_field_cell_value(neighbour_portal) as i32;
				let target_node = PortalNode::new(*neighbour_id, neighbour_portal, weight);
				// add the dge
				let edge = PortalEdge::new(source_node, target_node, SECTOR_BOUNDARY_PORTAL_PORTAL_DISTANCE, Direction::External);
				self.add_edge(edge);
			}
		}
	}
	/// Replaces the current graph with a fresh one
	pub fn reset_graph(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) -> &mut Self {
		self.get_nodes_mut().clear();
		self.get_edges_mut().clear();
		self.insert_all_portal_nodes(sector_portals, sector_cost_fields);
		self.build_all_internal_sector_edges(sector_portals, sector_cost_fields);
		self.build_all_external_sector_edges(sector_portals, sector_cost_fields, map_dimensions);
		self
	}
}

impl PortalGraph {
	/// When a [CostField] is updated the corresponding [Portals] should be updated. This means that
	/// the [PortalGraph]'s `graph` may no longer accurately reflect how to move from one sector to
	/// another. This method will recalculate the nodes and edges of the supplied sector and
	/// its neighbouring sectors.
	///
	/// # This must run after any updates to a [Portals]!
	pub fn update_graph(
		&mut self,
		changed_sector: SectorID,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) -> &mut Self {
		let sectors_to_rebuild =
			map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(&changed_sector);
		// remove old neighbour nodes along the boundary of the changed sector
		// and insert new nodes for the new portals
		self.replace_old_neighbour_nodes(&sectors_to_rebuild, sector_portals, sector_cost_fields);
		// remove all PortalNodes using the changed sector, replace them with
		// new nodes and calcualte the internal edges of the sector
		self.replace_changed_sector_nodes(&changed_sector, sector_portals, sector_cost_fields);
		// rebuild edges from neighbours to the changed sector and from changed to neighbour
		for (ordinal, neighbour_id) in sectors_to_rebuild {
			// get new portals along the changed sectors bounary
			let portals_array_changed_sector = sector_portals.get().get(&changed_sector).unwrap();
			let portals_changed = portals_array_changed_sector.get_portals_for_side(&ordinal);
			let cost_field_changed = sector_cost_fields.get_scaled().get(&changed_sector).unwrap();
			// get portals changed along neighbours boundary
			let ord_pointing_at_changed = ordinal.inverse();
			let portals_array_neighbour = sector_portals.get().get(&neighbour_id).unwrap();
			let portals_neighbour = portals_array_neighbour.get_portals_for_side(&ord_pointing_at_changed);
			let cost_field_neighbour = sector_cost_fields.get_scaled().get(&neighbour_id).unwrap();
			// create edges from changed sector to neighbour
			for (i, cell) in portals_changed.iter().enumerate() {
				// create the source node
				let weight_changed = cost_field_changed.get_field_cell_value(*cell) as i32;
				let source_node = PortalNode::new(changed_sector, *cell, weight_changed);
				// create the target node
				// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
				let neighbour_portal = portals_neighbour[i];
				let weight_neighbour = cost_field_neighbour.get_field_cell_value(neighbour_portal) as i32;
				let target_node = PortalNode::new(neighbour_id, neighbour_portal, weight_neighbour);
				// create the edge
				let edge = PortalEdge::new(source_node, target_node, SECTOR_BOUNDARY_PORTAL_PORTAL_DISTANCE, Direction::External);
				self.add_edge(edge);
			}
			// create edges from neighbour to changed sector
			for (i, cell) in portals_neighbour.iter().enumerate() {
				// create the source node in the neighbour
				let weight_neighbour = cost_field_neighbour.get_field_cell_value(*cell) as i32;
				let source_node = PortalNode::new(neighbour_id, *cell, weight_neighbour);
				// create the target node in the changed sector
				// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
				let changed_portal = portals_changed[i];
				let weight_changed = cost_field_changed.get_field_cell_value(changed_portal) as i32;
				let target_node = PortalNode::new(changed_sector, changed_portal, weight_changed);
				// create the edge
				let edge = PortalEdge::new(source_node, target_node, SECTOR_BOUNDARY_PORTAL_PORTAL_DISTANCE, Direction::External);
				self.add_edge(edge);
			}
		}
		self
	}

	/// Iterate through the graph finding neighbouring [PortalNode]s with edges
	/// to the updated sector and remove them from the graph and insert new
	/// nodes to reflect new portals and rebuild their internal edges
	fn replace_old_neighbour_nodes(&mut self, sectors_to_rebuild: &Vec<(Ordinal, SectorID)>, sector_portals: &SectorPortals, sector_cost_fields: &SectorCostFields,) {
		// remove portal nodes in neighbours that point towards the changed sector and
		// based on the new portals create new nodes
		let graph_copy = self.clone();
		for (ordinal, neighbour_id) in sectors_to_rebuild.iter() {
			// get the ordinal going from the neighbour to the changed sector
			// this can be used to find if an edge in the neighbour that links
			// to the changed sector
			let ord_pointing_at_changed = ordinal.inverse();
			// remove edges that reference the portals along a boundary
			let mut edge_node_to_remove = vec![];
			// remove the portals themselves - !! if they are no longer portals in sector_portals (otherwise subsequent updates may cause portals to be deleted and re-added with missing edges from corner adjacent sectors)
			let portals_array = sector_portals.get().get(&neighbour_id).unwrap();
			let portals = portals_array.get_portals_for_side(&ord_pointing_at_changed);
			let mut nodes_to_remove = vec![];
			match ord_pointing_at_changed {
				Ordinal::North => {
					for node in graph_copy.get_nodes().iter() {
						if *node.get_sector() == *neighbour_id {
							if node.get_portal_cell().get_row() == 0 {
								// self.get_graph_mut().remove(node).unwrap();
								edge_node_to_remove.push(node);
								if !portals.contains(node.get_portal_cell()) {
									nodes_to_remove.push(node);
								}
							}
						}
					}
				}
				Ordinal::East => {
					for node in graph_copy.get_nodes().iter() {
						if *node.get_sector() == *neighbour_id {
							if node.get_portal_cell().get_column() == FIELD_RESOLUTION - 1 {
								// self.get_graph_mut().remove(node).unwrap();
								edge_node_to_remove.push(node);
								if !portals.contains(node.get_portal_cell()) {
									nodes_to_remove.push(node);
								}
							}
						}
					}
				}
				Ordinal::South => {
					for node in graph_copy.get_nodes().iter() {
						if *node.get_sector() == *neighbour_id {
							if node.get_portal_cell().get_row() == FIELD_RESOLUTION - 1 {
								// self.get_graph_mut().remove(node).unwrap();
								edge_node_to_remove.push(node);
								if !portals.contains(node.get_portal_cell()) {
									nodes_to_remove.push(node);
								}
							}
						}
					}
				}
				Ordinal::West => {
					for node in graph_copy.get_nodes().iter() {
						if *node.get_sector() == *neighbour_id {
							if node.get_portal_cell().get_column() == 0 {
								// self.get_graph_mut().remove(node).unwrap();
								edge_node_to_remove.push(node);
								if !portals.contains(node.get_portal_cell()) {
									nodes_to_remove.push(node);
								}
							}
						}
					}
				}
				_ => panic!("Diagonals shouldn't exist between sectors in the PortalGraph"),
			}
			for node in edge_node_to_remove {
				// remove edges that reference these nodes
				for edge in graph_copy.get_edges().iter() {
					if edge.contains_target(*node) {
						self.get_edges_mut().remove(edge);
					}
				}
				// for (_nodes, edges) in self.get_graph_mut().iter_mut() {
				// 	edges.retain(|e| e.get_node() != node);
				// }
			}
			for node in nodes_to_remove {
				// remove the old nodes
				self.get_nodes_mut().remove(node);
			}
			// add new nodes to replace the ones removed in the neighbour
			let portals_array = sector_portals.get().get(&neighbour_id).unwrap();
			let portals = portals_array.get_portals_for_side(&ord_pointing_at_changed);
			let cost_field = sector_cost_fields.get_scaled().get(neighbour_id).unwrap();
			for cell in portals {
				let weight = cost_field.get_field_cell_value(*cell) as i32;
				let new_node = PortalNode::new(*neighbour_id, *cell, weight);
				self.insert_portal_node(new_node);
			}
			//TODO see self.insert_edge
			// recreate the internal edges of the neighbour sector
			let all_portals = sector_portals.get().get(&neighbour_id).unwrap().get_all(); 
			self.build_sector_internal_edges(neighbour_id, cost_field, &all_portals);
		}
	}
	/// Remove [PortalNode]s of a mutated sector and place new nodes reflecting the updated [Portals], additioanlly create new internal edges between the new [Portals]
	fn replace_changed_sector_nodes(&mut self, changed_sector: &SectorID, sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,) {
		// remove all PortalNodes using the changed sector
		let mut graph_copy = self.clone();
		let nodes_to_remove = graph_copy.get_nodes_containg_sector_mut(&changed_sector);
		for n in nodes_to_remove {
			self.get_nodes_mut().remove(n);
		}
		let cost_field = sector_cost_fields
			.get_scaled()
			.get(&changed_sector)
			.unwrap();
		let portals_array = sector_portals.get().get(&changed_sector).unwrap();
		// rebuild the changed sectors nodes
		for p in portals_array.get() {
			for cell in p {
				let weight = cost_field.get_field_cell_value(*cell) as i32;
				let portal_node = PortalNode::new(*changed_sector, *cell, weight);
				self.insert_portal_node(portal_node);
			}
		}
		// rebuild the changed sectors internal edges
		let all_portals = portals_array.get_all();
		self.build_sector_internal_edges(&changed_sector, cost_field, &all_portals);
	}
}

impl PortalGraph {
	/// From any field cell at a `source` sector find any pathable portals witihn that sector and generate a path from each portal to the target. Compare the results and return the path with the best cost associated with it
	pub fn find_best_path(
		&self,
		source: (SectorID, FieldCell),
		target: (SectorID, FieldCell),
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
	) -> Option<Vec<(SectorID, FieldCell)>> {
		// find portals reachable by the source actor position
		let source_sector_id = source.0;
		let source_field_cell = source.1;
		let source_weight = sector_cost_fields.get_scaled().get(&source_sector_id).unwrap().get_field_cell_value(source_field_cell) as i32;
		let mut source_portals = Vec::new();
		let portals = sector_portals.get().get(&source_sector_id).unwrap();
		for ordinal in portals.get().iter() {
			for cell in ordinal.iter() {
				let cost_field = sector_cost_fields
					.get_scaled()
					.get(&source_sector_id)
					.unwrap();
				if cost_field
					.can_internal_portal_pair_see_each_other(source_field_cell, *cell)
					.0
				{
					source_portals.push(*cell);
				}
			}
		}
		// find portals that can reach the target/goal
		let target_sector_id = target.0;
		let target_field_cell = target.1;
		let target_weight = sector_cost_fields.get_scaled().get(&target_sector_id).unwrap().get_field_cell_value(target_field_cell) as i32;
		let mut target_portals = Vec::new();
		let portals = sector_portals.get().get(&target_sector_id).unwrap();
		for ordinal in portals.get().iter() {
			for cell in ordinal.iter() {
				let cost_field = sector_cost_fields
					.get_scaled()
					.get(&target_sector_id)
					.unwrap();
				if cost_field
					.can_internal_portal_pair_see_each_other(target_field_cell, *cell)
					.0
				{
					target_portals.push(*cell);
				}
			}
		}
		// iterate over the source and target portals to find a series of paths
		let mut paths = Vec::new();
		for source_portal in source_portals.iter() {
			for target_portal in target_portals.iter() {
				let source_portal_node = PortalNode::new(source_sector_id, *source_portal, source_weight);
				let target_portal_node = PortalNode::new(target_sector_id, *target_portal, target_weight);
				if let Some(path) = self.find_path_between_sector_portals(
					source_portal_node,
					target_portal_node,
				) {
					paths.push(path);
				}
			}
		}
		// find and return the best
		let mut best_cost = i32::MAX;
		let mut best_path: Option<Vec<(SectorID, FieldCell)>> = None;
		for path in paths.iter() {
			if path.0 < best_cost {
				best_cost = path.0;
				best_path = Some(path.1.clone());
			}
		}
		best_path
	}
	/// Find a path from a source [PortalNode] to a target [PortalNode] if it
	/// exists and return the path with a weighting of how expensive it is
	fn find_path_between_sector_portals(&self, source_node: PortalNode, target_node: PortalNode) -> Option<(i32, Vec<(SectorID, FieldCell)>)> {
		if let Some(path) = self.astar(source_node, target_node) {
			let total_weight = path.0;
			let mut p = Vec::new();
			// extract portal node into a <sector, field_cell> representation
			for node in path.1 {
				p.push((node.get_sector().clone(), node.get_portal_cell().clone()));
			}
			Some((total_weight, p))
		} else {
			None
		}
	}
	/// Based on a source node and direction find any edges containing those parameters
	fn find_edges(&self, source: PortalNode, direction: Direction) -> Vec<PortalEdge> {
		let mut e = vec![];
		for edge in self.get_edges() {
			if *edge.get_source_node() == source && *edge.get_direction() == direction {
				e.push(edge.clone());
			}
		}
		e
	}
	/// Based on https://github.com/BlondeBurrito/pathfinding_astar
fn astar(&self, source_node: PortalNode, target_node: PortalNode) -> Option<(i32, Vec<PortalNode>)> {
	let nodes = self.get_nodes();
	// ensure nodes data contains start and end points
	if !nodes.contains(&source_node) {
		panic!("Node data does not contain start node {:?}", source_node);
	}
	if !nodes.contains(&target_node) {
		panic!("Node data does not contain end node {:?}", target_node);
	}
	// retreive the weight of the start point
	let start_weight: i32 = source_node.get_weight();

	// Every time we process a new node we add it to a map.
	// If a node has already been recorded then we replace it if it has a better a-star score (smaller number)
	// otherwise we discard it.
	// This is used to optimise the searching whereby if we find a new path to a previously
	// processed node we can quickly decide to discard or explore the new route
	let mut node_astar_scores: HashMap<PortalNode, i32> = HashMap::new();

	// add starting node a-star score to data set (starting node score is just its weight)
	node_astar_scores.insert(source_node, start_weight);

	// we always start at a portal on the boundary of the starting sector, therefore we search for an edge with direction of external
	let edge_direction = Direction::External;

	// create a queue of nodes to be processed based on discovery
	// of form (current_node, a_star_score, vec_previous_nodes_traversed, total_distance_traversed, edge_direction_to_explore)
	// start by add starting node to queue
	let mut queue = vec![(
		source_node,
		start_weight, // we haven't moved so starting node score is just its weight
		Vec::<PortalNode>::new(),
		0,
		edge_direction
	)];

	// If a path exists then the end node will shift to the beginning of the queue and we can return it.
	// If a path does not exist the `queue` will shrink to length 0 and we return `None` through a check
	// at the end of each loop iteration.
	while queue[0].0 != target_node {
		// info!("Curr queue {:?}", queue);
		// Remove the first element ready for processing
		let current_path = queue.swap_remove(0);
		// what edge direction to explore
		let edge_direction = current_path.4;
		// Grab the neighbours with their distances from the current path so we can explore each
		let neightbours = self.find_edges(current_path.0, current_path.4);
		// Process each new path
		for n in neightbours.iter() {
			let distance_traveled_so_far: i32 = current_path.3;
			let distance_to_this_neighbour: i32 = n.get_distance();
			// Calculate the total distance from the start to this neighbour node
			let distance_traveled = distance_traveled_so_far + distance_to_this_neighbour;
			let node_weight: i32 = n.get_target_node().get_weight();
			// Now we know the overall distance traveled and the weight of where we're going to we can score it
			let astar_score = a_star_score(distance_traveled, node_weight);
			// Create a vector of the nodes traversed to get to this `n`
			let mut previous_nodes_traversed = current_path.2.clone();
			previous_nodes_traversed.push(current_path.0);
			// Update the a-star data set.
			// If it already has a record of this node we choose to either update it or ignore this new path as it is worse than what we have calculated in a previous iteration
			if node_astar_scores.contains_key(&n.get_target_node()) {
				if node_astar_scores.get(&n.get_target_node()) >= Some(&astar_score) {
					// `node_astar_scores` contains a worse score so update the map with the better score
					node_astar_scores.insert(n.get_target_node().clone(), astar_score);
					// Search the queue to see if we already have a route to this node.
					// If we do but this new path is better then replace it, otherwise discard
					let mut new_queue_item_required_for_node = true;
					for q in queue.iter_mut() {
						if q.0 == *n.get_target_node() {
							// If existing score is worse (higher) then replace the queue item and
							// don't allow a fresh queue item to be added
							if q.1 >= astar_score {
								new_queue_item_required_for_node = false;
								q.1 = astar_score;
								q.2 = previous_nodes_traversed.clone();
								q.3 = distance_traveled;
								q.4 = edge_direction.flip()
							}
						}
					}
					// Queue doesn't contain a route to this node, as we have now found a better route
					// update the queue with it so it can be explored
					if new_queue_item_required_for_node {
						queue.push((
							n.get_target_node().clone(),
							astar_score,
							previous_nodes_traversed,
							distance_traveled,
							edge_direction.flip()
						));
					}
				}
			} else {
				// No record of node therefore this is the first time it has been visted
				// Update the a-star score data
				node_astar_scores.insert(n.get_target_node().clone(), astar_score);
				// Update the queue with this new route to process later
				queue.push((
					n.get_target_node().clone(),
					astar_score,
					previous_nodes_traversed,
					distance_traveled,
					edge_direction.flip()
				));
			}
		}

		// Sort the queue by a-star sores so each loop processes the current best path
		queue.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

		// As the `queue` is processed elements are removed, neighbours discovered and scores calculated.
		//If the `queue` length becomes zero then it means there are no routes to the `end_node` and we return `None`
		if queue.len() == 0 {
			return None;
		}
	}
	let score = queue[0].1;
	let mut best_path = queue[0].2.clone();
	// add end node to data
	best_path.push(target_node);
	Some((score, best_path))
}
}

/// Determines a score to rank a chosen path, lower scores are better
fn a_star_score(distance: i32, weighting: i32) -> i32 {
	distance + weighting
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use crate::flowfields::sectors::sector_cost::SectorCostFields;

use super::*;
	#[test]
	fn portal_graph_node_count() {
		//init
		let map_dimensions = MapDimensions::new(30, 30, 10, 0.5);
		let sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get_scaled().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, &map_dimensions),
				None => panic!("Key {:?} not found in Portals", sector_id),
			}
		}
		// build the graph
		let mut graph = PortalGraph::default();
		graph.insert_all_portal_nodes(&sector_portals, &sector_cost_fields);
		let result = graph.get_nodes().len();

		let portal_count = 24; // sum of portals for each sector in the 3x3 sector grid
		let actual = portal_count;
		assert_eq!(actual, result);
	}
	#[test]
	fn portal_graph_basic_sector_edge_count() {
		//init
		let map_dimensions = MapDimensions::new(30, 30, 10, 0.5);
		let sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get_scaled().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, &map_dimensions),
				None => panic!("Key {:?} not found in Portals", sector_id),
			}
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.insert_all_portal_nodes(&sector_portals, &sector_cost_fields);
		// build the edges within each sector
		portal_graph.build_all_internal_sector_edges(&sector_portals, &sector_cost_fields);
		let result = portal_graph.get_edges().len();

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
		// each portal in a sector is connected to every other portal in that sector
		let portal_to_portal_count = 44;
		let actual = portal_to_portal_count;
		assert_eq!(actual, result);
	}
	#[test]
	fn portal_graph_basic_node_count() {
		//init
		let map_dimensions = MapDimensions::new(30, 30, 10, 0.5);
		let sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get_scaled().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, &map_dimensions),
				None => panic!("Key {:?} not found in Portals", sector_id),
			}
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.insert_all_portal_nodes(&sector_portals, &sector_cost_fields);
		// build the edges within each sector
		portal_graph.build_all_internal_sector_edges(&sector_portals, &sector_cost_fields);
		// build the edges between sectors
		portal_graph.build_all_external_sector_edges(&sector_portals, &sector_cost_fields, &map_dimensions);
		let result = portal_graph.get_edges().len();

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
		// each portal in a sector is connected to every other portal in that sector
		let portal_to_portal_count = 44;
		// each sector boundary has an edge to the neighbouring sector boundary
		let sector_to_sector_count = 24;
		let actual = portal_to_portal_count + sector_to_sector_count;
		assert_eq!(actual, result);
	}
	#[test]
	fn update_graph_from_portals_change_node_count() {
		//init
		let map_dimensions = MapDimensions::new(30, 30, 10, 0.5);
		let mut sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id,&map_dimensions)
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.insert_all_portal_nodes(&sector_portals, &sector_cost_fields);
		// build the edges within each sector
		portal_graph.build_all_internal_sector_edges(&sector_portals, &sector_cost_fields);
		// build the edges between sectors
		portal_graph.build_all_external_sector_edges(&sector_portals, &sector_cost_fields, &map_dimensions);
		// the current graph has this plain representation of portals
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

		// update the top-left CostFields and calculate new portals
		let mutated_sector_id = SectorID::new(0, 0);
		sector_cost_fields.set_field_cell_value(mutated_sector_id, 255, FieldCell::new(4, 9), &map_dimensions);
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, &map_dimensions);

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

		// update the graph
		portal_graph.update_graph(mutated_sector_id, &sector_portals, &sector_cost_fields, &map_dimensions);
		// length is the number of nodes
		let result = portal_graph.get_nodes().len();
		// each portal in a sector is connected to every other portal in that sector
		let actual_nodes = 26;
		assert_eq!(actual_nodes, result);
	}
	#[test]
	fn update_graph_from_portals_change_edge_count() {
		//init
		let map_dimensions = MapDimensions::new(30, 30, 10, 0.5);
		let mut sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id,&map_dimensions)
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.insert_all_portal_nodes(&sector_portals, &sector_cost_fields);
		// build the edges within each sector
		portal_graph.build_all_internal_sector_edges(&sector_portals, &sector_cost_fields);
		// build the edges between sectors
		portal_graph.build_all_external_sector_edges(&sector_portals, &sector_cost_fields, &map_dimensions);
		// the current graph has this plain representation of portals
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

		// update the top-left CostFields and calculate new portals
		let mutated_sector_id = SectorID::new(0, 0);
		sector_cost_fields.set_field_cell_value(mutated_sector_id, 255, FieldCell::new(4, 9), &map_dimensions);
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, &map_dimensions);

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

		// update the graph
		portal_graph.update_graph(mutated_sector_id, &sector_portals, &sector_cost_fields, &map_dimensions);
		// test that the graph has updated with the new edges
		let result = portal_graph.get_edges().len();
		println!("Graph\n {:?}", portal_graph.get_edges());
		// each portal in a sector is connected to every other portal in that sector
		let portal_to_portal_count = 54; //SHOULD BE 54
		// each sector boundary has an edge to the neighbouring sector boundary
		let sector_to_sector_count = 26;
		let actual = portal_to_portal_count + sector_to_sector_count;
		assert_eq!(actual, result);
	}
	/// Update the costfield so that portal sitting in a corner serves as a link to two sectors at the same time
	#[test]
	fn update_graph_from_portals_change_edge_count_dual_corner_portals() {
		//init
		let map_dimensions = MapDimensions::new(20, 20, 10, 0.5);
		let mut sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id,&map_dimensions)
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.insert_all_portal_nodes(&sector_portals, &sector_cost_fields);
		// build the edges within each sector
		portal_graph.build_all_internal_sector_edges(&sector_portals, &sector_cost_fields);
		// build the edges between sectors
		portal_graph.build_all_external_sector_edges(&sector_portals, &sector_cost_fields, &map_dimensions);
		// the current graph has this plain representation of portals
		// _____________________
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |____P____|____P____|
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |_________|_________|

		// update the top-left CostFields and calculate new portals
		let mutated_sector_id_0 = SectorID::new(0, 0);
		sector_cost_fields.set_field_cell_value(mutated_sector_id_0, 255, FieldCell::new(9, 8), &map_dimensions);
		sector_portals.update_portals(mutated_sector_id_0, &sector_cost_fields, &map_dimensions);
		portal_graph.update_graph(mutated_sector_id_0, &sector_portals, &sector_cost_fields, &map_dimensions);

		let mutated_sector_id_1 = SectorID::new(0, 1);
		sector_cost_fields.set_field_cell_value(mutated_sector_id_1, 255, FieldCell::new(8, 0), &map_dimensions);
		sector_portals.update_portals(mutated_sector_id_1, &sector_cost_fields, &map_dimensions);
		portal_graph.update_graph(mutated_sector_id_1, &sector_portals, &sector_cost_fields, &map_dimensions);

		// This produces a new representation with an extra portal, `x` denotes the impassable point
		// just inserted
		// _____________________
		// |         |         |
		// |         P         |
		// |         |         |
		// |         x         |
		// |___P____P>____P____|
		// |       x |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |_________|_________|

		// test for node count
		let result_nodes = portal_graph.get_nodes().len();
		let actual_nodes = 11;
		// println!("Graph {:?}", portal_graph.graph);
		// println!("Portals {:?}", sector_portals.get());
		assert_eq!(actual_nodes, result_nodes);

		// test that the graph has updated with the new edges
		let result_edges = portal_graph.get_edges().len();
		// println!("Graph\n {:?}", portal_graph);
		// each portal in a sector is connected to every other portal in that sector
		let portal_to_portal_count = 20;
		// each sector boundary has an edge to the neighbouring sector boundary
		let sector_to_sector_count = 12;
		let actual_edges = portal_to_portal_count + sector_to_sector_count;
		assert_eq!(actual_edges, result_edges);
	}

	// #[test]
	// fn dugin() {
	// 	let map_x_dimension = 30;
	// 	let map_z_dimension = 30;
	// 	let mut sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
	// 	let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
	// 	// build portals
	// 	for (id, portals) in sector_portals.get_mut().iter_mut() {
	// 		portals.recalculate_portals(&sector_cost_fields, id, map_x_dimension, map_z_dimension)
	// 	}
	// 	// update the top-left CostFields and calculate new portals
	// 	let mutated_sector_id = (0, 0);
	// 	let field = sector_cost_fields.get_mut().get_mut(&mutated_sector_id).unwrap();
	// 	field.set_field_cell_value(255, 4, 9);
	// 	sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, map_x_dimension, map_z_dimension);

	// 	// build the graph
	// 	let mut portal_graph = PortalGraph::default();
	// 	portal_graph.build_graph_nodes(&sector_portals);
	// 	assert_eq!(35, portal_graph.graph.node_count());
	// 	// build the edges within each sector
	// 	portal_graph.build_edges_within_each_sector(&sector_portals);
	// 	assert_eq!(106, portal_graph.graph.edge_count());
	// 	// build the edges between sectors
	// 	portal_graph.build_edges_between_sectors(&sector_portals, map_x_dimension, map_z_dimension);
	// 	assert_eq!(132, portal_graph.graph.edge_count());
	// }
	#[test]
	fn best_path_as_sector_portals() {
		let map_dimensions = MapDimensions::new(30, 30, 10, 0.5);
		let sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get_scaled().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, &map_dimensions),
				None => panic!("Key {:?} not found in Portals", sector_id),
			}
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.insert_all_portal_nodes(&sector_portals, &sector_cost_fields);
		// build the edges within each sector
		portal_graph.build_all_internal_sector_edges(&sector_portals, &sector_cost_fields);
		// build the edges between sectors
		portal_graph.build_all_external_sector_edges(&sector_portals, &sector_cost_fields, &map_dimensions);

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

		// form of ((sector_id), (portal_cell_id))
		let source_sector = SectorID::new(0, 0);
		let source_field = FieldCell::new(4, 9);
		let source_weight = sector_cost_fields.get_scaled().get(&source_sector).unwrap().get_field_cell_value(source_field) as i32;
		let source_portal_node = PortalNode::new(source_sector, source_field, source_weight);

		let target_sector = SectorID::new(0, 2);
		let target_field = FieldCell::new(4, 0);
		let target_weight = sector_cost_fields.get_scaled().get(&target_sector).unwrap().get_field_cell_value(target_field) as i32;
		let target_portal_node = PortalNode::new(target_sector, target_field, target_weight);

		let path = portal_graph.find_path_between_sector_portals(source_portal_node, target_portal_node).unwrap();
		let actual = vec![(SectorID::new(0, 0), FieldCell::new(4, 9)), (SectorID::new(0, 1), FieldCell::new(4, 0)), (SectorID::new(0, 1), FieldCell::new(4, 9)), (SectorID::new(0, 2), FieldCell::new(4, 0))];
		
		assert_eq!(actual, path.1);
	}






	// #[test]
	// fn best_path_xyz() {
	// 	let map_x_dimension = 30;
	// 	let map_z_dimension = 30;
	// 	let sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
	// 	let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
	// 	// build portals
	// 	for (sector_id, _cost_fields) in sector_cost_fields.get().iter() {
	// 		let portals = sector_portals.get_mut();
	// 		match portals.get_mut(sector_id) {
	// 			Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, map_x_dimension, map_z_dimension),
	// 			None => assert!(false),
	// 		}
	// 	}
		
	// 	// build the graph
	// 	let mut portal_graph = PortalGraph::default();
	// 	portal_graph.build_graph_nodes(&sector_portals);
	// 	// build the edges within each sector
	// 	portal_graph.build_edges_within_each_sector(&sector_portals);
	// 	// build the edges between sectors
	// 	portal_graph.build_edges_between_sectors(&sector_portals, map_x_dimension, map_z_dimension);

	// 	// _______________________________
	// 	// |         |         |         |
	// 	// |         |         |         |
	// 	// |         P         P         |
	// 	// |         |         |         |
	// 	// |____P____|____P____|____P____|
	// 	// |         |         |         |
	// 	// |         |         |         |
	// 	// |         P         P         |
	// 	// |         |         |         |
	// 	// |____P____|____P____|____P____|
	// 	// |         |         |         |
	// 	// |         |         |         |
	// 	// |         P         P         |
	// 	// |         |         |         |
	// 	// |_________|_________|_________|

	// 	let source_sector = (0, 0);
	// 	let target_sector = (0, 2);
	// 	let path = portal_graph.find_path(source_sector, target_sector, &sector_portals, map_x_dimension, map_z_dimension);
	// 	println!("Path {:?}", path);
	// 	match path {
	// 		Some(_) => assert!(true),
	// 		None => assert!(false)
	// 	}
	// }
	//TODO more test, must be robust
}
