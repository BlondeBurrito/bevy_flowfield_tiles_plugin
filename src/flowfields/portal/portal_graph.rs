//! When an agent needs to path somewhere it is initially given a path based on moving from one portal
//! to another portal/end sector. The path is calculated from the [PortalGraph] which records the
//! points of navigation (`nodes`), the the paths bewteen them (`edges`).
//!
//! This ensures responsiveness so when a player issues a movement order
//! the agent immediately starts pathing. In the background the other components of the Flowfields can
//! calcualte a perfect path which can then supersede using portals to path when it's ready

use crate::prelude::*;
use bevy::{
	platform::collections::{HashMap, HashSet},
	prelude::*,
};

/// Used to provide a heuristic for portals that sit next to each other across
/// a portal boundary. This is used in the a-star calculation for determining
/// the best portal path to a goal
const SECTOR_BOUNDARY_PORTAL_PORTAL_DISTANCE: i32 = 1;

/// The graph contains a series of [Node] which denotes the Sector and FieldCell of a portal
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Reflect, Debug, Clone, Copy)]
struct Node {
	/// Sector containing the node
	sector_id: SectorID,
	/// FieldCell (column, row) position of the portal
	portal_cell: FieldCell,
	/// How expensive it is the move across this portal [FieldCell] ([CostField] value)
	weight: u8,
	/// What side of the sector the [Node] sits on
	side: Ordinal,
}

impl Node {
	/// Create a new instance of [Node] for the given sector and cell with a
	/// weight and sitting along a particular side of a sector
	fn new(sector_id: SectorID, portal_cell: FieldCell, weight: u8, side: Ordinal) -> Self {
		Node {
			sector_id,
			portal_cell,
			weight,
			side,
		}
	}
	/// Get the sector the [Node] is in
	fn get_sector(&self) -> &SectorID {
		&self.sector_id
	}
	/// Get the [FieldCell] of the portal
	fn get_portal_cell(&self) -> &FieldCell {
		&self.portal_cell
	}
	/// Get the [CostField] based expense of traversing this portal
	fn get_weight(&self) -> u8 {
		self.weight
	}
	/// Get the [Ordinal] side of the sector that this [Node] sits on
	fn get_side(&self) -> &Ordinal {
		&self.side
	}
	/// Compare the [SectorID] of `self` with another `compare` to see if they're the same
	fn is_in_sector(&self, compare: &SectorID) -> bool {
		self.sector_id == *compare
	}
}

impl PartialEq for Node {
	fn eq(&self, other: &Self) -> bool {
		self.sector_id == other.sector_id
			&& self.portal_cell == other.portal_cell
			&& self.side == other.side
	}
}

impl Eq for Node {}

impl std::hash::Hash for Node {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.sector_id.hash(state);
		self.portal_cell.hash(state);
	}
}

/// Defines a passage from one portal to another
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default, Reflect, Debug, Clone)]
struct Edge {
	/// Source [Node] of this edge
	from: Node,
	/// Target [Node] of this edge
	to: Node,
	/// How expensive it is to traverse this [Edge]
	distance: i32,
}

impl Edge {
	/// Create a new [Edge] indicating that a portal `from` connects with `to`, with a weighting of `distance`
	fn new(from: Node, to: Node, distance: i32) -> Self {
		Edge { from, to, distance }
	}
	/// Get the source [Node] of this edge
	fn get_from(&self) -> &Node {
		&self.from
	}
	/// Get the target [Node] of this edge
	fn get_to(&self) -> &Node {
		&self.to
	}
	/// Get how expensive it is to traverse this [Edge]
	fn get_distance(&self) -> i32 {
		self.distance
	}
}

impl PartialEq for Edge {
	fn eq(&self, other: &Self) -> bool {
		self.from == other.from && self.to == other.to
	}
}
impl Eq for Edge {}

impl std::hash::Hash for Edge {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.from.hash(state);
		self.to.hash(state);
	}
}

