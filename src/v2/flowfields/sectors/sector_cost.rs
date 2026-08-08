//! A map is split into a series of `MxN` sectors where each has a [CostField]
//! associated with it
//!
//!

use bevy::prelude::*;
use petgraph::{Directed, stable_graph::StableGraph};
use std::collections::BTreeMap;

use crate::v2::flowfields::{
	dimensions::Dimensions,
	fields::{Field, FieldCell, cost_field::CostField},
	sectors::SectorID,
	utilities::{FIELD_RESOLUTION, Ordinal},
};

/// Keys represent unique sector IDs and are in the format of `(column, row)`
/// when considering a grid of sectors across the map. The sectors begin in the
/// top left of the world dimensions and values are the [CostField] associated
/// with that sector
#[cfg_attr(
	feature = "serde",
	derive(serde::Deserialize, serde::Serialize),
	serde(default)
)]
#[derive(Component, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct SectorCostFields {
	/// Initial costs based on the unit size of each field
	baseline: BTreeMap<SectorID, CostField>,
	/// Each [FieldCell] containing an impassable `255` value is scaled based on actor size to close off gaps which the actor could not path through
	scaled: BTreeMap<SectorID, CostField>,
	/// Each scaled [CostField] requires a graph of local traversal. This is used to determine is a [FieldCell] is able to traverse to a [Portal]
	#[reflect(ignore)] //TODO
	graphs: BTreeMap<SectorID, StableGraph<u8, u8, Directed, u16>>,
}

