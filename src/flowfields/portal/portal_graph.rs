//! When an agent needs to path somewhere it is initially given a path based on moving from one portal
//! to another portal/end sector. The path is calculated from the [PortalGraph] which records the
//! points of navigation (`nodes`), the the paths bewteen them (`edges`).
//!
//! This ensures responsiveness so when a player issues a movement order
//! the agent immediately starts pathing. In the background the other components of the Flowfields can
//! calcualte a perfect path which can then supersede using portals to path when it's ready

use std::collections::BTreeMap;

use bevy::prelude::*;
use petgraph::{
	algo::astar,
	stable_graph::{NodeIndex, StableGraph},
};

use crate::flowfields::{
	sectors::{
		get_ids_of_neighbouring_sectors, get_ordinal_and_ids_of_neighbouring_sectors,
		get_xyz_from_field_cell_within_sector, SectorPortals,
	},
	Ordinal,
};

use super::portals::{PortalNode, Portals};

/// Each sector contains a series of Portals. A [StableGraph] allows a route to be calculated from one
/// sector to another via the portal boundaries.
///
/// To enable responsiveness in moving actors around a world they should initially be given a
/// route to navigate based upon moving between the Sectors of the world, later on once a
/// [FlowFields] has been generated they can be given a more accurate route to follow.
/// [PortalGraph] enables a navigational route to be found between sectors using the [Portals]
/// of sector boundaries to provide the responsiveness of "movement asked for, begin going in
/// this direction, get a better route later"
//TODO #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))] https://github.com/petgraph/petgraph/pull/550
#[derive(Component, Default)]
pub struct PortalGraph {
	graph: StableGraph<u32, i32, petgraph::Directed>,
	/// The `graph` cannot store [PortalNode]s directly instead it records entries with a [NodeIndex].
	/// Based on the vectors of each sector in [SectorPortals] we create a means of storing
	/// [NodeIndex] in an identical structure to [Portals]. This allows a [PortalNode] to be
	/// mapped to a [NodeIndex] and vice versa.
	///
	/// The keys of the map correspond to the unique IDs of Sectors. The tuple elements correspond to:
	/// * [NodeIndex] of the sector itself
	/// * An [Ordinal] list structure which should be identical in structure for each sectors [Portals]
	node_index_translation: BTreeMap<(u32, u32), (NodeIndex, [Vec<NodeIndex>; 4])>,
}
//TODO need a means of chekcing graph capacity, if it's near usize, usize then rebuild it from scrtach to reset size
impl PortalGraph {
	/// Builds the [StableGraph] nodes for each sector and the nodes for each portal within a sector
	pub fn build_graph_nodes(&mut self, sector_portals: &SectorPortals) -> &mut Self {
		for (sector_id, portals) in sector_portals.get().iter() {
			self.build_sector_nodes(sector_id, portals);
		}
		self
	}
	/// For the given Sector and its [Portals] add a node to the graph for the sector itself
	/// and a node for each [PortalNode]
	pub fn build_sector_nodes(&mut self, sector_id: &(u32, u32), portals: &Portals) -> &mut Self {
		let graph = &mut self.graph;
		let translator = &mut self.node_index_translation;
		// add a node representing the sector
		let sector_node = graph.add_node(1);
		// initialise the sector within the translator
		translator.insert(*sector_id, (sector_node, [vec![], vec![], vec![], vec![]]));
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
					Some(value) => {
						let ordinal_array = &mut value.1;
						ordinal_array[ordinal_index].push(node_index);
					}
					None => panic!("Translator doesn't contain sector {:?}", sector_id),
				}
			}
		}
		self
	}
	/// Builds the edges between each portal within every sector
	pub fn build_edges_within_each_sector(&mut self, sector_portals: &SectorPortals) -> &mut Self {
		// for each sector create edges
		for (sector_id, portals) in sector_portals.get().iter() {
			self.build_internal_sector_edges(sector_id, portals);
		}
		self
	}
	/// Create graph edges between each portal of the Sector
	pub fn build_internal_sector_edges(
		&mut self,
		sector_id: &(u32, u32),
		portals: &Portals,
	) -> &mut Self {
		let graph = &mut self.graph;
		let translator = &mut self.node_index_translation;
		// for each side of a sector create edges between the sector itself and portals
		//
		// find the index node of the graph that corresponds to the central sector node
		// in order to create a pathable edge from the sectors [NodeIndex] to each portal [NodeIndex]
		let sector_node_index = {
			match translator.get(sector_id) {
				Some(value) => value.0,
				None => panic!("Translator doesn't contain sector {:?}", sector_id),
			}
		};
		// create a combined list of portal points which can be iterated over to link a portal
		// to all portals in the sector
		let all_sector_portals = portals
			.get()
			.to_vec()
			.into_iter()
			.flatten()
			.collect::<Vec<PortalNode>>();
		let mut all_portal_indices = Vec::new();
		// for each ordinal (side of a sector) get the list of portal nodes
		for (ordinal_index, portal_node_list) in portals.get().iter().enumerate() {
			// for each portal find its index
			for (element_index, _portal_node) in portal_node_list.iter().enumerate() {
				let portal_node_index = {
					match translator.get(sector_id) {
						Some(v) => {
							let ordinal = &v.1[ordinal_index];
							ordinal.get(element_index).unwrap()
						}
						None => panic!("Translator doesn't contain sector {:?}", sector_id),
					}
				};
				// link the sector to each portal
				graph.add_edge(sector_node_index, *portal_node_index, 1);
				graph.add_edge(*portal_node_index, sector_node_index, 1);
				// store each portal index so they can be linked to each other later
				all_portal_indices.push(portal_node_index.clone());
			}
		}
		// link each portal to all other portals in the sector
		for (i, portal_index) in all_portal_indices.iter().enumerate() {
			for (j, target_index) in all_portal_indices.iter().enumerate() {
				if i == j {
					continue;
				} else {
					// ue len() squared between points as the weight
					let weight = {
						(all_sector_portals[j].get_column_row().0 as i32
							- all_sector_portals[i].get_column_row().0 as i32)
							.pow(2) + (all_sector_portals[j].get_column_row().1 as i32
							- all_sector_portals[i].get_column_row().1 as i32)
							.pow(2)
					};
					graph.add_edge(*portal_index, *target_index, weight);
				}
			}
		}
		self
	}
	/// Builds the edges from each sector boundary to another
	pub fn build_edges_between_sectors(
		&mut self,
		sector_portals: &SectorPortals,
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> &mut Self {
		for (sector_id, _portals) in sector_portals.get().iter() {
			self.build_external_sector_edges(sector_id, map_x_dimension, map_z_dimension);
		}
		self
	}
	/// Create edges along the boundary of the chosen Sector [PortalNode]s to its neighbouring
	/// sector boundary [PortalNode]s
	pub fn build_external_sector_edges(
		&mut self,
		sector_id: &(u32, u32),
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> &mut Self {
		let graph = &mut self.graph;
		let translator = &mut self.node_index_translation;
		let sector_neighbours = get_ordinal_and_ids_of_neighbouring_sectors(
			sector_id,
			map_x_dimension,
			map_z_dimension,
		);
		for (ordinal, neighbour_id) in sector_neighbours.iter() {
			match ordinal {
				Ordinal::North => {
					// use the northern boundary of this sector to connect portals to the southern
					// boundary of the neighbour
					// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
					let this_sector_portals = &translator.get(sector_id).unwrap().1[0];
					let neighbour_portals = &translator.get(neighbour_id).unwrap().1[2];
					for (i, portal_index) in this_sector_portals.iter().enumerate() {
						graph.update_edge(*portal_index, neighbour_portals[i], 1);
					}
				}
				Ordinal::East => {
					// use the eastern boundary of this sector to connect portals to the western
					// boundary of the neighbour
					// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
					let this_sector_portals = &translator.get(sector_id).unwrap().1[1];
					let neighbour_portals = &translator.get(neighbour_id).unwrap().1[3];
					for (i, portal_index) in this_sector_portals.iter().enumerate() {
						graph.update_edge(*portal_index, neighbour_portals[i], 1);
					}
				}
				Ordinal::South => {
					// use the southern boundary of this sector to connect portals to the northern
					// boundary of the neighbour
					// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
					let this_sector_portals = &translator.get(sector_id).unwrap().1[2];
					let neighbour_portals = &translator.get(neighbour_id).unwrap().1[0];
					for (i, portal_index) in this_sector_portals.iter().enumerate() {
						graph.update_edge(*portal_index, neighbour_portals[i], 1);
					}
				}
				Ordinal::West => {
					// use the western boundary of this sector to connect portals to the eastern
					// boundary of the neighbour
					// TODO this will panic if the adjoining boundary doesn't have the same number of portals, either constrain system ordering so rebuilding the portals has to finish before creating these edges or have a soft warning/come back later
					let this_sector_portals = &translator.get(sector_id).unwrap().1[3];
					let neighbour_portals = &translator.get(neighbour_id).unwrap().1[1];
					for (i, portal_index) in this_sector_portals.iter().enumerate() {
						graph.update_edge(*portal_index, neighbour_portals[i], 1);
					}
				}
				_ => panic!("Cannot create diagonals between sectors"),
			}
		}
		self
	}
	/// When a [CostFields] is updated the corresponding [Portals] should be updated. This means that
	/// the [PortalGraph]'s `graph` may no longer accurately reflect how to move from one sector to
	/// another. This method will recalculate the nodes and edges of the supplied sector and
	/// its neighbouring sectors.
	///
	/// # This must run after any updates to a [Portals]!
	pub fn update_graph(
		&mut self,
		changed_sector: (u32, u32),
		sector_portals: &SectorPortals,
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> &mut Self {
		let mut sectors_to_rebuild =
			get_ids_of_neighbouring_sectors(&changed_sector, map_x_dimension, map_z_dimension);
		let graph = &mut self.graph;
		let translator = &mut self.node_index_translation;
		// remove the nodes from the sector and its neighbours
		sectors_to_rebuild.push(changed_sector);
		for sector_id in sectors_to_rebuild.iter() {
			// lookup the [NodeIndex]s of each sector
			let indices = translator
				.get(sector_id)
				.expect("PortalGraph is missing a unique sector ID");
			let sector_node_index = indices.0;
			let ordinal_node_indices = &indices.1;
			// iterate over each node in each ordinal and remove them from the graph
			for ordinal in ordinal_node_indices.iter() {
				for node_index in ordinal.iter() {
					let removed = graph.remove_node(*node_index);
					if removed.is_none() {
						panic!("[PortalGraph] `node_index_translation` is not syncronised with the `graph`. Attempted to remove a portal within sector {:?}", sector_id);
					}
				}
			}
			graph.remove_node(sector_node_index);
		}
		// rebuild the nodes and  rebuild the edges within each sector
		for sector_id in sectors_to_rebuild.iter() {
			let portals = sector_portals
				.get()
				.get(sector_id)
				.expect("SectorPortals is missing a sector ID");
			self.build_sector_nodes(sector_id, portals);
			self.build_internal_sector_edges(sector_id, portals);
		}
		// rebuild the edges between each sector
		for sector_id in sectors_to_rebuild.iter() {
			self.build_external_sector_edges(sector_id, map_x_dimension, map_z_dimension);
		}
		// Note the adjacent sectors that were completely rebuilt have destroyed !their!
		// neighbours connectivity. Rebuild them too
		sectors_to_rebuild.pop(); // don't need to rebuild the one that had its cost fields changed
		for sector_id in sectors_to_rebuild.iter() {
			let neighbours_neighbour =
				get_ids_of_neighbouring_sectors(sector_id, map_x_dimension, map_z_dimension);
			for id in neighbours_neighbour.iter() {
				self.build_external_sector_edges(id, map_x_dimension, map_z_dimension);
			}
		}
		self
	}
	/// Replaces the current graph with a fresh one
	pub fn reset_graph(
		&mut self,
		sector_portals: &SectorPortals,
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> &mut Self {
		let mut graph = PortalGraph::default();
		graph
			.build_graph_nodes(&sector_portals)
			.build_edges_within_each_sector(&sector_portals)
			.build_edges_between_sectors(&sector_portals, map_x_dimension, map_z_dimension);
		self.graph = graph.graph;
		self
	}
	/// Uses A* pathfinding algorithm to produce a portal-to-portal path from a starting sector
	/// to an end sector
	pub fn find_path_of_sector_grid_indices(
		&self,
		source_sector: (u32, u32),
		target_sector: (u32, u32),
		sector_portals: &SectorPortals,
	) -> Option<BTreeMap<(u32, u32), Vec<(u32, u32)>>> {
		let source_index = match self.node_index_translation.get(&source_sector) {
			Some(v) => v.0,
			None => panic!("Translator doesn't contain sector {:?}", source_sector),
		};
		let target_index = match self.node_index_translation.get(&target_sector) {
			Some(v) => v.0,
			None => panic!("Translator doesn't contain sector {:?}", target_sector),
		};
		let estimate_cost = {
			(target_sector.0 as i32 - source_sector.0 as i32).pow(2)
				+ (target_sector.1 as i32 - source_sector.1 as i32).pow(2)
		};
		let path = astar(
			&self.graph,
			source_index,
			|fin| fin == target_index,
			|e| *e.weight(),
			|_| estimate_cost,
		);
		if let Some((_, nodes)) = path {
			let translator = &self.node_index_translation;
			let mut portal_node_list: BTreeMap<(u32, u32), Vec<(u32, u32)>> = BTreeMap::new();
			let mut working_path = nodes.clone();
			// init the sector IDs making up the path
			for node_index in nodes.iter() {
				for (sector_id, (sector_node_index, _)) in translator.iter() {
					if node_index == sector_node_index {
						portal_node_list.insert(*sector_id, vec![]);
						working_path.retain(|x| x != node_index);
					}
				}
			}
			// the first element of the working path is the exiting portal of the source sector
			let value = self.find_portal_node_cell_indices_from_node_index(
				sector_portals,
				&working_path[0],
				source_sector,
			);
			portal_node_list
				.get_mut(&source_sector)
				.unwrap()
				.push(value.unwrap());
			working_path.remove(0);
			// the last element of the working path is the entry portal of target sector
			let value = self.find_portal_node_cell_indices_from_node_index(
				sector_portals,
				&working_path.last().unwrap(),
				target_sector,
			);
			portal_node_list
				.get_mut(&target_sector)
				.unwrap()
				.push(value.unwrap());
			working_path.pop();
			// all other elements are pairs on entering a sector and exiting it
			let mut current_index = 1;
			for nodes in working_path.chunks(2) {
				for (i, (key, list)) in portal_node_list.iter_mut().enumerate() {
					if i == current_index {
						let value = self.find_portal_node_cell_indices_from_node_index(
							sector_portals,
							&nodes[0],
							*key,
						);
						list.push(value.unwrap());
						let value = self.find_portal_node_cell_indices_from_node_index(
							sector_portals,
							&nodes[1],
							*key,
						);
						list.push(value.unwrap());
					}
				}
				current_index += 1;
			}
			return Some(portal_node_list);
		};
		None
	}
	/// Uses A* pathfinding algorithm to produce a portal-to-portal path from a starting sector
	/// to an end sector in terms of real-world `x, y, z` coordinates with an origin at `(0, 0, 0)`
	///
	/// A value of `None` indicates that a path could be found
	pub fn find_path(
		&self,
		source_sector: (u32, u32),
		target_sector: (u32, u32),
		sector_portals: &SectorPortals,
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> Option<Vec<Vec3>> {
		let sector_indices =
			self.find_path_of_sector_grid_indices(source_sector, target_sector, sector_portals);
		match sector_indices {
			Some(map) => {
				// convert the indices of each sector into real world positions
				let mut real_world_coord_path: Vec<Vec3> = Vec::new();
				for (key, value) in map.iter() {
					for field_id in value.iter() {
						let real = get_xyz_from_field_cell_within_sector(
							*key,
							*field_id,
							map_x_dimension,
							map_z_dimension,
						);
						real_world_coord_path.push(real);
					}
				}

				Some(real_world_coord_path)
			}
			None => None,
		}
	}
	/// Search the "translator" (`self.node_index_translation`) based on a `sector` and a graph
	/// [NodeIndex] representation of a [PortalNode] to handle a backwards translation to prduce
	/// the `(column, row)` of the [PortalNode]
	fn find_portal_node_cell_indices_from_node_index(
		&self,
		sector_portals: &SectorPortals,
		search_index: &NodeIndex,
		sector: (u32, u32),
	) -> Option<(u32, u32)> {
		let translator = &self.node_index_translation;
		for (i, node_ordinal) in translator.get(&sector).unwrap().1.iter().enumerate() {
			for (j, node_index) in node_ordinal.iter().enumerate() {
				if *search_index == *node_index {
					return Some(
						sector_portals.get().get(&sector).unwrap().get()[i][j].get_column_row(),
					);
				}
			}
		}
		None
	}
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use crate::flowfields::sectors::SectorCostFields;

use super::*;
	#[test]
	fn portal_graph_node_count() {
		//init
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
		let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, map_x_dimension, map_z_dimension),
				None => assert!(false),
			}
		}
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.build_graph_nodes(&sector_portals);
		let result = portal_graph.graph.node_count();

		let sector_count = 9; // each sector produces a node
		let portal_count = 24; // sum of portals for each sector in the 3x3 sector grid
		let actual = sector_count + portal_count;
		assert_eq!(actual, result);
	}
	#[test]
	fn portal_graph_basic_sector_edge_count() {
		//init
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
		let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, map_x_dimension, map_z_dimension),
				None => assert!(false),
			}
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals);
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
		// each sector has an edge to edge portal on its boundary
		let sector_to_portal_count = 48;
		// each portal in a sector is connected to every other portal in that sector
		let portal_to_portal_count = 44;
		let actual = sector_to_portal_count + portal_to_portal_count;
		assert_eq!(actual, result);
	}
	#[test]
	fn portal_graph_basic_edge_count() {
		//init
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
		let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, map_x_dimension, map_z_dimension),
				None => assert!(false),
			}
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals);
		// build the edges between sectors
		portal_graph.build_edges_between_sectors(&sector_portals, map_x_dimension, map_z_dimension);
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
		// each sector has an edge to edge portal on its boundary
		let sector_to_portal_count = 48;
		// each portal in a sector is connected to every other portal in that sector
		let portal_to_portal_count = 44;
		// each sector boundary has an edge to the neighbouring sector boundary
		let sector_to_sector_count = 24;
		let actual = sector_to_portal_count + portal_to_portal_count + sector_to_sector_count;
		assert_eq!(actual, result);
	}
	#[test]
	fn update_graph_from_portals_change() {
		//init
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let mut sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
		let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
		// build portals
		for (id, portals) in sector_portals.get_mut().iter_mut() {
			portals.recalculate_portals(&sector_cost_fields, id, map_x_dimension, map_z_dimension)
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals);
		// build the edges between sectors
		portal_graph.build_edges_between_sectors(&sector_portals, map_x_dimension, map_z_dimension);
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
		let mutated_sector_id = (0, 0);
		let field = sector_cost_fields.get_mut().get_mut(&mutated_sector_id).unwrap();
		field.set_grid_value(255, 4, 9);
		sector_portals.update_portals(mutated_sector_id, &sector_cost_fields, map_x_dimension, map_z_dimension);

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
		portal_graph.update_graph(mutated_sector_id, &sector_portals, map_x_dimension, map_z_dimension);
		// test that the graph has updated with the new edges
		let result = portal_graph.graph.edge_count();
		// each sector has an edge to edge portal on its boundary
		let sector_to_portal_count = 52;
		// each portal in a sector is connected to every other portal in that sector
		let portal_to_portal_count = 54; //SHOULD BE 54
		// each sector boundary has an edge to the neighbouring sector boundary
		let sector_to_sector_count = 26;
		let actual = sector_to_portal_count + portal_to_portal_count + sector_to_sector_count;
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
	// 	field.set_grid_value(255, 4, 9);
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
	fn best_path_indices() {
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
		let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, map_x_dimension, map_z_dimension),
				None => assert!(false),
			}
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals);
		// build the edges between sectors
		portal_graph.build_edges_between_sectors(&sector_portals, map_x_dimension, map_z_dimension);

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

		let source_sector = (0, 0);
		let target_sector = (0, 2);
		let path = portal_graph.find_path_of_sector_grid_indices(source_sector, target_sector, &sector_portals);
		let mut actual = BTreeMap::new();
		actual.insert((0, 0), vec![(4, 9)]);
		actual.insert((0, 1), vec![(4, 0), (4, 9)]);
		actual.insert((0, 2), vec![(4, 0)]);
		match path {
			Some(p) => assert_eq!(actual, p),
			None => assert!(false),
		}
	}
	#[test]
	fn best_path_xyz() {
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let sector_cost_fields = SectorCostFields::new(map_x_dimension, map_z_dimension);
		let mut sector_portals = SectorPortals::new(map_x_dimension, map_z_dimension);
		// build portals
		for (sector_id, _cost_fields) in sector_cost_fields.get().iter() {
			let portals = sector_portals.get_mut();
			match portals.get_mut(sector_id) {
				Some(portals) => portals.recalculate_portals(&sector_cost_fields, sector_id, map_x_dimension, map_z_dimension),
				None => assert!(false),
			}
		}
		
		// build the graph
		let mut portal_graph = PortalGraph::default();
		portal_graph.build_graph_nodes(&sector_portals);
		// build the edges within each sector
		portal_graph.build_edges_within_each_sector(&sector_portals);
		// build the edges between sectors
		portal_graph.build_edges_between_sectors(&sector_portals, map_x_dimension, map_z_dimension);

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

		let source_sector = (0, 0);
		let target_sector = (0, 2);
		let path = portal_graph.find_path(source_sector, target_sector, &sector_portals, map_x_dimension, map_z_dimension);
		println!("Path {:?}", path);
		match path {
			Some(_) => assert!(true),
			None => assert!(false)
		}
	}
	//TODO more test, must be robust
}
