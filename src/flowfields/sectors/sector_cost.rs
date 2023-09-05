//! A map is split into a series of `MxN` sectors where each has a [CostField]
//! associated with it
//!
//!

use std::collections::BTreeMap;

use crate::prelude::*;
use bevy::prelude::*;

/// Keys represent unique sector IDs and are in the format of `(column, row)`
/// when considering a grid of sectors across the map. The sectors begin in the
/// top left of the map ((-x_max, -z_max) for 3d, (-x_max, y_max) for 2d)
/// and values are the [CostField] associated with that sector
#[cfg_attr(
	feature = "serde",
	derive(serde::Deserialize, serde::Serialize),
	serde(default)
)]
#[derive(Component, Clone, Default)]
pub struct SectorCostFields {
	/// Initial costs based on the unit size of each field
	baseline: BTreeMap<SectorID, CostField>,
	/// Each [FieldCell] containing an impassable `255` value is scaled based on actor size to close off gaps which the actor could not path through
	scaled: BTreeMap<SectorID, CostField>,
}

impl SectorCostFields {
	/// Create a new instance of [SectorCostFields] based on the map dimensions containing [CostField]
	pub fn new(map_dimensions: &MapDimensions) -> Self {
		let mut sector_cost_fields = SectorCostFields::default();
		let column_count = map_dimensions.get_length() / map_dimensions.get_sector_resolution();
		let row_count = map_dimensions.get_depth() / map_dimensions.get_sector_resolution();
		for m in 0..column_count {
			for n in 0..row_count {
				sector_cost_fields
					.baseline
					.insert(SectorID::new(m, n), CostField::default());
			}
		}
		sector_cost_fields.scale_all_costfields(map_dimensions);
		sector_cost_fields
	}
	/// Get a reference to the map of the baseline sectors and [CostField]
	pub fn get_baseline(&self) -> &BTreeMap<SectorID, CostField> {
		&self.baseline
	}
	/// Get a mutable reference to the map of the baseline sectors and [CostField]
	pub fn get_baseline_mut(&mut self) -> &mut BTreeMap<SectorID, CostField> {
		&mut self.baseline
	}
	/// Get a reference to the map of scaled sectors and [CostField]
	pub fn get_scaled(&self) -> &BTreeMap<SectorID, CostField> {
		&self.scaled
	}
	/// Get a mutable reference to the map of scaled sectors and [CostField]
	pub fn get_scaled_mut(&mut self) -> &mut BTreeMap<SectorID, CostField> {
		&mut self.scaled
	}
	/// Update a cost within a particular `sector_id`. This in turn will update the scaled field based on `actor_scale`
	pub fn set_field_cell_value(
		&mut self,
		sector_id: SectorID,
		value: u8,
		field_cell: FieldCell,
		map_dimensions: &MapDimensions,
	) {
		if let Some(cost_field) = self.get_baseline_mut().get_mut(&sector_id) {
			cost_field.set_field_cell_value(value, field_cell);
			self.scale_costfield(&sector_id, map_dimensions)
		} else {
			error!(
				"Cannot mutate CostField in non-existent sector {:?}",
				sector_id
			);
		}
	}
	/// Iterate over all sectors and scale any impassable [FieldCell] based on `actor_scale`.
	///
	/// This can be expensive so should typically be used as part of data initialisation, i.e when loading [SectorCostFields] from a file or within a loading type of operation to a world
	fn scale_all_costfields(&mut self, map_dimensions: &MapDimensions) {
		let sector_ids: Vec<SectorID> = self.baseline.keys().cloned().collect();
		//TODO multithread?
		for sector_id in sector_ids.iter() {
			self.scaled.insert(
				*sector_id,
				self.get_baseline().get(sector_id).unwrap().clone(),
			);
		}
		for sector_id in sector_ids.iter() {
			self.scale_costfield(sector_id, map_dimensions);
		}
	}
	/// Inspects a sector for impassable cost values and based on an actor scale it expands any impassable costs into any neighbouring [FieldCell] to close off any gaps so that the actor won't try and path through a gap it can't fit
	fn scale_costfield(&mut self, sector_id: &SectorID, map_dimensions: &MapDimensions) {
		/// Helper updates a tracker of what cells/sectors have been processed
		fn update_processed(
			processed: &mut BTreeMap<SectorID, Vec<FieldCell>>,
			field_cell: FieldCell,
			sector_id: &SectorID,
		) {
			if let Some(list) = processed.get_mut(sector_id) {
				list.push(field_cell);
			} else {
				processed.insert(*sector_id, vec![field_cell]);
			}
		}
		/// Helper that adds tracking data of what final cells to update in the scaled fields
		fn add_to_be_marked(
			marks_as_impassable: &mut BTreeMap<SectorID, Vec<FieldCell>>,
			processed: &BTreeMap<SectorID, Vec<FieldCell>>,
		) {
			for (sector, cell_list) in processed.iter() {
				if let Some(list) = marks_as_impassable.get_mut(sector) {
					list.extend(cell_list);
				} else {
					marks_as_impassable.insert(*sector, cell_list.clone());
				}
			}
		}

		if map_dimensions.get_actor_scale() == 1 {
			//TODO handle adjacent diagnonals where movement should be blocked?
			self.scaled.insert(
				*sector_id,
				self.get_baseline().get(sector_id).unwrap().clone(),
			);
		} else {
			//TODO init scaling to close edge case gaps?????
			// identify all impassable cells
			let mut impassable_indices = Vec::new();
			let cost_field = self.get_baseline_mut().get(sector_id).unwrap();
			let field_array = cost_field.get();
			for (column, rows) in field_array.iter().enumerate() {
				for (row, cost) in rows.iter().enumerate() {
					if *cost == 255 {
						impassable_indices.push((column, row));
					}
				}
			}
			// For each impassable cell walk along the neighbouring ordinals
			// and close any small gaps in the scaled field
			let mut marks_as_impassable: BTreeMap<SectorID, Vec<FieldCell>> = BTreeMap::new();
			for (column, row) in impassable_indices.iter() {
				// North
				let mut processed: BTreeMap<SectorID, Vec<FieldCell>> = BTreeMap::new();
				'ord: for i in 1..=map_dimensions.get_actor_scale() as usize {
					if let Some(n_row) = row.checked_sub(i) {
						let field_cell = FieldCell::new(*column, n_row);
						update_processed(&mut processed, field_cell, sector_id);
						let value = self
							.get_baseline()
							.get(sector_id)
							.unwrap()
							.get_field_cell_value(field_cell);
						// hit impassable before exceeding scale therefore
						// gap too small for pathing
						if value == 255 {
							add_to_be_marked(&mut marks_as_impassable, &processed);
							// marks_as_impassable.extend(&processed);
							break 'ord;
						}
					} else {
						// check next northerly sector if not on a boundary sector
						// and actor size would straddle over a sector edge
						if let Some(n_sector) =
							map_dimensions.get_sector_id_from_ordinal(Ordinal::North, sector_id)
						{
							//TODO how to handle an actor that spands more then just the next sector?
							// adjust sizing to step through neightbour sector
							for x in 0..=map_dimensions.get_actor_scale() as usize - i {
								if let Some(n_row) = 9_usize.checked_sub(x) {
									let field_cell = FieldCell::new(*column, n_row);
									update_processed(&mut processed, field_cell, &n_sector);
									let value = self
										.get_baseline()
										.get(&n_sector)
										.unwrap()
										.get_field_cell_value(field_cell);
									// hit impassable before exceeding scale therefore
									// gap too small for pathing
									if value == 255 {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									}
								}
							}
						} else {
							// hit the edge of the world so actor can't fit through the gap
							add_to_be_marked(&mut marks_as_impassable, &processed);
							break 'ord;
						}
					}
				}
				processed.clear();
				// East
				'ord: for i in 1..=map_dimensions.get_actor_scale() as usize {
					if column + i < FIELD_RESOLUTION {
						let field_cell = FieldCell::new(column + i, *row);
						update_processed(&mut processed, field_cell, sector_id);
						let value = self
							.get_baseline()
							.get(sector_id)
							.unwrap()
							.get_field_cell_value(field_cell);
						// hit impassable before exceeding scale therefore
						// gap too small for pathing
						if value == 255 {
							add_to_be_marked(&mut marks_as_impassable, &processed);
							// marks_as_impassable.extend(&processed);
							break 'ord;
						}
					} else {
						// check next easterly sector if not on a boundary sector
						// and actor size would straddle over a sector edge
						if let Some(n_sector) =
							map_dimensions.get_sector_id_from_ordinal(Ordinal::East, sector_id)
						{
							//TODO how to handle an actor that spands more then just the next sector?
							// adjust sizing to step through neightbour sector
							for x in 0..=map_dimensions.get_actor_scale() as usize - i {
								if x < FIELD_RESOLUTION {
									let field_cell = FieldCell::new(x, *row);
									update_processed(&mut processed, field_cell, &n_sector);
									let value = self
										.get_baseline()
										.get(&n_sector)
										.unwrap()
										.get_field_cell_value(field_cell);
									// hit impassable before exceeding scale therefore
									// gap too small for pathing
									if value == 255 {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									}
								}
							}
						} else {
							// hit the edge of the world so actor can't fit through the gap
							add_to_be_marked(&mut marks_as_impassable, &processed);
							break 'ord;
						}
					}
				}
				processed.clear();
				// South
				'ord: for i in 1..=map_dimensions.get_actor_scale() as usize {
					if row + i < FIELD_RESOLUTION {
						let field_cell = FieldCell::new(*column, *row + i);
						update_processed(&mut processed, field_cell, sector_id);
						let value = self
							.get_baseline()
							.get(sector_id)
							.unwrap()
							.get_field_cell_value(field_cell);
						// hit impassable before exceeding scale therefore
						// gap too small for pathing
						if value == 255 {
							add_to_be_marked(&mut marks_as_impassable, &processed);
							// marks_as_impassable.extend(&processed);
							break 'ord;
						}
					} else {
						// check next southerly sector if not on a boundary sector
						// and actor size would straddle over a sector edge
						if let Some(n_sector) =
							map_dimensions.get_sector_id_from_ordinal(Ordinal::South, sector_id)
						{
							//TODO how to handle an actor that spands more then just the next sector?
							// adjust sizing to step through neightbour sector
							for x in 0..=map_dimensions.get_actor_scale() as usize - i {
								if x < FIELD_RESOLUTION {
									let field_cell = FieldCell::new(*column, x);
									update_processed(&mut processed, field_cell, &n_sector);
									let value = self
										.get_baseline()
										.get(&n_sector)
										.unwrap()
										.get_field_cell_value(field_cell);
									// hit impassable before exceeding scale therefore
									// gap too small for pathing
									if value == 255 {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									}
								}
							}
						} else {
							// hit the edge of the world so actor can't fit through the gap
							add_to_be_marked(&mut marks_as_impassable, &processed);
							break 'ord;
						}
					}
				}
				processed.clear();
				// West
				'ord: for i in 1..=map_dimensions.get_actor_scale() as usize {
					if let Some(n_column) = column.checked_sub(i) {
						let field_cell = FieldCell::new(n_column, *row);
						update_processed(&mut processed, field_cell, sector_id);
						let value = self
							.get_baseline()
							.get(sector_id)
							.unwrap()
							.get_field_cell_value(field_cell);
						// hit impassable before exceeding scale therefore
						// gap too small for pathing
						if value == 255 {
							add_to_be_marked(&mut marks_as_impassable, &processed);
							// marks_as_impassable.extend(&processed);
							break 'ord;
						}
					} else {
						// check next westerly sector if not on a boundary sector
						// and actor size would straddle over a sector edge
						if let Some(n_sector) =
							map_dimensions.get_sector_id_from_ordinal(Ordinal::North, sector_id)
						{
							//TODO how to handle an actor that spands more then just the next sector?
							// adjust sizing to step through neightbour sector
							for x in 0..=map_dimensions.get_actor_scale() as usize - i {
								if let Some(n_column) = 9_usize.checked_sub(x) {
									let field_cell = FieldCell::new(n_column, *row);
									update_processed(&mut processed, field_cell, &n_sector);
									let value = self
										.get_baseline()
										.get(&n_sector)
										.unwrap()
										.get_field_cell_value(field_cell);
									// hit impassable before exceeding scale therefore
									// gap too small for pathing
									if value == 255 {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									}
								}
							}
						} else {
							// hit the edge of the world so actor can't fit through the gap
							add_to_be_marked(&mut marks_as_impassable, &processed);
							break 'ord;
						}
					}
				}
				processed.clear();
			}
			// mark any cells
			for (sector, cells) in marks_as_impassable.iter() {
				for cell in cells.iter() {
					self.get_scaled_mut()
						.get_mut(sector)
						.unwrap()
						.set_field_cell_value(255, *cell)
				}
			}
		}
	}
	/// From a `ron` file generate the [SectorCostFields]
	#[cfg(feature = "ron")]
	pub fn from_ron(path: String, map_dimensions: &MapDimensions) -> Self {
		let file = std::fs::File::open(path).expect("Failed opening CostField file");
		let mut fields: SectorCostFields = match ron::de::from_reader(file) {
			Ok(fields) => fields,
			Err(e) => panic!("Failed deserializing SectorCostFields: {}", e),
		};
		fields.scale_all_costfields(map_dimensions);
		fields
	}
	/// From a directory containing a series of CSV files generate the [SectorCostFields]
	#[cfg(feature = "csv")]
	pub fn from_csv_dir(map_dimensions: &MapDimensions, directory: String) -> Self {
		let required_files_count = (map_dimensions.get_length() * map_dimensions.get_depth())
			as usize / (map_dimensions.get_sector_resolution().pow(2))
			as usize;
		let files = std::fs::read_dir(directory)
			.expect("Unable to read csv directory")
			.map(|res| {
				res.map(|e| {
					(
						e.path().into_os_string().into_string().unwrap(),
						e.file_name().into_string().unwrap(),
					)
				})
			})
			.collect::<Result<Vec<_>, std::io::Error>>()
			.expect("Failed to filter for CSV files");
		let mut csvs = Vec::new();
		for (file_path, file_name) in files {
			if file_path.ends_with(".csv") {
				let sector_id_str = file_name.trim_end_matches(".csv").split_once('_').unwrap();
				let sector_id = SectorID::new(
					sector_id_str
						.0
						.parse::<u32>()
						.expect("Failed to parse sector ID from csv file name"),
					sector_id_str
						.1
						.parse::<u32>()
						.expect("Failed to parse sector ID from csv file name"),
				);
				csvs.push((file_path, sector_id));
			}
		}
		if csvs.len() != required_files_count {
			panic!(
				"Found {} CSVs, expected {}",
				csvs.len(),
				required_files_count
			);
		}
		let mut sector_cost_fields = SectorCostFields::default();
		for (csv_file, sector_id) in csvs.iter() {
			let data = std::fs::File::open(csv_file).expect("Failed opening csv");
			let mut rdr = csv::ReaderBuilder::new()
				.has_headers(false)
				.from_reader(data);
			let mut cost_field = CostField::default();
			for (row, record) in rdr.records().enumerate() {
				for (column, value) in record.unwrap().iter().enumerate() {
					let value_u8: u8 = value.parse().expect("CSV expects u8 values");
					cost_field.set_field_cell_value(value_u8, FieldCell::new(column, row));
				}
			}
			sector_cost_fields
				.get_baseline_mut()
				.insert(*sector_id, cost_field);
		}
		sector_cost_fields.scale_all_costfields(map_dimensions);
		sector_cost_fields
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	#[cfg(feature = "ron")]
	fn sector_cost_fields_file() {
		let map_dimensions = MapDimensions::new(1920, 1920, 640, 16.0);
		let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
		let _cost_fields = SectorCostFields::from_ron(path, &map_dimensions);
	}
	#[test]
	fn scale_north_one() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 0.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(3, 1);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(3, 2);
		let actual = 1;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_north_two_closed() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(3, 1);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(3, 2);
		let actual = 255;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_north_two_open() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(3, 0);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(3, 2);
		let actual = 1;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
		let inspect_field = FieldCell::new(3, 1);
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_east_one() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 0.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(5, 3);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(4, 3);
		let actual = 1;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_east_two_closed() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(5, 3);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(4, 3);
		let actual = 255;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_east_two_open() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(6, 3);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(4, 3);
		let actual = 1;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
		let inspect_field = FieldCell::new(5, 3);
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_south_one() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 0.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(3, 5);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(3, 4);
		let actual = 1;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_south_two_closed() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(3, 5);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(3, 4);
		let actual = 255;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_south_two_open() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(3, 6);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(3, 4);
		let actual = 1;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
		let inspect_field = FieldCell::new(3, 5);
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_west_one() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 0.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(1, 3);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(2, 3);
		let actual = 1;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_west_two_closed() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(5, 3);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(4, 3);
		let actual = 255;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_west_two_open() {
		let map_dimensions = MapDimensions::new(10, 10, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let value = 255;
		// gap between impassables
		let field_first = FieldCell::new(3, 3);
		let field_second = FieldCell::new(0, 3);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(2, 3);
		let actual = 1;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
		let inspect_field = FieldCell::new(1, 3);
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_north_cross_coundary() {
		let map_dimensions = MapDimensions::new(20, 20, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		// gap between impassables
		let value = 255;
		let sector_id = SectorID::new(0, 1);
		let field_first = FieldCell::new(3, 0);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		let sector_id = SectorID::new(0, 0);
		let field_second = FieldCell::new(3, 8);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(3, 9);
		let actual = 255;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
	#[test]
	fn scale_east_cross_coundary() {
		let map_dimensions = MapDimensions::new(20, 20, 10, 1.5);
		let mut cost_fields = SectorCostFields::new(&map_dimensions);
		// gap between impassables
		let value = 255;
		let sector_id = SectorID::new(0, 0);
		let field_first = FieldCell::new(9, 4);
		cost_fields.set_field_cell_value(sector_id, value, field_first, &map_dimensions);
		let sector_id = SectorID::new(1, 0);
		let field_second = FieldCell::new(1, 4);
		cost_fields.set_field_cell_value(sector_id, value, field_second, &map_dimensions);
		// gap shouldn't be filled in
		let inspect_field = FieldCell::new(0, 4);
		let actual = 255;
		let result = cost_fields
			.get_scaled()
			.get(&sector_id)
			.unwrap()
			.get_field_cell_value(inspect_field);
		assert_eq!(actual, result);
	}
}