impl SectorCostFields {
	/// Create a new instance of [SectorCostFields] based on the dimensions
	pub fn new(dimensions: &Dimensions) -> Self {
		let mut sector_cost_fields = SectorCostFields::default();
		let column_count = dimensions.get_sector_column_count();
		let row_count = dimensions.get_sector_row_count();
		for m in 0..column_count as i32 {
			for n in 0..row_count as i32 {
				sector_cost_fields
					.baseline
					.insert(SectorID::new(m, n), CostField::default());
				sector_cost_fields
					.graphs
					.insert(SectorID::new(m, n), StableGraph::default());
			}
		}
		sector_cost_fields.scale_all_costfields(dimensions);
		create_all_graphs(&mut sector_cost_fields.graphs, &sector_cost_fields.scaled);

		sector_cost_fields
	}
	/// Create a new instance of [SectorCostFields] based on the map dimensions where the supplied `cost` is used as the default value in all [CostField]
	fn new_with_cost(dimensions: &Dimensions, cost: u8) -> Self {
		let mut sector_cost_fields = SectorCostFields::default();
		let column_count = dimensions.get_sector_column_count();
		let row_count = dimensions.get_sector_row_count();
		for m in 0..column_count as i32 {
			for n in 0..row_count as i32 {
				sector_cost_fields
					.baseline
					.insert(SectorID::new(m, n), CostField::new_with_cost(cost));
				sector_cost_fields
					.graphs
					.insert(SectorID::new(m, n), StableGraph::default());
			}
		}
		sector_cost_fields.scale_all_costfields(dimensions);
		create_all_graphs(&mut sector_cost_fields.graphs, &sector_cost_fields.scaled);

		sector_cost_fields
	}
	/// Iterate over all sectors and scale any impassable [FieldCell] based on `actor_scale`.
	///
	/// This can be expensive so should typically be used as part of data initialisation, i.e when loading [SectorCostFields] from a file or within a loading type of operation to a world
	pub fn scale_all_costfields(&mut self, dimensions: &Dimensions) {
		let sector_ids: Vec<SectorID> = self.baseline.keys().cloned().collect();
		for sector_id in sector_ids.iter() {
			self.scaled
				.insert(*sector_id, self.baseline.get(sector_id).unwrap().clone());
		}
		// only proceed if scaling is required
		if dimensions.get_actor_scale() == 1 {
			return;
		}
		for sector_id in sector_ids.iter() {
			self.scale_costfield(sector_id, dimensions);
		}
	}
	/// Inspects a sector for impassable cost values and based on an actor
	/// scale it expands any impassable costs into any neighbouring [FieldCell]
	/// walls. This is to close off any gaps so that the actor won't try and path
	/// through a gap it can't fit through
	fn scale_costfield(&mut self, sector_id: &SectorID, dimensions: &Dimensions) {
		let scale_count = dimensions.get_actor_scale();
		let base = self.baseline.get(sector_id).unwrap();
		let scaled = &mut self.scaled;

		let base_field = base.get();
		// search through all costs looking for impassable values to scale into other cells
		for (index, value) in base_field.iter().enumerate() {
			if *value == 255 {
				// this index needs to expanded in all directions based on actor scale
				// and cell values updated
				let cell = FieldCell::from_index(index);

				// walk north
				scale_in_ordinal_direction(&Ordinal::North, scale_count, cell, sector_id, scaled);
				// walk east
				scale_in_ordinal_direction(&Ordinal::East, scale_count, cell, sector_id, scaled);
				// walk south
				scale_in_ordinal_direction(&Ordinal::South, scale_count, cell, sector_id, scaled);
				// walk west
				scale_in_ordinal_direction(&Ordinal::West, scale_count, cell, sector_id, scaled);
				// NE
				scale_in_ordinal_direction(
					&Ordinal::NorthEast,
					scale_count,
					cell,
					sector_id,
					scaled,
				);
				// SE
				scale_in_ordinal_direction(
					&Ordinal::SouthEast,
					scale_count,
					cell,
					sector_id,
					scaled,
				);
				// SW
				scale_in_ordinal_direction(
					&Ordinal::SouthWest,
					scale_count,
					cell,
					sector_id,
					scaled,
				);
				// NW
				scale_in_ordinal_direction(
					&Ordinal::NorthWest,
					scale_count,
					cell,
					sector_id,
					scaled,
				);
			}
		}
	}
	/// Get a reference to the scaled fields
	pub fn get_scaled_costs(&self) -> &BTreeMap<SectorID, CostField> {
		&self.scaled
	}
	/// Get a reference to the graphs of sector [FieldCell] connectivity
	pub fn get_graphs(&self) -> &BTreeMap<SectorID, StableGraph<u8, u8, Directed, u16>> {
		&self.graphs
	}
	/// Set a [FieldCell] cost within a [SectorID]. This will recalculate scaling and graphs
	pub fn set_field_cost(
		&mut self,
		sector: &SectorID,
		field_cell: &FieldCell,
		cost: u8,
		dimensions: &Dimensions,
	) {
		// set the new cost in base and reset the scaled field for regenerating
		if let Some(field) = self.baseline.get_mut(sector) {
			field.set_field_cell_value(cost, *field_cell);
			*self.scaled.get_mut(sector).unwrap() = field.clone();
		}
		// reset the scaled field of the sectors around the mutated one
		let ordinals = [
			Ordinal::North,
			Ordinal::East,
			Ordinal::South,
			Ordinal::West,
			Ordinal::NorthEast,
			Ordinal::SouthEast,
			Ordinal::SouthWest,
			Ordinal::NorthWest,
		];
		let mut adjacent_sectors = vec![];
		for ordinal in ordinals.iter() {
			let adjacent = sector.get_in_ordinal_direction(ordinal, 1);
			// only store valid sector
			if self.baseline.contains_key(&adjacent) {
				adjacent_sectors.push(adjacent);
			}
		}
		for adjacent in adjacent_sectors.iter() {
			if let Some(base_field) = self.baseline.get(adjacent) {
				*self.scaled.get_mut(adjacent).unwrap() = base_field.clone();
			}
		}
		// re-scale the sectors
		self.scale_costfield(sector, dimensions);
		for adjacent in adjacent_sectors.iter() {
			self.scale_costfield(adjacent, dimensions);
		}
		//TODO is it feasible to just modify existing graphs instead of rebuilding?
		// regenerate graph of modified sectors
		wipe_sector_graph(sector, &mut self.graphs);
		if let Some(graph) = self.graphs.get_mut(sector) {
			create_graph_for_sector(sector, graph, &self.scaled);
		}
		for adjacent in adjacent_sectors.iter() {
			wipe_sector_graph(adjacent, &mut self.graphs);
			if let Some(graph) = self.graphs.get_mut(adjacent) {
				create_graph_for_sector(adjacent, graph, &self.scaled);
			}
		}
	}
}

