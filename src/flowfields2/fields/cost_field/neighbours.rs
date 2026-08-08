use crate::flowfields2::{
	fields::{Field, FieldCell, cost_field::CostField},
	utilities::{FIELD_RESOLUTION, Ordinal},
};

/// Recursively process the cells to see if there's a path
pub fn process_neighbours_visibility(
	cost_field: &CostField,
	target: &FieldCell,
	queue: Vec<FieldCell>,
	propagation: &mut [bool; FIELD_RESOLUTION * FIELD_RESOLUTION],
) -> bool {
	let mut next_queue = vec![];
	// iterate over the queue to explore neighbours
	for cell in queue.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
		// iterate over the neighbours to try and find the target
		for n in neighbours.iter() {
			if *n == *target {
				return true;
			}
			let cell_cost = cost_field.get_field_cell_value(*n);
			// ignore impassable cells
			if cell_cost != 255 {
				let index = n.as_1d_index();
				let has_existing_propagation = propagation[index];
				if !has_existing_propagation {
					propagation[index] = true;
					// keep exploring
					next_queue.push(*n);
				}
			}
		}
	}
	if !next_queue.is_empty() {
		process_neighbours_visibility(cost_field, target, next_queue, propagation)
	} else {
		false
	}
}
/// Recursively process the cells to see if there's a path and a weighting for the distance between the cell pair
pub fn process_neighbours_distance(
	cost_field: &CostField,
	target: &FieldCell,
	queue: Vec<(FieldCell, i32)>,
	propagation: &mut [i32; FIELD_RESOLUTION * FIELD_RESOLUTION],
) -> Option<i32> {
	let mut next_queue = vec![];
	for (cell, prev_cost) in queue.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
		for n in neighbours {
			let n_cost = cost_field.get_field_cell_value(n);
			// ignore impassable
			if n_cost != 255 {
				// let cumulative_cost = n_cost as i32 + prev_cost;
				let cumulative_cost = 1_i32 + prev_cost;
				let index = n.as_1d_index();
				let existing_propagation_cost = propagation[index];
				if cumulative_cost < existing_propagation_cost {
					propagation[index] = cumulative_cost;
					next_queue.push((n, cumulative_cost));
				}
			}
		}
	}
	if !next_queue.is_empty() {
		process_neighbours_distance(cost_field, target, next_queue, propagation)
	} else {
		let t_index = target.as_1d_index();
		let target_cumulative = propagation[t_index];
		if target_cumulative != 65535 {
			Some(target_cumulative)
		} else {
			None
		}
	}
}
