//! When an agent needs to path somewhere it is initially given a path based on moving from one portal
//! to another portal/end sector. The path is calculated from the [PortalGraph] which records the
//! points of navigation (`nodes`), the the paths bewteen them (`edges`).
//!
//! This ensures responsiveness so when a player issues a movement order
//! the agent immediately starts pathing. In the background the other components of the Flowfields can
//! calcualte a perfect path which can then supersede using portals to path when it's ready

use std::{collections::BTreeMap, sync::{Arc, Mutex}};

use bevy::prelude::*;
use petgraph::{
	algo::astar,
	stable_graph::{NodeIndex, StableGraph},
};

use crate::prelude::*;

use super::portals::Portals;

/// Each sector contains a series of Portals. A [StableGraph] allows a route to be calculated from one
/// sector to another via the portal boundaries.
///
/// To enable responsiveness in moving actors around a world they should initially be given a
/// route to navigate based upon moving between the Sectors of the world, later on once a
/// [FlowField] has been generated they can be given a more accurate route to follow.
/// [PortalGraph] enables a navigational route to be found between sectors using the [Portals]
/// of sector boundaries to provide the responsiveness of "movement asked for, begin going in
/// this direction, get a better route later"
//TODO #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))] https://github.com/petgraph/petgraph/pull/550
#[derive(Component, Default, Clone)]
pub struct PortalGraph {
	/// The graph for storing nodes and edges
	graph: StableGraph<u32, i32, petgraph::Directed>,
	/// The `graph` cannot store portal [FieldCell]s directly instead it
	/// records entries with a [NodeIndex]. Based on the vectors of each sector
	/// in [SectorPortals] we create a means of storing [NodeIndex] in an
	/// identical structure to [Portals]. This allows a portal [FieldCell] to be
	/// mapped to a [NodeIndex] and vice versa.
	///
	/// The keys of the map correspond to the unique IDs of Sectors and the values are an [Ordinal] list structure which should be identical in structure for each sectors [Portals]
	node_index_translation: BTreeMap<SectorID, [Vec<NodeIndex>; 4]>,
}
//TODO need a means of chekcing graph capacity, if it's near usize, usize then rebuild it from scrtach to reset size
impl PortalGraph {
	/// Create a new instance of [PortalGraph] with inital nodes and edges built
	pub fn new(
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) -> Self {
		let mut portal_graph = PortalGraph::default();
		portal_graph.build_graph_nodes(sector_portals);
		portal_graph.build_edges_within_each_sector(sector_portals, sector_cost_fields);
		portal_graph.build_edges_between_sectors(sector_portals, map_dimensions);
		portal_graph
	}
	/// Builds the [StableGraph] nodes for each portal within a sector
	fn build_graph_nodes(&mut self, sector_portals: &SectorPortals) -> &mut Self {
		for (sector_id, portals) in sector_portals.get().iter() {
			self.build_sector_nodes(sector_id, portals);
		}
		self
	}
	/// For the given Sector [Portals] add a node to the graph for each portal [FieldCell]
	fn build_sector_nodes(&mut self, sector_id: &SectorID, portals: &Portals) -> &mut Self {
		let graph = &mut self.graph;
		let translator = &mut self.node_index_translation;
		// initialise the sector within the translator
		translator.insert(*sector_id, [vec![], vec![], vec![], vec![]]);
		// iterate over the array of portals where each element corresponds to a particular [Ordinal],
		// we use enumeration here to represent each [Ordinal] of the sector edges, the Northern,
		// Eastern, Southern and Western, as enumeration index 0..3
		for (ordinal_index, portal_node_list) in portals.get().iter().enumerate() {
			for _portal_node in portal_node_list.iter() {
				// add a node representing the portal being processed
				let node_index = graph.add_node(1);
				// update the translator with this [NodeIndex] to enable mapping a value in a
				// sector of [Portals] to a node in the graph
				match translator.get_mut(sector_id) {
					Some(ordinal_array) => {
						ordinal_array[ordinal_index].push(node_index);
					}
					None => panic!("Translator doesn't contain sector {:?}", sector_id),
				}
			}
		}
		self
	}
	/// Builds the edges between each portal within every sector
	fn build_edges_within_each_sector(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
	) -> &mut Self {
		// for each sector create edges
		for (sector_id, portals) in sector_portals.get().iter() {
			self.build_internal_sector_edges(
				sector_id,
				portals,
				sector_cost_fields.get_arc_scaled_sector(sector_id),
			);
		}
		self
	}