/// Walk a number of `scale_count` steps in an [Ordinal] marking any scaled
/// [FieldCell] along the way as impassable if it collides which an existing
/// wall
fn scale_in_ordinal_direction(
	ordinal: &Ordinal,
	scale_count: u32,
	origin_cell: FieldCell,
	origin_sector_id: &SectorID,
	scaled: &mut BTreeMap<SectorID, CostField>,
) {
	let mut has_hit_wall = false;
	let mut fields_to_mark = vec![];
	'scale_loop: for n in 1..=scale_count {
		// find any sector change and cell arrived at
		let (sector_delta, next_cell) = ordinal.step_cell_in_direction(&origin_cell, n as usize);
		let next_sector = SectorID::new(
			origin_sector_id.column + sector_delta.column,
			origin_sector_id.row + sector_delta.row,
		);
		// verify next_sector is real as the delta doesn't account for sector boundary
		if let Some(cost_field) = scaled.get(&next_sector) {
			let cost = cost_field.get_field_cell_value(next_cell);
			if cost == 255 {
				has_hit_wall = true;
				break 'scale_loop;
			} else {
				fields_to_mark.push((*origin_sector_id, next_cell));
			}
		} else {
			// next_sector is not real which means the boundary of the world has been reached,
			// close the gap
			has_hit_wall = true;
			break;
		}

		// calc next FieldCell
		if let Some(new_cell) = origin_cell.get_in_ordinal_direction(ordinal, n as usize) {
			// if hitting wall then breakout and close the gap
			let cost = scaled
				.get(origin_sector_id)
				.unwrap()
				.get_field_cell_value(new_cell);
			if cost == 255 {
				has_hit_wall = true;
				break 'scale_loop;
			} else {
				fields_to_mark.push((*origin_sector_id, new_cell));
			}
		} else {
			// encountered sector boundary
			// attempt to propagate remaining steps into neighbouring sector
			let (sector_delta, entry_cell) = ordinal.get_sector_cell_entry(&origin_cell);
			let next_sector = SectorID::new(
				origin_sector_id.column + sector_delta.column,
				origin_sector_id.row + sector_delta.row,
			);
			let steps_remaining = scale_count - n;
			if let Some(next_field) = scaled.get_mut(&next_sector) {
				// begin walking in the neighbour sector
				for a in 0..=steps_remaining {
					// given that actor scale never exceeds field len we don't
					// need to worry about walking into any more
					// adjacent sectors
					if let Some(new_cell) = entry_cell.get_in_ordinal_direction(ordinal, a as usize)
					{
						// if hitting wall then breakout and close the gap
						let cost = next_field.get_field_cell_value(new_cell);
						if cost == 255 {
							has_hit_wall = true;
							break 'scale_loop;
						} else {
							fields_to_mark.push((next_sector, new_cell));
						}
					} else {
						// encountered another sector boundary
						// attempt to propagate remaining steps into neighbouring sector
						// due to actor scale constraint this is the only extra time
						// a different sector may be entered
						let (sector_delta, entry_cell) =
							ordinal.get_sector_cell_entry(&origin_cell);
						let next_sector = SectorID::new(
							origin_sector_id.column + sector_delta.column,
							origin_sector_id.row + sector_delta.row,
						);
						let steps_remaining = scale_count - n;
					}
				}
			} else {
				// next sector is not valid, stop walking
				// but mark cells as we've hit a world boundary so the gap should be closed
				has_hit_wall = true;
				break;
			}
		}
	}
	if has_hit_wall {
		for (sector, cell) in fields_to_mark.iter() {
			if let Some(field) = scaled.get_mut(sector) {
				field.set_field_cell_value(255, *cell);
			}
		}
	}
}