/// The graph represents all the [Portals] across sectors in a [Node] notation. Each [Node] is then associated with `edges_internal` and `edges_external` which define routes to travel between [Portals].
///
/// The graph can be queried to find the best path of [Portals] from one sector to another
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Default, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct PortalGraph {
	/// Points that represent the gateway from one sector to another
	nodes: HashSet<Node>,
	/// A pair of [Node]s that indicate that a [Node] within the current sector can allow passage to another [Node] within the same sector
	edges_internal: HashSet<Edge>,
	/// A pair of [Node]s that indicate that a [Node] within the current sector can allow passage to another [Node] in a different sector
	edges_external: HashSet<Edge>,
}
// interface methods to the graph
impl PortalGraph {
	/// Get a reference to the set of [Node]s
	fn get_nodes(&self) -> &HashSet<Node> {
		&self.nodes
	}
	/// Add a [Node] to the graph
	fn add_node(&mut self, node: Node) {
		self.nodes.insert(node);
	}
	/// Remove a [Node] from the graph. This will also remove any [Edge] involving it
	fn remove_node(&mut self, node: &Node) {
		let mut edges_to_remove_int = vec![];
		for edge in &self.edges_internal {
			if edge.from == *node || edge.to == *node {
				edges_to_remove_int.push(edge.clone());
			}
		}
		let mut edges_to_remove_ext = vec![];
		for edge in &self.edges_external {
			if edge.from == *node || edge.to == *node {
				edges_to_remove_ext.push(edge.clone());
			}
		}
		for edge in edges_to_remove_int.iter() {
			self.remove_edge_internal(edge);
		}
		for edge in edges_to_remove_ext.iter() {
			self.remove_edge_external(edge);
		}
		self.nodes.remove(node);
	}
	/// Get a referecne to the internal edges of the graph
	fn get_edges_internal(&self) -> &HashSet<Edge> {
		&self.edges_internal
	}
	/// Get a referecne to the external edges of the graph
	fn get_edges_external(&self) -> &HashSet<Edge> {
		&self.edges_external
	}
	/// Add an internal [Edge] to the graph
	fn add_edge_internal(&mut self, edge: Edge) {
		self.edges_internal.insert(edge);
	}
	/// Add an external [Edge] to the graph``
	fn add_edge_external(&mut self, edge: Edge) {
		self.edges_external.insert(edge);
	}
	/// Remove an internal [Edge] from the graph
	fn remove_edge_internal(&mut self, edge: &Edge) {
		self.edges_internal.remove(edge);
	}
	/// Remove and  external [Edge] from the graph
	fn remove_edge_external(&mut self, edge: &Edge) {
		self.edges_external.remove(edge);
	}
}
// graph building related methods
impl PortalGraph {
	/// Create a new instance of [PortalGraph] from sector data
	pub fn new(
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) -> Self {
		let mut graph = PortalGraph::default();
		graph.create_all_nodes(sector_portals, sector_cost_fields);
		graph.create_all_internal_edges(sector_portals, sector_cost_fields);
		graph.create_all_external_edges(sector_portals, sector_cost_fields, map_dimensions);
		graph
	}
	/// Add nodes for all sectors to the [PortalGraph]
	fn create_all_nodes(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
	) {
		let portals_map = sector_portals.get();
		for (sector_id, portals) in portals_map {
			self.create_sector_nodes(sector_cost_fields, sector_id, portals);
		}
	}
	/// For a given `sector_id` create a [Node] for each portal
	fn create_sector_nodes(
		&mut self,
		sector_cost_fields: &SectorCostFields,
		sector_id: &SectorID,
		portals: &Portals,
	) {
		let ords = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
		for ord in ords.iter() {
			for cell in portals.get(ord).iter() {
				let weight = sector_cost_fields
					.get_scaled()
					.get(sector_id)
					.unwrap()
					.get_field_cell_value(*cell);
				let portal_node = Node::new(*sector_id, *cell, weight, *ord);
				self.add_node(portal_node);
			}
		}
	}
	/// Iterate over every sector and create [Edge]s between each [Node] within
	/// that sector
	fn create_all_internal_edges(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
	) {
		for (sector_id, portals) in sector_portals.get() {
			// get the cost field for this sector
			let cost_field = sector_cost_fields.get_scaled().get(sector_id).unwrap();
			// create edges between portals that can see each other
			self.create_sector_internal_edges(sector_id, cost_field, portals);
		}
	}
	/// For the given sector create [Edge]s between any [Portals] within it
	fn create_sector_internal_edges(
		&mut self,
		sector_id: &SectorID,
		cost_field: &CostField,
		portals: &Portals,
	) {
		// create edges between portals that can see each other
		let ords = [Ordinal::North, Ordinal::South, Ordinal::West, Ordinal::East];
		let mut cells = vec![];
		for ord in ords.iter() {
			for cell in portals.get(ord).iter() {
				cells.push((cell, ord));
			}
		}
		for (i, (source, ord_source)) in cells.iter().enumerate() {
			for (j, (target, ord_target)) in cells.iter().enumerate() {
				if i != j {
					if let Some(distance) = cost_field.get_distance_between_cells(source, target) {
						// create the edge
						let s_weight = cost_field.get_field_cell_value(**source);
						let source_node = Node::new(*sector_id, **source, s_weight, **ord_source);
						let t_weight = cost_field.get_field_cell_value(**target);
						let target_node = Node::new(*sector_id, **target, t_weight, **ord_target);
						let edge = Edge::new(source_node, target_node, distance);
						self.add_edge_internal(edge);
					}
				}
			}
		}
	}
	/// Create [PortalEdge]s at the portal crossing/boundary [FieldCell]s for each neighbouring sector
	fn create_all_external_edges(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) {
		for (sector_id, portals) in sector_portals.get() {
			// sectors bordering this one
			let sector_neighbours =
				map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(sector_id);
			self.create_sector_external_edges(
				sector_portals,
				sector_cost_fields,
				sector_id,
				portals,
				&sector_neighbours,
			);
		}
	}
	/// Create [PortalEdge]s from the `portals` of this `sector_id` to its neighbour portals
	fn create_sector_external_edges(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		sector_id: &SectorID,
		portals: &Portals,
		sector_neighbours: &[(Ordinal, SectorID)],
	) {
		for (ordinal, neighbour_id) in sector_neighbours.iter() {
			let cost_field_source = sector_cost_fields.get_scaled().get(sector_id).unwrap();
			let cost_field_target = sector_cost_fields.get_scaled().get(neighbour_id).unwrap();
			// get portals along boundary of current sector being worked on
			//? if a portal overlaps a corner we lose an edge pair as we only look at one ord
			let boundary_portals = portals.get(ordinal);
			// get inverse ordinal portals along boundary of the neighbour
			let neighbour_portals = sector_portals.get().get(neighbour_id).unwrap();
			let neighbour_boundary_portals = neighbour_portals.get(&ordinal.inverse());
			// create edges between the portals
			for (i, cell) in boundary_portals.iter().enumerate() {
				// source of the edge
				let source_weight = cost_field_source.get_field_cell_value(*cell);
				let source_node = Node::new(*sector_id, *cell, source_weight, *ordinal);
				// target of the edge
				// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
				let neighbour_portal = neighbour_boundary_portals[i];
				let target_weight = cost_field_target.get_field_cell_value(neighbour_portal);
				let target_node = Node::new(
					*neighbour_id,
					neighbour_portal,
					target_weight,
					ordinal.inverse(),
				);
				// add the dge
				let edge = Edge::new(
					source_node,
					target_node,
					SECTOR_BOUNDARY_PORTAL_PORTAL_DISTANCE,
				);
				self.add_edge_external(edge);
			}
		}
	}
}

