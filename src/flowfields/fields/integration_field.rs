//! The IntegrationField contains a 2D array of 16-bit values and it uses a [CostField] to
//! produce a cumulative cost of reaching the goal/target. Every Sector has a [IntegrationField] associated with it.
//!
//! When a new route needs to be processed the field is reset to `u16::MAX` and the field cell containing the goal is set to `0`. A series of passes are performed from the goal as an expanding wavefront calculating the field values:
//!
//! 1. The valid ordinal neighbours of the goal are determined (North, East, South, West, when not against a boundary)
//! 2. For each ordinal field cell lookup their `CostField` value
//! 3. Add their cost to the `IntegrationField`s cost of the current cell (at the beginning this is the goal so + `0`)
//! 4. Propagate to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their integration cost, and repeat until the entire field is done
//!
//! This produces a nice diamond-like pattern as the wave expands (the underlying `CostField` are set to `1` here):
//!
//! ```text
//!  ___________________________________________________________
//! |     |     |     |     |     |     |     |     |     |     |
//! |  8  |  7  |  6  |  5  |  4  |  5  |  6  |  7  |  8  |  9  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  7  |  6  |  5  |  4  |  3  |  4  |  5  |  6  |  7  |  8  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  6  |  5  |  4  |  3  |  2  |  3  |  4  |  5  |  6  |  7  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  5  |  4  |  3  |  2  |  1  |  2  |  3  |  4  |  5  |  6  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  4  |  3  |  2  |  1  |  0  |  1  |  2  |  3  |  4  |  5  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  5  |  4  |  3  |  2  |  1  |  2  |  3  |  4  |  5  |  6  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  6  |  5  |  4  |  3  |  2  |  3  |  4  |  5  |  6  |  7  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  7  |  6  |  5  |  4  |  3  |  4  |  5  |  6  |  7  |  8  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  8  |  7  |  6  |  5  |  4  |  5  |  6  |  7  |  8  |  9  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! |     |     |     |     |     |     |     |     |     |     |
//! |  9  |  8  |  7  |  6  |  5  |  6  |  7  |  8  |  9  | 10  |
//! |_____|_____|_____|_____|_____|_____|_____|_____|_____|_____|
//! ```
//!
//! When it comes to `CostField` containing impassable markers, `255` as black boxes, they are ignored so the wave flows around those areas and when your `CostField` is using a range of values to indicate different areas to traverse, such as a steep hill, then you have various intermediate values similar to a terrain gradient.
//!
//! So this encourages the pathing algorithm around obstacles and expensive regions.
//!

use crate::prelude::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy)]
pub struct IntegrationField([[u16; FIELD_RESOLUTION]; FIELD_RESOLUTION]);

impl Default for IntegrationField {
	fn default() -> Self {
		IntegrationField([[u16::MAX; FIELD_RESOLUTION]; FIELD_RESOLUTION])
	}
}