/// For every [SectorID] generate a graph of which [FieldCell] can path to other [FieldCell]
fn create_all_graphs(
	graphs: &mut BTreeMap<SectorID, StableGraph<u8, u8, Directed, u16>>,
	scaled_costs: &BTreeMap<SectorID, CostField>,
) {
	for (sector, graph) in graphs.iter_mut() {
		create_graph_for_sector(sector, graph, scaled_costs);
	}
}

/// Create graph nodes and edges for a particular sector
fn create_graph_for_sector(
	sector: &SectorID,
	graph: &mut StableGraph<u8, u8, Directed, u16>,
	scaled_costs: &BTreeMap<SectorID, CostField>,
) {
	// add nodes represented at `index` convention of FieldCell
	for _ in 0..FIELD_RESOLUTION * FIELD_RESOLUTION {
		graph.add_node(1);
	}
	let sector_costs = scaled_costs.get(sector).unwrap();
	// based on cost value create edges between nodes
	for n in 0..FIELD_RESOLUTION * FIELD_RESOLUTION {
		// using n as the index into the `field` of CostField
		// we can use FieldCell to easy find its neighbours
		let origin = FieldCell::from_index(n as usize);
		let origin_cost = sector_costs.get_field_cell_value(origin);
		// if origin has cost 255 then it won't have any edges
		if origin_cost == 255 {
			continue;
		}
		// if a neighbour has cost 255 an edge cannot be created to it
		let neighbours = origin.get_orthogonal_neighbours();
		for n_cell in neighbours.iter() {
			let n_cost = sector_costs.get_field_cell_value(*n_cell);
			if n_cost == 255 {
				continue;
			}
			let index = n_cell.as_1d_index() as u16;
			graph.add_edge((n as u16).into(), index.into(), 1);
		}
	}
}

