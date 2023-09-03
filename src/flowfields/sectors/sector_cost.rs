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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Clone, Default)]
pub struct SectorCostFields(BTreeMap<SectorID, CostField>);

impl SectorCostFields {
	/// Create a new instance of [SectorCostFields] based on the map dimensions containing [CostField]
	pub fn new(map_x_dimension: u32, map_z_dimension: u32, sector_resolution: u32) -> Self {
		let mut map = BTreeMap::new();
		let column_count = map_x_dimension / sector_resolution;
		let row_count = map_z_dimension / sector_resolution;
		for m in 0..column_count {
			for n in 0..row_count {
				map.insert(SectorID::new(m, n), CostField::default());
			}
		}
		SectorCostFields(map)
	}
	/// Get a reference to the map of sectors and [CostField]
	pub fn get(&self) -> &BTreeMap<SectorID, CostField> {
		&self.0
	}
	/// Get a mutable reference to the map of sectors and [CostField]
	pub fn get_mut(&mut self) -> &mut BTreeMap<SectorID, CostField> {
		&mut self.0
	}
	/// Update a cost within a particular `sector_id`
	pub fn set_field_cell_value(&mut self, sector_id: SectorID, value: u8, field_cell: FieldCell) {
		if let Some(cost_field) = self.get_mut().get_mut(&sector_id) {
			cost_field.set_field_cell_value(value, field_cell);
		}
	}
	/// From a `ron` file generate the [SectorCostFields]
	#[cfg(feature = "ron")]
	pub fn from_ron(path: String) -> Self {
		let file = std::fs::File::open(path).expect("Failed opening CostField file");
		let fields: SectorCostFields = match ron::de::from_reader(file) {
			Ok(fields) => fields,
			Err(e) => panic!("Failed deserializing SectorCostFields: {}", e),
		};
		//TODO handle scaling
		fields
	}
	/// From a directory containing a series of CSV files generate the [SectorCostFields]
	#[cfg(feature = "csv")]
	pub fn from_csv_dir(
		//TODO scaling
		map_length: u32,
		map_depth: u32,
		sector_resolution: u32,
		directory: String,
	) -> Self {
		let required_files_count =
			(map_length * map_depth) as usize / (sector_resolution * sector_resolution) as usize;
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
			sector_cost_fields.get_mut().insert(*sector_id, cost_field);
		}
		sector_cost_fields
	}
}

/// Keys represent unique sector IDs and are in the format of `(column, row)`
/// when considering a grid of sectors across the map. The sectors begin in the
/// top left of the map ((-x_max, -z_max) for 3d, (-x_max, y_max) for 2d)
/// and values are the [CostField] associated with that sector
///
/// Each [FieldCell] containing an impassable `255` value is scaled based on actor size to close off gaps which the actor could not path through
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Clone, Default)]
pub struct SectorCostFieldsScaled(BTreeMap<SectorID, CostField>);

impl SectorCostFieldsScaled {
	/// Create a new instance of [SectorCostFields] based on the map dimensions containing [CostField]
	pub fn new(sector_cost_fields: &SectorCostFields, actor_scale: u32) -> Self {
		let mut map = SectorCostFieldsScaled::default();
		let duplicate = sector_cost_fields.clone();
		for (sector, field) in duplicate.get().iter() {
			if actor_scale > 1 {
				scale_costfields(&mut map, sector_cost_fields, sector, field, actor_scale);
			} else {
				map.get_mut().insert(*sector, field.clone());
			}
		}
		map
	}
	/// Get a reference to the map of scaled sectors and [CostField]
	pub fn get(&self) -> &BTreeMap<SectorID, CostField> {
		&self.0
	}
	/// Get a mutable reference to the map of scaled sectors and [CostField]
	pub fn get_mut(&mut self) -> &mut BTreeMap<SectorID, CostField> {
		&mut self.0
	}
	/// From an adjusted [CostField] recalculate the actor size based scaling
	/// of impassable cells across the sector
	pub fn update(&mut self, sector_cost_fields: &SectorCostFields, sector_id: &SectorID, field: &CostField, actor_scale: u32) {
		if actor_scale > 1 {
			scale_costfields(self, sector_cost_fields, sector_id, field, actor_scale);
		} else {
			self.get_mut().insert(*sector_id, field.clone());
		}
	}
}

/// Inspects a sector for impassable cost values and based on an actor scale it expands any impassable costs into any neighbouring [FieldCell] to close off any gaps so that the actor won't try and path through a gap it can't fit
fn scale_costfields(scaled: &mut SectorCostFieldsScaled, baseline: &SectorCostFields, sector_id: &SectorID, field: &CostField, actor_scale: u32) {}


/// Inspects a sector for impassable cost values and based on an actor scale it expands any impassable costs into any neighbouring [FieldCell] to close off any gaps so that the actor won't try and path through a gap it can't fit
fn scale_costfields_mut(scaled: &mut SectorCostFieldsScaled, baseline: &mut SectorCostFields, sector_id: &SectorID, field: &CostField, actor_scale: u32) {}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	#[cfg(feature = "ron")]
	fn sector_cost_fields_file() {
		let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
		let _cost_fields = SectorCostFields::from_ron(path);
	}
}