impl Field<u16> for IntegrationField {
	/// Get a reference to the field array
	fn get(&self) -> &[[u16; FIELD_RESOLUTION]; FIELD_RESOLUTION] {
		&self.0
	}
	/// Retrieve a field cell value
	fn get_field_cell_value(&self, field_cell: FieldCell) -> u16 {
		if field_cell.get_column() >= self.0.len() || field_cell.get_row() >= self.0[0].len() {
			panic!("Cannot get a IntegrationField value, index out of bounds. Asked for column {}, row {}, field column length is {}, field row length is {}", field_cell.get_column(), field_cell.get_row(), self.0.len(), self.0[0].len())
		}
		self.0[field_cell.get_column()][field_cell.get_row()]
	}
	/// Set a field cell to a value
	fn set_field_cell_value(&mut self, value: u16, field_cell: FieldCell) {
		if field_cell.get_column() >= self.0.len() || field_cell.get_row() >= self.0[0].len() {
			panic!("Cannot set a IntegrationField value, index out of bounds. Asked for column {}, row {}, field column length is {}, field row length is {}", field_cell.get_column(), field_cell.get_row(), self.0.len(), self.0[0].len())
		}
		self.0[field_cell.get_column()][field_cell.get_row()] = value;
	}
}
impl IntegrationField {
	/// Creates a new [IntegrationField] where all cells are set to `u16::MAX` apart from the `goals` which is set to `0`
	pub fn new(goals: &Vec<FieldCell>) -> Self {
		let mut field = IntegrationField([[u16::MAX; FIELD_RESOLUTION]; FIELD_RESOLUTION]);
		for goal in goals {
			field.set_field_cell_value(0, *goal);
		}
		field
	}
	/// Reset all the cells of the [IntegrationField] to `u16::MAX` apart from the `goals` which are the starting points of calculating the field which is set to `0`
	pub fn reset(&mut self, goals: &Vec<FieldCell>) {
		for i in 0..FIELD_RESOLUTION {
			for j in 0..FIELD_RESOLUTION {
				self.set_field_cell_value(u16::MAX, FieldCell::new(i, j));
			}
		}
		for goal in goals {
			self.set_field_cell_value(0, *goal);
		}
	}
	//TODO: diamond like propagation and wasted extra lookups looking at previously calcualted neighbours, try fast marching method of solving Eikonal PDE for a spherical approx that visits each cell once
	/// From a list of `goals` (the actual end target goal or portal field cells
	/// to the next sector towards the goal sector) field cells iterate over
	/// successive neighbouring cells and calculate the field values from the
	/// `cost_field`
	pub fn calculate_field(&mut self, goals: &[FieldCell], cost_field: &CostField) {
		// further positions to process, tuple element 0 is the position, element 1 is the integration cost from the previous cell needed to help calculate element 0s cost
		let mut queue: Vec<(FieldCell, u16)> = Vec::new();
		for goal in goals.iter() {
			queue.push(((*goal), self.get_field_cell_value(*goal)));
		}
		process_neighbours(self, queue, cost_field);
	}
}

/// Recursively expand the neighbours of a list of [FieldCell] and calculate
/// their value in the [IntegrationField]
fn process_neighbours(
	int_field: &mut IntegrationField,
	queue: Vec<(FieldCell, u16)>,
	cost_field: &CostField,
) {
	let mut next_neighbours = Vec::new();
	// iterate over the queue calculating neighbour int costs
	for (cell, prev_int_cost) in queue.iter() {
		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
		// iterate over the neighbours calculating int costs
		for n in neighbours.iter() {
			let cell_cost = cost_field.get_field_cell_value(*n);
			// ignore impassable cells
			if cell_cost != 255 {
				// don't overwrite an int cell with a better cost
				let int_cost = cell_cost as u16 + prev_int_cost;
				if int_cost < int_field.get_field_cell_value(*n) {
					int_field.set_field_cell_value(int_cost, *n);
					next_neighbours.push((*n, int_cost));
				}
			}
		}
	}
	if !next_neighbours.is_empty() {
		process_neighbours(int_field, next_neighbours, cost_field);
	}
}

// impl IntegrationField {
// 	pub fn __calculate_field(&self, goals: &[FieldCell], cost_field: &CostField) {
// 		// let locked_this = this.lock().unwrap();
// 		let queue: Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>> = Mutex::new(Vec::new());
// 		let mut locked_queue = queue.lock().unwrap();
// 		for goal in goals.iter() {
// 			locked_queue.push((Arc::new(*goal), Arc::new(self.get_field_cell_value(*goal))));
// 		}
// 		drop(locked_queue);
// 		// drop(locked_this);
// 		let cost_field = Arc::new(cost_field.clone());
// 		let int_field = Arc::new(Mutex::new(self));
// 		// process_queue2(int_field, queue, cost_field);
// 	}
// }