	/// Create graph edges between each portal of the Sector
	fn build_internal_sector_edges(
		&mut self,
		sector_id: &SectorID,
		portals: &Portals,
		cost_field: Arc<CostField>,
	) -> &mut Self {
		// create a combined list of portal points which can be iterated over to link a portal
		// to all portals in the sector
		let all_sector_portals = Arc::new(portals
			.get()
			.iter()
			.flatten()
			.cloned()
			.collect::<Vec<FieldCell>>());
		// create pairings of portals that can reach each other
		let visible_pairs_with_cost = Arc::new(Mutex::new(Vec::new()));
		let mut handles = vec![];
		let all_sector_portals = Arc::clone(&all_sector_portals);
		for (i, source) in all_sector_portals.iter().enumerate() {
			for (j, target) in all_sector_portals.iter().enumerate() {
				if i == j {
					continue;
				} else {
					let source = source.clone();
					let target = target.clone();
					let cost_field = Arc::clone(&cost_field);
					let visible_pairs_with_cost = Arc::clone(&visible_pairs_with_cost);
					let handle = std::thread::spawn(move || {
						let is_visible =
						cost_field.can_internal_portal_pair_see_each_other_arc(source, target);
						if is_visible.0 {
							let mut locked_list = visible_pairs_with_cost.lock().unwrap();
							locked_list.push(((source, target), is_visible.1));
						}
					});
					handles.push(handle);
					// let is_visible =
					// 	cost_field.can_internal_portal_pair_see_each_other(*source, *target);
					// if is_visible.0 {
					// 	visible_pairs_with_cost.push(((source, target), is_visible.1));
					// }
				}
			}
		}
		for h in handles {
			h.join().unwrap();
		}
		let graph = &mut self.graph;
		let translator = &mut self.node_index_translation;
		// convert the field cell positions to [NodeIndex]
		let mut node_indices_to_edge = Vec::new();
		let locked_list = visible_pairs_with_cost.lock().unwrap();
		for (pair, cost) in locked_list.iter() {
			let source_index = find_index_from_single_portal_and_portal_cell(
				translator, portals, sector_id, &pair.0,
			)
			.unwrap();
			let target_index = find_index_from_single_portal_and_portal_cell(
				translator, portals, sector_id, &pair.1,
			)
			.unwrap();
			node_indices_to_edge.push(((source_index, target_index), *cost));
		}

		for (index_pair, cost) in node_indices_to_edge.iter() {
			graph.update_edge(index_pair.0, index_pair.1, *cost);
		}
		self
	}