// graph mutation
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
		let mut nodes_to_remove = vec![];
		let original_graph = self.clone();
		// affected nodes from the changed sector
		for n in original_graph.get_nodes().iter() {
			if n.is_in_sector(&changed_sector) {
				nodes_to_remove.push(n);
			}
		}
		// affected nodes along the boundary of each neighbouring sector
		for (ord, sector) in sectors_to_rebuild.iter() {
			let neighbours_boundary_ord = ord.inverse();
			for n in original_graph.get_nodes().iter() {
				if n.is_in_sector(sector) && *n.get_side() == neighbours_boundary_ord {
					nodes_to_remove.push(n);
				}
			}
		}
		// remove the affected nodes
		for n in nodes_to_remove {
			self.remove_node(n);
		}
		// create new nodes in changed sector
		let portals = sector_portals.get().get(&changed_sector).unwrap();
		self.create_sector_nodes(sector_cost_fields, &changed_sector, portals);
		// create nodes in the neighbouring sectors
		//TODO lets not rebuild all, on 3 sides of neighbours they should be exactly as they are
		for (_ord, sector) in sectors_to_rebuild.iter() {
			let portals = sector_portals.get().get(sector).unwrap();
			self.create_sector_nodes(sector_cost_fields, sector, portals);
		}
		// create internal edges within the changed sector
		let cost_field = sector_cost_fields
			.get_scaled()
			.get(&changed_sector)
			.unwrap();
		self.create_sector_internal_edges(&changed_sector, cost_field, portals);
		// recreate internal edges in the neighbouring sectors
		//TODO lets not rebuild all, on 3 sides of neighbours they should be exactly as they are
		for (_ord, sector) in sectors_to_rebuild.iter() {
			let cost_field = sector_cost_fields.get_scaled().get(sector).unwrap();
			let portals = sector_portals.get().get(sector).unwrap();
			self.create_sector_internal_edges(sector, cost_field, portals);
		}
		// create external edges from the changed sector to neighbours
		let portals = sector_portals.get().get(&changed_sector).unwrap();
		self.create_sector_external_edges(
			sector_portals,
			sector_cost_fields,
			&changed_sector,
			portals,
			&sectors_to_rebuild,
		);
		// create external edges from the neighbours to the changed sector
		for (ord, neighbour_sector) in sectors_to_rebuild.iter() {
			let portals = sector_portals.get().get(neighbour_sector).unwrap();
			let orignal_sector = vec![(ord.inverse(), changed_sector)];
			self.create_sector_external_edges(
				sector_portals,
				sector_cost_fields,
				neighbour_sector,
				portals,
				&orignal_sector,
			);
		}
		self
	}
}