// fn process_queue2(
// 	int_field: Arc<Mutex<IntegrationField>>,
// 	queue: Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>,
// 	cost_field: Arc<CostField>,
// ) -> Arc<Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>> {
// 	let next_queue: Arc<Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>> = Arc::new(Mutex::new(vec![]));
// 	let queue_values = queue.lock().unwrap();
// 	let mut handles = vec![];
// 	for (cell, prev_int_cost) in queue_values.iter() {
// 		let cost_field = Arc::clone(&cost_field);
// 		let int_field = Arc::clone(&int_field);
// 		let cell = Arc::clone(cell);
// 		let prev_int_cost = Arc::clone(prev_int_cost);
// 		let next_queue = Arc::clone(&next_queue);
// 		let handle = std::thread::spawn(move || {
// 			let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
// 			// iterate over the neighbours calculating int costs
// 			for n in neighbours.iter() {
// 				process_neighbour(&cost_field, n, &prev_int_cost, &int_field, &next_queue);
// 			}
// 		});
// 		handles.push(handle);
// 	}
// 	for h in handles {
// 		h.join().unwrap();
// 	}
// 	next_queue
// 	// // let locked_queue = next_queue.lock().unwrap();
// 	// if !next_queue.lock().unwrap().is_empty() {
// 	// 	process_neighbours2(int_field, next_queue, cost_field);
// 	// }
// }

// fn process_neighbour(
// 	cost_field: &Arc<CostField>,
// 	n: &FieldCell,
// 	prev_int_cost: &Arc<u16>,
// 	int_field: &Arc<Mutex<IntegrationField>>,
// 	next_queue: &Arc<Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>>,
// ) {
// 	let cell_cost = cost_field.get_field_cell_value(*n);
// 	// ignore impassable cells
// 	if cell_cost != 255 {
// 		// don't overwrite an int cell with a better cost
// 		let int_cost = cell_cost as u16 + **prev_int_cost;
// 		let mut locked_int_field = int_field.lock().unwrap();
// 		if int_cost < locked_int_field.get_field_cell_value(*n) {
// 			locked_int_field.set_field_cell_value(int_cost, *n);
// 			let mut locked_next_queue = next_queue.lock().unwrap();
// 			locked_next_queue.push((Arc::new(*n), Arc::new(int_cost)));
// 		}
// 	}
// }

// fn process_neighbours2(
// 	int_field: Arc<Mutex<IntegrationField>>,
// 	queue: Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>,
// 	cost_field: Arc<CostField>,
// ) -> Arc<Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>> {
// 	let next_queue: Arc<Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>> = Arc::new(Mutex::new(vec![]));
// 	let queue_values = queue.lock().unwrap();

// 	let mut handles = vec![];
// 	for (cell, prev_int_cost) in queue_values.iter() {
// 		let cost_field = Arc::clone(&cost_field);
// 		let int_field = Arc::clone(&int_field);
// 		let cell = Arc::clone(cell);
// 		let prev_int_cost = Arc::clone(prev_int_cost);
// 		let next_queue = Arc::clone(&next_queue);

// 		let handle = std::thread::spawn(move || {
// 			let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
// 			// iterate over the neighbours calculating int costs
// 			for n in neighbours.iter() {
// 				let cell_cost = cost_field.get_field_cell_value(*n);
// 				// ignore impassable cells
// 				if cell_cost != 255 {
// 					// don't overwrite an int cell with a better cost
// 					let int_cost = cell_cost as u16 + *prev_int_cost;
// 					let mut locked_int_field = int_field.lock().unwrap();
// 					if int_cost < locked_int_field.get_field_cell_value(*n) {
// 						locked_int_field.set_field_cell_value(int_cost, *n);
// 						let mut locked_next_queue = next_queue.lock().unwrap();
// 						locked_next_queue.push((Arc::new(*n), Arc::new(int_cost)));
// 					}
// 				}
// 			}
// 		});
// 		handles.push(handle);
// 	}
// 	for h in handles {
// 		h.join().unwrap();
// 	}
// 	next_queue
// 	// // let locked_queue = next_queue.lock().unwrap();
// 	// if !next_queue.lock().unwrap().is_empty() {
// 	// 	process_neighbours2(int_field, next_queue, cost_field);
// 	// }
// }