	/// Builds the edges from each sector boundary to another
	fn build_edges_between_sectors(
		&mut self,
		sector_portals: &SectorPortals,
		map_dimensions: &MapDimensions,
	) -> &mut Self {
		for (sector_id, _portals) in sector_portals.get().iter() {
			self.build_external_sector_edges(sector_id, map_dimensions);
		}
		self
	}
	/// Create edges along the boundary of the chosen Sector portal [FieldCell]s to its neighbouring
	/// sector boundary portal [FieldCell]s
	fn build_external_sector_edges(
		&mut self,
		sector_id: &SectorID,
		map_dimensions: &MapDimensions,
	) -> &mut Self {
		let graph = &mut self.graph;
		let translator = &mut self.node_index_translation;
		let sector_neighbours =
			map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(sector_id);
		for (ordinal, neighbour_id) in sector_neighbours.iter() {
			match ordinal {
				Ordinal::North => {
					// use the northern boundary of this sector to connect portals to the southern
					// boundary of the neighbour
					// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
					let this_sector_portals = &translator.get(sector_id).unwrap()[0];
					let neighbour_portals = &translator.get(neighbour_id).unwrap()[2];
					for (i, portal_index) in this_sector_portals.iter().enumerate() {
						if let Some(neighbour) = neighbour_portals.get(i) {
							if graph.contains_node(*portal_index) && graph.contains_node(*neighbour)
							{
								graph.update_edge(*portal_index, *neighbour, 1);
							}
						}
					}
				}
				Ordinal::East => {
					// use the eastern boundary of this sector to connect portals to the western
					// boundary of the neighbour
					// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
					let this_sector_portals = &translator.get(sector_id).unwrap()[1];
					let neighbour_portals = &translator.get(neighbour_id).unwrap()[3];
					for (i, portal_index) in this_sector_portals.iter().enumerate() {
						if let Some(neighbour) = neighbour_portals.get(i) {
							if graph.contains_node(*portal_index) && graph.contains_node(*neighbour)
							{
								graph.update_edge(*portal_index, *neighbour, 1);
							}
						}
					}
				}
				Ordinal::South => {
					// use the southern boundary of this sector to connect portals to the northern
					// boundary of the neighbour
					// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
					let this_sector_portals = &translator.get(sector_id).unwrap()[2];
					let neighbour_portals = &translator.get(neighbour_id).unwrap()[0];
					for (i, portal_index) in this_sector_portals.iter().enumerate() {
						if let Some(neighbour) = neighbour_portals.get(i) {
							if graph.contains_node(*portal_index) && graph.contains_node(*neighbour)
							{
								graph.update_edge(*portal_index, *neighbour, 1);
							}
						}
					}
				}
				Ordinal::West => {
					// use the western boundary of this sector to connect portals to the eastern
					// boundary of the neighbour
					// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
					let this_sector_portals = &translator.get(sector_id).unwrap()[3];
					let neighbour_portals = &translator.get(neighbour_id).unwrap()[1];
					for (i, portal_index) in this_sector_portals.iter().enumerate() {
						if let Some(neighbour) = neighbour_portals.get(i) {
							if graph.contains_node(*portal_index) && graph.contains_node(*neighbour)
							{
								graph.update_edge(*portal_index, *neighbour, 1);
							}
						}
					}
				}
				_ => panic!("Cannot create diagonals between sectors"),
			}
		}
		self
	}
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
		let mut sectors_to_rebuild =
			map_dimensions.get_ids_of_neighbouring_sectors(&changed_sector);
		let graph = &mut self.graph;
		let translator = &mut self.node_index_translation;
		// remove the nodes from the sector and its neighbours
		sectors_to_rebuild.push(changed_sector);
		for sector_id in sectors_to_rebuild.iter() {
			// lookup the [NodeIndex]s of each sector
			let ordinal_node_indices = translator
				.get(sector_id)
				.expect("PortalGraph is missing a unique sector ID");
			// iterate over each node in each ordinal and remove them from the graph
			for ordinal in ordinal_node_indices.iter() {
				for node_index in ordinal.iter() {
					let removed = graph.remove_node(*node_index);
					if removed.is_none() {
						panic!("[PortalGraph] `node_index_translation` is not syncronised with the `graph`. Attempted to remove a portal within sector {:?}", sector_id);
					}
				}
			}
		}
		// rebuild the nodes and  rebuild the edges within each sector
		for sector_id in sectors_to_rebuild.iter() {
			let cost_field = sector_cost_fields.get_arc_scaled_sector(sector_id);
			let portals = sector_portals
				.get()
				.get(sector_id)
				.expect("SectorPortals is missing a sector ID");
			self.build_sector_nodes(sector_id, portals);
			self.build_internal_sector_edges(sector_id, portals, cost_field);
		}
		// rebuild the edges between each sector
		for sector_id in sectors_to_rebuild.iter() {
			self.build_external_sector_edges(sector_id, map_dimensions);
		}
		// Note the adjacent sectors that were completely rebuilt have destroyed !their!
		// neighbours connectivity. Rebuild them too
		sectors_to_rebuild.pop(); // don't need to rebuild the one that had its cost fields changed
		for sector_id in sectors_to_rebuild.iter() {
			let neighbours_neighbour = map_dimensions.get_ids_of_neighbouring_sectors(sector_id);
			for id in neighbours_neighbour.iter() {
				self.build_external_sector_edges(id, map_dimensions);
			}
		}
		self
	}
	/// Replaces the current graph with a fresh one
	pub fn reset_graph(
		&mut self,
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) -> &mut Self {
		let mut graph = PortalGraph::default();
		graph
			.build_graph_nodes(sector_portals)
			.build_edges_within_each_sector(sector_portals, sector_cost_fields)
			.build_edges_between_sectors(sector_portals, map_dimensions);
		self.graph = graph.graph;
		self
	}
	/// From the [NodeIndex] of a starting portal attempt to find a path to a
	/// target portal [NodeIndex]. If successful the total cost of the path
	/// and a list of portal [NodeIndex]s making up the path is returned
	fn find_path_of_portals(
		&self,
		source: NodeIndex,
		target: NodeIndex,
		estimate_cost: i32,
	) -> Option<(i32, Vec<NodeIndex>)> {
		astar(
			&self.graph,
			source,
			|fin| fin == target,
			|e| *e.weight(),
			|_| estimate_cost,
		)
	}
	pub fn find_path_between_sector_portals(
		&self,
		source: (SectorID, FieldCell),
		target: (SectorID, FieldCell),
		sector_portals: &SectorPortals,
	) -> Option<(i32, Vec<NodeIndex>)> {
		if let Some(source_index) = self.find_index_from_sector_portals_and_portal_cell(
			sector_portals,
			&source.0,
			&source.1,
		) {
			if let Some(target_index) = self.find_index_from_sector_portals_and_portal_cell(
				sector_portals,
				&target.0,
				&target.1,
			) {
				let estimate_cost = {
					(target.0.get_column() as i32 - source.0.get_column() as i32).pow(2)
						+ (target.0.get_row() as i32 - source.0.get_row() as i32).pow(2)
				};
				return self.find_path_of_portals(source_index, target_index, estimate_cost);
			}
		}
		None
	}
	/// Using a path of portal [NodeIndex]s convert them into a list of sector portal pairings. Note that the first element contains the starting sector with the portal to leave and enter a different sector, the last element contains the goal sector and goal portal cell, and all the other elements are in duos whereby defining the entry and exit points of sectors along the way
	pub fn convert_index_path_to_sector_portal_cells(
		&self,
		portal_path: Vec<NodeIndex>,
		sector_portals: &SectorPortals,
	) -> Vec<(SectorID, FieldCell)> {
		let mut path = Vec::new();
		for node in portal_path.iter() {
			let (sector_id, cell_id) = self
				.find_portal_sector_id_and_cell_position_from_graph_index(sector_portals, node)
				.unwrap();
			path.push((sector_id, cell_id));
		}
		path
	}
	/// From any field cell at a `source` sector find any pathable portals witihn that sector and generate a path from each portal to the target. Compare the results and return the path with the best cost associated with it
	pub fn find_best_path(
		&self,
		source: (SectorID, FieldCell),
		target: (SectorID, FieldCell),
		sector_portals: &SectorPortals,
		sector_cost_fields: &SectorCostFields,
	) -> Option<(i32, Vec<NodeIndex>)> {
		// find portals reachable by the source actor position
		let source_sector_id = source.0;
		let source_field_cell = source.1;
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
		// find multiple paths from every source to target
		let mut paths = Vec::new();
		for source_portal in source_portals.iter() {
			for target_portal in target_portals.iter() {
				if let Some(path) = self.find_path_between_sector_portals(
					(source_sector_id, *source_portal),
					(target_sector_id, *target_portal),
					sector_portals,
				) {
					paths.push(path);
				}
			}
		}
		// find and return the best
		let mut best_cost = i32::MAX;
		let mut best_path: Option<(i32, Vec<NodeIndex>)> = None;
		for path in paths.iter() {
			if path.0 < best_cost {
				best_cost = path.0;
				best_path = Some(path.clone());
			}
		}
		best_path
	}
	// /// Iterate over the "translator" (`self.node_index_translation`) and search for a portal's `search_index`
	// /// ([NodeIndex]), if found return the `sector_id` it is located in
	// fn find_sector_id_from_graph_index(&self, search_index: &NodeIndex) -> Option<SectorID> {
	// 	let translator = &self.node_index_translation;
	// 	for (sector_id, node_ordinals) in translator.iter() {
	// 		for nodes in node_ordinals.iter() {
	// 			for n in nodes.iter() {
	// 				if *search_index == *n {
	// 					return Some(*sector_id);
	// 				}
	// 			}
	// 		}
	// 	}
	// 	None
	// }
	/// Iterate over the "translator" (`self.node_index_translation`) and search for a field cell of
	/// `search_index` and identify the sector it resides in and the cell
	/// position of it within that sector
	fn find_portal_sector_id_and_cell_position_from_graph_index(
		&self,
		sector_portals: &SectorPortals,
		search_index: &NodeIndex,
	) -> Option<(SectorID, FieldCell)> {
		let translator = &self.node_index_translation;
		for (sector_id, node_ordinals) in translator.iter() {
			for (i, nodes) in node_ordinals.iter().enumerate() {
				for (j, n) in nodes.iter().enumerate() {
					if *search_index == *n {
						let cell = sector_portals.get().get(sector_id).unwrap().get()[i][j];
						return Some((*sector_id, cell));
					}
				}
			}
		}
		None
	}
	/// Iterate through the [SectorPortals] using the `sector_id` and `portal_cell` to identify the lookup indices to find the portals [NodeIndex] from its recorded position in the "translator" (`self.node_index_translation`)
	fn find_index_from_sector_portals_and_portal_cell(
		&self,
		sector_portals: &SectorPortals,
		sector_id: &SectorID,
		portal_cell: &FieldCell,
	) -> Option<NodeIndex> {
		let translator = &self.node_index_translation;
		// locate the indices within sector_portals which can be used to access the
		// right elements of the translator
		for (i, ordinals) in sector_portals
			.get()
			.get(sector_id)
			.unwrap()
			.get()
			.iter()
			.enumerate()
		{
			for (j, portal) in ordinals.iter().enumerate() {
				if *portal == *portal_cell {
					return Some(translator.get(sector_id).unwrap()[i][j]);
				}
			}
		}
		None
	}
	// /// Iterate through the [Portals] [Ordinal] lists to locate the graph positional indices of a particular field portal position, these indices are then used to find the  [NodeIndex] from its recorded position in the "translator" (`self.node_index_translation`)
	// fn find_index_from_single_portal_and_portal_cell(
	// 	&self,
	// 	portals: &Portals,
	// 	sector_id: &SectorID,
	// 	portal_cell: &FieldCell,
	// ) -> Option<NodeIndex> {
	// 	let translator = &self.node_index_translation;
	// 	// locate the indices within sector_portals which can be used to access the
	// 	// right elements of the translator
	// 	for (i, ordinals) in portals.get().iter().enumerate() {
	// 		for (j, portal) in ordinals.iter().enumerate() {
	// 			if portal.get_column_row() == *portal_cell {
	// 				return Some(translator.get(sector_id).unwrap()[i][j]);
	// 			}
	// 		}
	// 	}
	// 	None
	// }
}