/// Remove all nodes and edges from a graph
fn wipe_sector_graph(
	sector: &SectorID,
	graphs: &mut BTreeMap<SectorID, StableGraph<u8, u8, Directed, u16>>,
) {
	if let Some(graph) = graphs.get_mut(sector) {
		graph.clear();
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use petgraph::graph::NodeIndex;

	use super::*;

	// mutate a costs and ensure a gap between them is closed
	#[test]
	fn scale_one_field() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 1.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let mut sector_costs = SectorCostFields::new(&dimensions);
		// create a wall gap that should be filled in by scaling
		let mutate_sector = SectorID::new(0, 0);
		let mutate_cell1 = FieldCell::new(4, 4);
		let mutate_cell2 = FieldCell::new(6, 4);
		let cost = 255;
		sector_costs.set_field_cost(&mutate_sector, &mutate_cell1, cost, &dimensions);
		sector_costs.set_field_cost(&mutate_sector, &mutate_cell2, cost, &dimensions);

		let scaled_cell = FieldCell::new(5, 4);
		let result = sector_costs
			.scaled
			.get(&mutate_sector)
			.unwrap()
			.get_field_cell_value(scaled_cell);
		assert!(result == 255)
	}

	// find shortest path through graph without modifying any costs
	#[test]
	fn graph_path_unmodified() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let sector_costs = SectorCostFields::new(&dimensions);

		let sector = SectorID::new(0, 0);
		let start = 0;
		let end = 9;
		let graph = sector_costs.graphs.get(&sector).unwrap();

		let result = petgraph::algo::astar(
			graph,
			start.into(),
			|finish| finish == end.into(),
			|edge| *edge.weight(),
			|_| 0,
		)
		.unwrap();

		let result_cost = result.0;
		let result_path = result.1;

		let actual_cost = 9;
		let actual_path = vec![
			NodeIndex::new(0),
			NodeIndex::new(1),
			NodeIndex::new(2),
			NodeIndex::new(3),
			NodeIndex::new(4),
			NodeIndex::new(5),
			NodeIndex::new(6),
			NodeIndex::new(7),
			NodeIndex::new(8),
			NodeIndex::new(9),
		];

		assert_eq!(actual_cost, result_cost);
		assert_eq!(actual_path, result_path);
	}

	// find shortest path through graph with walls blocking direct path
	#[test]
	fn graph_path_modified() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 0.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let mut sector_costs = SectorCostFields::new(&dimensions);

		let sector = SectorID::new(0, 0);
		let mutate_cell = FieldCell::new(1, 0);
		sector_costs.set_field_cost(&sector, &mutate_cell, 255, &dimensions);

		let start = 0;
		let end = 2;
		let graph = sector_costs.graphs.get(&sector).unwrap();

		let result = petgraph::algo::astar(
			graph,
			start.into(),
			|finish| finish == end.into(),
			|edge| *edge.weight(),
			|_| 0,
		)
		.unwrap();

		let result_cost = result.0;
		let result_path = result.1;

		let actual_cost = 4;
		let actual_path = vec![
			NodeIndex::new(0),
			NodeIndex::new(10),
			NodeIndex::new(11),
			NodeIndex::new(12),
			NodeIndex::new(2),
		];

		assert_eq!(actual_cost, result_cost);
		assert_eq!(actual_path, result_path);
	}

	// fail to find path where wall scaling has closed off a gap
	#[test]
	fn graph_path_scaled() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 1.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let mut sector_costs = SectorCostFields::new(&dimensions);

		let sector = SectorID::new(0, 0);
		// walls running north to south with small gap in the middle
		let mutates = [
			FieldCell::new(3, 0),
			FieldCell::new(3, 1),
			FieldCell::new(3, 2),
			FieldCell::new(3, 3),
			FieldCell::new(3, 4),
			FieldCell::new(3, 6),
			FieldCell::new(3, 7),
			FieldCell::new(3, 8),
			FieldCell::new(3, 9),
		];
		// updating costs with walls should cause gap to close
		for cell in mutates.iter() {
			sector_costs.set_field_cost(&sector, cell, 255, &dimensions);
		}

		let start = 17;
		let end = 11;
		let graph = sector_costs.graphs.get(&sector).unwrap();

		let result = petgraph::algo::astar(
			graph,
			start.into(),
			|finish| finish == end.into(),
			|edge| *edge.weight(),
			|_| 0,
		);

		assert!(result.is_none());
	}

	// ensure walls scale to world boundary when no more sectors in direction
	#[test]
	fn graph_scaled_world_boundary() {
		let origin = (0.0, 0.0);
		let size = (20.0, 20.0);
		let world_unit_size = 1.0;
		let actor_size = 1.5;
		let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
		let mut sector_costs = SectorCostFields::new(&dimensions);

		let sector = SectorID::new(0, 0);
		let mutate = FieldCell::new(1, 1);
		// place wall just before corner so it should get scaled to the boundary
		sector_costs.set_field_cost(&sector, &mutate, 255, &dimensions);

		let blocked_cell1 = FieldCell::new(0, 1);
		let blocked_cell2 = FieldCell::new(1, 0);

		let blocked_cost1 = sector_costs
			.scaled
			.get(&sector)
			.unwrap()
			.get_field_cell_value(blocked_cell1);
		let blocked_cost2 = sector_costs
			.scaled
			.get(&sector)
			.unwrap()
			.get_field_cell_value(blocked_cell2);

		assert!(blocked_cost1 == 255);
		assert!(blocked_cost2 == 255);
	}
}