// fn process_neighbours3(
// 	int_field: Arc<Mutex<IntegrationField>>,
// 	queue: Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>,
// 	cost_field: Arc<CostField>,
// ) -> Arc<Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>> {
// 	let next_queue: Arc<Mutex<Vec<(Arc<FieldCell>, Arc<u16>)>>> = Arc::new(Mutex::new(vec![]));
// 	let queue_values = queue.lock().unwrap();
// 	let mut handles = vec![];
// 	for (cell, prev_int_cost) in queue_values.iter() {
// 		let cost_field = Arc::clone(&cost_field);
// 		let int_field = Arc::clone(&int_field);
// 		let cell = Arc::clone(cell);
// 		let prev_int_cost = Arc::clone(prev_int_cost);
// 		let next_queue = Arc::clone(&next_queue);

// 		let handle = std::thread::spawn(move || {
// 			let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
// 			let neigh: Vec<Arc<FieldCell>> = neighbours.iter().map(|&n| Arc::new(n)).collect();
// 			// iterate over the neighbours calculating int costs
// 			let mut handles = vec![];
// 			for n in neigh.iter() {
// 				let n = Arc::clone(n);
// 				let cf = Arc::clone(&cost_field);
// 				let prev_int_cost = Arc::clone(&prev_int_cost);
// 				let int_field = Arc::clone(&int_field);
// 				let next_queue = Arc::clone(&next_queue);

// 				let handle = std::thread::spawn(move || {
// 					let cell_cost = cf.get_field_cell_value(*n);
// 					// ignore impassable cells
// 					if cell_cost != 255 {
// 						// don't overwrite an int cell with a better cost
// 						let int_cost = cell_cost as u16 + *prev_int_cost;
// 						let mut locked_int_field = int_field.lock().unwrap();
// 						if int_cost < locked_int_field.get_field_cell_value(*n) {
// 							locked_int_field.set_field_cell_value(int_cost, *n);
// 							let mut locked_next_queue = next_queue.lock().unwrap();
// 							locked_next_queue.push((Arc::new(*n), Arc::new(int_cost)));
// 						}
// 					}
// 				});
// 				handles.push(handle);
// 			}
// 			for h in handles {
// 				h.join().unwrap();
// 			}
// 		});
// 		handles.push(handle);
// 	}
// 	for h in handles {
// 		h.join().unwrap();
// 	}
// 	next_queue
// 	// let locked_queue = next_queue.lock().unwrap();
// 	// if !next_queue.lock().unwrap().is_empty() {
// 	// 	process_neighbours3(next_queue, cost_field);
// 	// }
// }

// impl IntegrationField {
// 	pub fn channel_calculate_field(&mut self, goals: &[FieldCell], cost_field: &CostField) {
// 		// further positions to process, tuple element 0 is the position, element 1 is the integration cost from the previous cell needed to help calculate element 0s cost
// 		let mut queue: Vec<(FieldCell, u16)> = Vec::new();
// 		for goal in goals.iter() {
// 			queue.push(((*goal), self.get_field_cell_value(*goal)));
// 		}
// 		process_neighbours(self, queue, cost_field);
// 	}
// }

// fn process_neighbours_channel(
// 	int_field: &mut IntegrationField,
// 	queue: Vec<(FieldCell, u16)>,
// 	cost_field: &CostField,
// ) {
// 	// let (tx_queue, rx_queue) = mpsc::channel();
// 	// let (tx_int, rx_int) = mpsc::channel();