/// Iterate through the [Portals] [Ordinal] lists to locate the graph positional indices of a particular field portal position, these indices are then used to find the  [NodeIndex] from its recorded position in the "translator" (`self.node_index_translation`)
fn find_index_from_single_portal_and_portal_cell(
	translator: &BTreeMap<SectorID, [Vec<NodeIndex>; 4]>,
	portals: &Portals,
	sector_id: &SectorID,
	portal_cell: &FieldCell,
) -> Option<NodeIndex> {
	// locate the indices within sector_portals which can be used to access the
	// right elements of the translator
	for (i, ordinals) in portals.get().iter().enumerate() {
		for (j, portal) in ordinals.iter().enumerate() {
			if *portal == *portal_cell {
				return Some(translator.get(sector_id).unwrap()[i][j]);
			}
		}
	}
	None
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
		let mut portal_graph = PortalGraph::default();
		portal_graph.build_graph_nodes(&sector_portals);
		let result = portal_graph.graph.node_count();

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
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals, &sector_cost_fields);
		let result = portal_graph.graph.edge_count();

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
	fn portal_graph_basic_edge_count() {
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
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals, &sector_cost_fields);
		// build the edges between sectors
		portal_graph.build_edges_between_sectors(&sector_portals, &map_dimensions);
		let result = portal_graph.graph.edge_count();

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
	fn update_graph_from_portals_change() {
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
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals, &sector_cost_fields);
		// build the edges between sectors
		portal_graph.build_edges_between_sectors(&sector_portals, &map_dimensions);
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
		let result = portal_graph.graph.edge_count();
		// each portal in a sector is connected to every other portal in that sector
		let portal_to_portal_count = 54; //SHOULD BE 54
		// each sector boundary has an edge to the neighbouring sector boundary
		let sector_to_sector_count = 26;
		let actual = portal_to_portal_count + sector_to_sector_count;
		assert_eq!(actual, result);
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
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals, &sector_cost_fields);
		// build the edges between sectors
		portal_graph.build_edges_between_sectors(&sector_portals, &map_dimensions);

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
		let source = (SectorID::new(0, 0), FieldCell::new(4, 9));
		let target = (SectorID::new(0, 2), FieldCell::new(4, 0));
		let portal_path = portal_graph.find_path_between_sector_portals(source, target, &sector_portals);
		let path = portal_graph.convert_index_path_to_sector_portal_cells(portal_path.unwrap().1, &sector_portals);
		let actual = vec![(SectorID::new(0, 0), FieldCell::new(4, 9)), (SectorID::new(0, 1), FieldCell::new(4, 0)), (SectorID::new(0, 1), FieldCell::new(4, 9)), (SectorID::new(0, 2), FieldCell::new(4, 0))];
		
		assert_eq!(actual, path);
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
