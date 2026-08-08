// //! A Portal indicates a pathable area from one Sector to another.
// //!
// //! A [portal_graph::PortalGraph] is used to calculate a path between portals (effectively a
// //! high level path of traversing from one sector to another).

// use std::collections::{BTreeMap, BTreeSet};

// use bevy::prelude::*;
// use petgraph::{Directed, graph::NodeIndex, stable_graph::StableGraph};

// use crate::v2::flowfields::{
// 	fields::{Field, FieldCell, cost_field::CostField},
// 	sectors::{SectorID, sector_cost::SectorCostFields},
// 	utilities::Ordinal,
// };

// /// Describes the start and end indices of a portal
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Default, Debug, Clone, Reflect, PartialEq, Eq, Ord, PartialOrd, Copy)]
// struct PortalWindow {
// 	start: FieldCell,
// 	end: FieldCell,
// }

// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Default, Debug, Clone, Reflect, Ord, PartialOrd, Eq, Copy)]
// struct Portal {
// 	boundary_a: (SectorID, PortalWindow),
// 	boundary_b: (SectorID, PortalWindow),
// }

// impl PartialEq for Portal {
// 	fn eq(&self, other: &Self) -> bool {
// 		(self.boundary_a == other.boundary_a && self.boundary_b == other.boundary_b)
// 			|| (self.boundary_a == other.boundary_b && self.boundary_b == other.boundary_a)
// 	}
// }
// //TODO might need to impl PartialOrd

// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Component, Default, Debug, Clone, Reflect)]
// #[reflect(Component)]
// pub struct Portals {
// 	portals: BTreeSet<Portal>,
// 	#[reflect(ignore)] //TODO
// 	nodes: BTreeMap<Portal, NodeIndex<u32>>,
// 	#[reflect(ignore)] //TODO
// 	graph: StableGraph<u8, u8, Directed, u32>,
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
// 		// build the edges
// 		portals.generate_all_edges(graphs);

// 		portals
// 	}
// 	/// Iterate over all [CostField] and identify [`Portal`s] across sector boundaries
// 	fn generate_all_portals(&mut self, scaled_costs: &BTreeMap<SectorID, CostField>) {
// 		// step through each sector and compare boundaries with adjacent sectors
// 		// determining portals
// 		for (origin_sector, origin_field) in scaled_costs.iter() {
// 			self.generate_sector_portal(scaled_costs, origin_sector, origin_field);
// 		}
// 	}
// 	/// Walk through each boundary of sector and compute the [`Portal`s`]
// 	fn generate_sector_portal(
// 		&mut self,
// 		scaled_costs: &BTreeMap<SectorID, CostField>,
// 		origin_sector: &SectorID,
// 		origin_field: &CostField,
// 	) {
// 		let ordinals = [Ordinal::North, Ordinal::East, Ordinal::South, Ordinal::West];
// 		for ordinal in ordinals.iter() {
// 			if let Some(portals) =
// 				walk_sector_boundary(origin_sector, origin_field, scaled_costs, ordinal)
// 			{
// 				for portal in portals.iter() {
// 					self.portals.insert(*portal);
// 				}
// 			}
// 		}
// 	}
// 	/// Iterate over all [CostField] and identify [Node] across sector boundaries
// 	fn generate_all_nodes(&mut self) {
// 		for portal in self.portals.iter() {
// 			let node = self.graph.add_node(1);
// 			self.nodes.insert(*portal, node);
// 		}
// 	}
// 	/// Iterate through all [`Node`s] and establish graph edges between portal windows
// 	fn generate_all_edges(
// 		&mut self,
// 		cost_graphs: &BTreeMap<SectorID, StableGraph<u8, u8, Directed, u8>>,
// 	) {
// 		let nodes = &self.nodes;
// 		let portal_graph = &mut self.graph;
// 		for (portal, node) in nodes.iter() {
// 			// find all nodes that contain boundary sector of portal
// 		}
// 	}
// }

// fn walk_sector_boundary(
// 	origin_sector: &SectorID,
// 	origin_field: &CostField,
// 	scaled_costs: &BTreeMap<SectorID, CostField>,
// 	ordinal: &Ordinal,
// ) -> Option<Vec<Portal>> {
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
// 				current_window.push((origin_cell, adjacent_cells[1]));
// 			} else {
// 				if !current_window.is_empty() {
// 					let start = current_window.first().unwrap().clone();
// 					let end = current_window.last().unwrap().clone();
// 					windows.push((start, end));
// 					current_window.clear();
// 				}
// 			}
// 		}
// 		// turn windows into Portals
// 		let mut portals = vec![];
// 		for window in windows.iter() {
// 			let origin_window = PortalWindow {
// 				start: *window.0.0,
// 				end: *window.1.0,
// 			};
// 			let adjacent_window = PortalWindow {
// 				start: window.0.1,
// 				end: window.1.1,
// 			};
// 			let portal = Portal {
// 				boundary_a: (*origin_sector, origin_window),
// 				boundary_b: (adjacent_sector, adjacent_window),
// 			};
// 			portals.push(portal);
// 		}
// 		Some(portals)
// 	} else {
// 		None
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