// 	let mut next_queue = Vec::new();
// 	// iterate over the queue calculating neighbour int costs
// 	for (cell, prev_int_cost) in queue.iter() {
// 		let neighbours = Ordinal::get_orthogonal_cell_neighbours(*cell);
// 		// iterate over the neighbours calculating int costs
// 		for n in neighbours.iter() {
// 			let cell_cost = cost_field.get_field_cell_value(*n);
// 			// ignore impassable cells
// 			if cell_cost != 255 {
// 				// don't overwrite an int cell with a better cost
// 				let int_cost = cell_cost as u16 + prev_int_cost;
// 				if int_cost < int_field.get_field_cell_value(*n) {
// 					int_field.set_field_cell_value(int_cost, *n);
// 					next_queue.push((*n, int_cost));
// 				}
// 			}
// 		}
// 	}
// 	if !next_queue.is_empty() {
// 		process_neighbours(int_field, next_queue, cost_field);
// 	}
// }

#[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	/// Calculate integration field from a uniform cost field with a source near the centre
	#[test]
	fn basic_field() {
		let cost_field = CostField::default();
		let mut integration_field = IntegrationField::default();
		let source = vec![FieldCell::new(4, 4)];
		integration_field.reset(&source);
		integration_field.calculate_field(&source, &cost_field);
		let result = integration_field.get();

		let actual: [[u16; FIELD_RESOLUTION]; FIELD_RESOLUTION] = [
			[8,7,6,5,4,5,6,7,8,9], [7,6,5,4,3,4,5,6,7,8], [6,5,4,3,2,3,4,5,6,7], [5,4,3,2,1,2,3,4,5,6], [4,3,2,1,0,1,2,3,4,5], [5,4,3,2,1,2,3,4,5,6], [6,5,4,3,2,3,4,5,6,7], [7,6,5,4,3,4,5,6,7,8], [8,7,6,5,4,5,6,7,8,9], [9,8,7,6,5,6,7,8,9,10]
		];


		assert_eq!(actual, *result);
	}
	/// Calculate integration field from a custom cost field set
	#[test]
	fn complex_field() {
		let mut cost_field = CostField::default();
		cost_field.set_field_cell_value(255, FieldCell::new(5, 6));
		cost_field.set_field_cell_value(255, FieldCell::new(5, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 9));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 8));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 7));
		cost_field.set_field_cell_value(255, FieldCell::new(6, 4));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 9));
		cost_field.set_field_cell_value(255, FieldCell::new(7, 4));
		cost_field.set_field_cell_value(255, FieldCell::new(8, 4));
		cost_field.set_field_cell_value(255, FieldCell::new(9, 4));
		cost_field.set_field_cell_value(255, FieldCell::new(1, 2));
		cost_field.set_field_cell_value(255, FieldCell::new(1, 1));
		cost_field.set_field_cell_value(255, FieldCell::new(2, 1));
		cost_field.set_field_cell_value(255, FieldCell::new(2, 2));
		let mut integration_field = IntegrationField::default();
		let source = vec![FieldCell::new(4, 4)];
		integration_field.reset(&source);
		integration_field.calculate_field(&source, &cost_field);
		let result = integration_field.get();

		let actual: [[u16; FIELD_RESOLUTION]; FIELD_RESOLUTION] = [
			[8,7,6,5,4,5,6,7,8,9], [7,65535,65535,4,3,4,5,6,7,8], [6,65535,65535,3,2,3,4,5,6,7], [5,4,3,2,1,2,3,4,5,6], [4,3,2,1,0,1,2,3,4,5], [5,4,3,2,1,2,65535,65535,5,6], [6,5,4,3,65535,3,4,65535,65535,65535], [7,6,5,4,65535,4,5,6,7,65535], [8,7,6,5,65535,5,6,7,8,9], [9,8,7,6,65535,6,7,8,9,10]
		];
		assert_eq!(actual, *result);
	}
}