/// An edge between [PortalNode]s comes in two varieties.
///
/// Internal means it's an edge to another Portal within the same sector, External means it is a Portal to a neighbouring sector Portal
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Direction {
	/// Edge within a sector
	Internal,
	/// Edge that links to a different sector
	External,
}

impl Direction {
	/// Invert the direction
	fn flip(self) -> Direction {
		if self == Direction::Internal {
			Direction::External
		} else {
			Direction::Internal
		}
	}
}

/// A candidate in A-Star pathing
#[derive(Debug)]
struct AStarQueueItem {
	/// Current node being explored
	current_node: Node,
	/// A-Star score of this node
	score: i32,
	/// List of previous nodes traversed
	node_history: Vec<Node>,
	/// Overall weight of eaching this node
	cumulative_distance: i32,
	/// Indicates whether the node is linked internally or externally
	edge_direction: Direction,
}

impl AStarQueueItem {
	/// Create a new [AStarQueueItem] for portal path exploration
	fn new(
		node: Node,
		score: i32,
		node_history: Vec<Node>,
		cumulative_distance: i32,
		edge_direction: Direction,
	) -> Self {
		AStarQueueItem {
			current_node: node,
			score,
			node_history,
			cumulative_distance,
			edge_direction,
		}
	}
}

// graph querying
impl PortalGraph {
	/// From any field cell at a `source` sector find any pathable portals witihn that sector and generate a path from each portal to the target. Compare the results and return the path with the best cost associated with it
	pub fn find_best_path(
		&self,
		source: (SectorID, FieldCell),
		target: (SectorID, FieldCell),
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
	) -> Option<Vec<(SectorID, FieldCell)>> {
		let cost_fields_scaled = sector_cost_fields.get_scaled();
		// find portals reachable by the source actor position
		let source_sector_id = source.0;
		let source_field_cell = source.1;
		let source_weight = sector_cost_fields
			.get_scaled()
			.get(&source_sector_id)
			.unwrap()
			.get_field_cell_value(source_field_cell);
		let mut source_portals = Vec::new();
		let portals = sector_portals.get().get(&source_sector_id).unwrap();
		let ords = [Ordinal::North, Ordinal::South, Ordinal::West, Ordinal::East];
		for ord in ords.iter() {
			for cell in portals.get(ord) {
				let cost_field = cost_fields_scaled.get(&source_sector_id).unwrap();
				if let Some(source_distance) =
					cost_field.get_distance_between_cells(&source_field_cell, cell)
				{
					source_portals.push((*cell, *ord, source_distance));
				}
				// if cost_field.is_cell_pair_reachable(source_field_cell, *cell) {
				// 	source_portals.push((*cell, *ord));
				// }
			}
		}
		// find portals that can reach the target/goal
		let target_sector_id = target.0;
		let target_field_cell = target.1;
		let target_weight = cost_fields_scaled
			.get(&target_sector_id)
			.unwrap()
			.get_field_cell_value(target_field_cell);
		let mut target_portals = Vec::new();
		let portals = sector_portals.get().get(&target_sector_id).unwrap();
		let ords = [Ordinal::North, Ordinal::South, Ordinal::West, Ordinal::East];
		for ord in ords.iter() {
			for cell in portals.get(ord) {
				let cost_field = cost_fields_scaled.get(&target_sector_id).unwrap();
				if cost_field.is_cell_pair_reachable(target_field_cell, *cell) {
					target_portals.push((*cell, *ord));
				}
			}
		}
		// iterate over the source and target portals to find a series of paths
		let mut best_path: Option<(i32, Vec<(SectorID, FieldCell)>)> = None;
		// if local sector add a cheaper direct route, prevents pathing out of a sector and back in when there are extreme local costs
		//TODO maybe skip searching for other paths if this is true? improve perf but would a really bad local route be given (maybe only under extreme circumstances tho)
		if source_sector_id == target_sector_id {
			if let Some(cost) = cost_fields_scaled
				.get(&source_sector_id)
				.unwrap()
				.get_distance_between_cells(&source_field_cell, &target_field_cell)
			{
				best_path = Some((cost, vec![(target_sector_id, target_field_cell)]));
			}
		}
		for (source_portal, source_ordinal, source_distance) in source_portals.iter() {
			for (target_portal, target_ordinal) in target_portals.iter() {
				let source_portal_node = Node::new(
					source_sector_id,
					*source_portal,
					source_weight,
					*source_ordinal,
				);
				let target_portal_node = Node::new(
					target_sector_id,
					*target_portal,
					target_weight,
					*target_ordinal,
				);
				self.find_path_between_sector_portals(
					&mut best_path,
					source_portal_node,
					target_portal_node,
					*source_distance,
				);
			}
		}
		if let Some((_score, p)) = best_path {
			Some(p)
		} else {
			None
		}
	}
	/// Find a path from a source [Node] to a target [Node] if it
	/// exists and return the path with a weighting of how expensive it is
	fn find_path_between_sector_portals(
		&self,
		best_path: &mut Option<(i32, Vec<(SectorID, FieldCell)>)>,
		source_node: Node,
		target_node: Node,
		source_distance: i32,
	) {
		let current_best_score = if let Some((score, _)) = best_path {
			Some(*score)
		} else {
			None
		};
		if let Some(path) = self.astar(
			current_best_score,
			source_node,
			target_node,
			source_distance,
		) {
			let total_weight = path.0;
			let mut p = Vec::new();
			// extract portal node into a <sector, field_cell> representation
			for node in path.1 {
				p.push((*node.get_sector(), *node.get_portal_cell()));
			}
			if let Some((score, curr_path)) = best_path {
				if *score > total_weight {
					*score = total_weight;
					*curr_path = p;
				}
			} else {
				*best_path = Some((total_weight, p));
			}
		}
	}
	/// From a given [Node] find any edges within the same sector
	fn find_edges_internal(&self, source: Node) -> Vec<&Edge> {
		let mut edges = vec![];
		for edge in self.get_edges_internal().iter() {
			if *edge.get_from().get_sector() == *source.get_sector()
				&& *edge.get_to().get_sector() == *source.get_sector()
				&& *edge.get_from().get_portal_cell() == *source.get_portal_cell()
			{
				edges.push(edge);
			}
		}
		edges
	}
	/// From a given [Node] find any edges that lead to a neighbouring sector
	fn find_edges_external(&self, source: Node) -> Vec<&Edge> {
		let mut edges = vec![];
		for edge in self.get_edges_external().iter() {
			if *edge.get_from() == source && *edge.get_to().get_sector() != *source.get_sector() {
				edges.push(edge);
			}
		}
		edges
	}
	/// Based on https://github.com/BlondeBurrito/pathfinding_astar
	fn astar(
		&self,
		current_best_score: Option<i32>,
		source_node: Node,
		target_node: Node,
		source_distance: i32,
	) -> Option<(i32, Vec<Node>)> {
		let nodes = self.get_nodes();
		// ensure nodes data contains start and end points
		if !nodes.contains(&source_node) {
			error!("Node data does not contain start node {:?}, this is probably a bug, please report it", source_node);
			// panic!("Node data does not contain start node {:?}", source_node);
			return None;
		}
		if !nodes.contains(&target_node) {
			error!("Node data does not contain end node {:?}, this is probably a bug, please report it", target_node);
			// panic!("Node data does not contain end node {:?}", target_node);
			return None;
		}
		// retreive the weight of the start point
		let start_weight: i32 = source_node.get_weight() as i32;

		// Every time we process a new node we add it to a map.
		// If a node has already been recorded then we replace it if it has a better a-star score (smaller number)
		// otherwise we discard it.
		// This is used to optimise the searching whereby if we find a new path to a previously
		// processed node we can quickly decide to discard or explore the new route
		let mut node_astar_scores: HashMap<Node, i32> = HashMap::new();

		// add starting node a-star score to data set (starting node score is just its weight)
		node_astar_scores.insert(source_node, start_weight);

		// we always start at a portal on the boundary of the starting sector, therefore we search for an edge with direction of external
		let initial_edge_direction = Direction::External;

		// create a queue of nodes to be processed based on discovery
		// of form (current_node, a_star_score, vec_previous_nodes_traversed, total_distance_traversed, edge_direction_to_explore)
		// start by add starting node to queue
		let mut queue = vec![AStarQueueItem::new(
			source_node,
			start_weight,
			Vec::<Node>::new(),
			source_distance,
			initial_edge_direction,
		)];

		// If a path exists then the end node will shift to the beginning of the queue and we can return it.
		// If a path does not exist the `queue` will shrink to length 0 and we return `None` through a check
		// at the end of each loop iteration.
		while queue[0].current_node != target_node {
			// info!("Curr queue {:?}", queue);
			// Remove the first element ready for processing
			let current_path = queue.swap_remove(0);
			// short circuit, if the path being explored is already more expensive than what has been discovered already then return early instead of wasting time exploring other paths
			if let Some(curr_score) = current_best_score {
				if curr_score < current_path.score {
					return None;
				}
			}
			// what edge direction to explore
			let edge_direction = current_path.edge_direction;
			// Grab the neighbours with their distances from the current path so we can explore each
			let neighbours = match edge_direction {
				Direction::Internal => self.find_edges_internal(current_path.current_node),
				Direction::External => self.find_edges_external(current_path.current_node),
			};
			// Process each new path
			for n in neighbours.iter() {
				let distance_traveled_so_far: i32 = current_path.cumulative_distance;
				let distance_to_this_neighbour: i32 = n.get_distance();
				// Calculate the total distance from the start to this neighbour node
				let distance_traveled = distance_traveled_so_far + distance_to_this_neighbour;
				let node_weight: i32 = n.get_to().get_weight() as i32;
				// Now we know the overall distance traveled and the weight of where we're going to we can score it
				let astar_score = distance_traveled + node_weight;
				// Create a vec of the nodes traversed to get to this `n`
				let mut previous_nodes_traversed = current_path.node_history.clone();
				previous_nodes_traversed.push(current_path.current_node);
				// Update the a-star data set.
				// If it already has a record of this node we choose to either update it or ignore this new path as it is worse than what we have calculated in a previous iteration
				if node_astar_scores.contains_key(n.get_to()) {
					if node_astar_scores.get(n.get_to()) > Some(&astar_score) {
						// `node_astar_scores` contains a worse score so update the map with the better score
						node_astar_scores.insert(*n.get_to(), astar_score);
						// Search the queue to see if we already have a route to this node.
						// If we do but this new path is better then replace it, otherwise discard
						let mut new_queue_item_required_for_node = true;
						for q in queue.iter_mut() {
							if q.current_node == *n.get_to() {
								// If existing score is worse (higher) then replace the queue item and
								// don't allow a fresh queue item to be added
								if q.score >= astar_score {
									new_queue_item_required_for_node = false;
									q.score = astar_score;
									q.node_history.clone_from(&previous_nodes_traversed);
									q.cumulative_distance = distance_traveled;
									q.edge_direction = edge_direction.flip();
								}
							}
						}
						// Queue doesn't contain a route to this node, as we have now found a better route
						// update the queue with it so it can be explored
						if new_queue_item_required_for_node {
							queue.push(AStarQueueItem::new(
								*n.get_to(),
								astar_score,
								previous_nodes_traversed,
								distance_traveled,
								edge_direction.flip(),
							));
						}
					}
				} else {
					// No record of node therefore this is the first time it has been visted
					// Update the a-star score data
					node_astar_scores.insert(*n.get_to(), astar_score);
					// Update the queue with this new route to process later
					queue.push(AStarQueueItem::new(
						*n.get_to(),
						astar_score,
						previous_nodes_traversed,
						distance_traveled,
						edge_direction.flip(),
					));
				}
			}

			// Sort the queue by a-star sores so each loop processes the current best path
			queue.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
			// As the `queue` is processed elements are removed, neighbours discovered and scores calculated.
			//If the `queue` length becomes zero then it means there are no routes to the `end_node` and we return `None`
			if queue.is_empty() {
				return None;
			}
		}
		// if a path has been discovered then return it otherwise None
		// queue has arrived at the target node, we're done
		let score = queue[0].score;
		let mut best_path = queue[0].node_history.clone();
		// add end node to data
		best_path.push(target_node);
		Some((score, best_path))
	}
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use crate::flowfields::sectors::sector_cost::SectorCostFields;

use super::*;
	// useful reference diagram for 3x3 sectors
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
	#[test]
	fn node_count_default() {
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
		graph.create_all_nodes(&sector_portals, &sector_cost_fields);
		let result = graph.get_nodes().len();

		let actual = 24; // sum of portals for each sector in the 3x3 sector grid
		assert_eq!(actual, result);
	}
	#[test]
	fn edge_count_internal() {
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
		graph.create_all_nodes(&sector_portals, &sector_cost_fields);
		graph.create_all_internal_edges(&sector_portals, &sector_cost_fields);
		let result = graph.get_edges_internal().len();

		let actual = 44; // sum of internal edges across all sectors
		assert_eq!(actual, result);
	}
	#[test]
	fn edge_count_external() {
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
		let graph = PortalGraph::new(&sector_portals, &sector_cost_fields, &map_dimensions);
		let result = graph.get_edges_external().len();

		let actual = 24;
		assert_eq!(actual, result);
	}
	#[test]
	fn edge_count_default() {
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
		let graph = PortalGraph::new(&sector_portals, &sector_cost_fields, &map_dimensions);
		
		let result_internal = graph.get_edges_internal().len();
		let internal = 44; // sum of internal edges for each sector
		assert_eq!(internal, result_internal);
		let result_external = graph.get_edges_external().len();
		let external = 24; // sum of external edges for each sector
		assert_eq!(external, result_external);
	}
	// useful reference diagram for 2x2 sectors
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
	#[test]
	fn node_count_mutation() {
		//init
		let map_dimensions = MapDimensions::new(20, 20, 10, 0.5);
		let mut sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		for (sector_id, _cost_fields) in sector_cost_fields.get_scaled().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, &map_dimensions),
				None => panic!("Key {:?} not found in Portals", sector_id),
			}
		}
		let mut graph = PortalGraph::new(&sector_portals, &sector_cost_fields, &map_dimensions);
		// update the costfield to add an impassable field cell
		let mutated_sector_id = SectorID::new(0, 0);
		let mutated_field_cell =FieldCell::new(4, 9);
		let value = 255;
		sector_cost_fields.set_field_cell_value(mutated_sector_id, value, mutated_field_cell, &map_dimensions);
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, &map_dimensions);
		// update the graph
		println!("graph before {:?}", graph);
		graph.update_graph(mutated_sector_id, &sector_portals, &sector_cost_fields, &map_dimensions);
		// it should now have portals like this
		// _____________________
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |_p__x_p__|____P____|
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |_________|_________|
		let result = graph.get_nodes().len();
		let actual = 10;
		println!("graph {:?}", graph);
		assert_eq!(actual, result);
	}
	#[test]
	fn edge_count_mutation() {
		//init
		let map_dimensions = MapDimensions::new(20, 20, 10, 0.5);
		let mut sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		for (sector_id, _cost_fields) in sector_cost_fields.get_scaled().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, &map_dimensions),
				None => panic!("Key {:?} not found in Portals", sector_id),
			}
		}
		let mut graph = PortalGraph::new(&sector_portals, &sector_cost_fields, &map_dimensions);
		// update the costfield to add an impassable field cell
		let mutated_sector_id = SectorID::new(0, 0);
		let mutated_field_cell =FieldCell::new(4, 9);
		let value = 255;
		sector_cost_fields.set_field_cell_value(mutated_sector_id, value, mutated_field_cell, &map_dimensions);
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, &map_dimensions);
		// update the graph
		graph.update_graph(mutated_sector_id, &sector_portals, &sector_cost_fields, &map_dimensions);
		// it should now have portals like this
		// _____________________
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |_p__x_p__|____P____|
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |_________|_________|
		let result_internal = graph.get_edges_internal().len();
		let internal = 16;
		assert_eq!(internal, result_internal);
		let result_external = graph.get_edges_external().len();
		let external = 10;
		assert_eq!(external, result_external);
	}
	#[test]
	fn multi_mutation() {
		//init
		let map_dimensions = MapDimensions::new(20, 20, 10, 0.5);
		let mut sector_cost_fields = SectorCostFields::new(&map_dimensions);
		let mut sector_portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
		for (sector_id, _cost_fields) in sector_cost_fields.get_scaled().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, &map_dimensions),
				None => panic!("Key {:?} not found in Portals", sector_id),
			}
		}
		let mut graph = PortalGraph::new(&sector_portals, &sector_cost_fields, &map_dimensions);
		// update the costfield to add an impassable field cell
		let mutated_sector_id = SectorID::new(0, 0);
		let mutated_field_cell =FieldCell::new(8, 9);
		let value = 255;
		sector_cost_fields.set_field_cell_value(mutated_sector_id, value, mutated_field_cell, &map_dimensions);
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, &map_dimensions);
		// update the graph
		graph.update_graph(mutated_sector_id, &sector_portals, &sector_cost_fields, &map_dimensions);
		// it should now have portals like this
		// _____________________
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |___p___xp|____P____|
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |_________|_________|
		// update the costfield to add an impassable field cell
		let mutated_sector_id = SectorID::new(1, 0);
		let mutated_field_cell =FieldCell::new(0, 8);
		let value = 255;
		sector_cost_fields.set_field_cell_value(mutated_sector_id, value, mutated_field_cell, &map_dimensions);
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, &map_dimensions);
		// update the graph
		graph.update_graph(mutated_sector_id, &sector_portals, &sector_cost_fields, &map_dimensions);
		// it should now have portals like this
		// _____________________
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |x        |
		// |___p___xp<____P____|
		// |         |         |
		// |         |         |
		// |         P         |
		// |         |         |
		// |_________|_________|
		let result_nodes = graph.get_nodes().len();
		let actual_nodes = 12;
		println!("nodes actual {}, result {}", actual_nodes, result_nodes);
		assert_eq!(actual_nodes, result_nodes);
		let result_internal = graph.get_edges_internal().len();
		let actual_edges_internal = 26;
		println!("edges_internal actual {},, result {}", actual_edges_internal, result_internal);
		assert_eq!(actual_edges_internal, result_internal);
		let result_external = graph.get_edges_external().len();
		let actual_edges_external = 12;
		println!("edges_external actual {}, result {}", actual_edges_external, result_external);
		println!("edges ext {:?}", graph.get_edges_external());
		assert_eq!(actual_edges_external, result_external);
	}
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
		let graph = PortalGraph::new(&sector_portals, &sector_cost_fields, &map_dimensions);

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
		let source_weight = sector_cost_fields.get_scaled().get(&source_sector).unwrap().get_field_cell_value(source_field);
		let source_portal_node = Node::new(source_sector, source_field, source_weight, Ordinal::South) ;

		let target_sector = SectorID::new(0, 2);
		let target_field = FieldCell::new(4, 0);
		let target_weight = sector_cost_fields.get_scaled().get(&target_sector).unwrap().get_field_cell_value(target_field);
		let target_portal_node = Node::new(target_sector, target_field, target_weight, Ordinal::North);

		let mut best_path: Option<(i32, Vec<(SectorID, FieldCell)>)> = None;
		graph.find_path_between_sector_portals(&mut best_path, source_portal_node, target_portal_node, 0);
		let actual = vec![(SectorID::new(0, 0), FieldCell::new(4, 9)), (SectorID::new(0, 1), FieldCell::new(4, 0)), (SectorID::new(0, 1), FieldCell::new(4, 9)), (SectorID::new(0, 2), FieldCell::new(4, 0))];
		
		assert_eq!(actual, best_path.unwrap().1);
	}
}
