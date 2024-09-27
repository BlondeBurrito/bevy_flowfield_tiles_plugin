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
	/// Create a new instance of [SectorCostFields] based on the map dimensions where the supplied `cost` is used as the default value in all [CostField]
	fn new_with_cost(map_dimensions: &MapDimensions, cost: u8) -> Self {
		let mut sector_cost_fields = SectorCostFields::default();
		let column_count = map_dimensions.get_length() / map_dimensions.get_sector_resolution();
		let row_count = map_dimensions.get_depth() / map_dimensions.get_sector_resolution();
		for m in 0..column_count {
			for n in 0..row_count {
				sector_cost_fields
					.baseline
					.insert(SectorID::new(m, n), CostField::new_with_cost(cost));
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
	// /// Get the [CostField] of a sector wrapped in in Arc
	// pub fn get_arc_scaled_sector(&self, sector_id: &SectorID) -> Arc<CostField> {
	// 	//TODO really a clone?
	// 	Arc::new(self.get_scaled().get(sector_id).unwrap().clone())
	// }
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
	pub fn scale_all_costfields(&mut self, map_dimensions: &MapDimensions) {
		let sector_ids: Vec<SectorID> = self.baseline.keys().cloned().collect();
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
	/// Inspects a sector for impassable cost values and based on an actor
	/// scale it expands any impassable costs into any neighbouring orthogonal
	/// [FieldCell] to close off any gaps so that the actor won't try and path
	/// through a gap it can't fit
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
			self.scaled.insert(
				*sector_id,
				self.get_baseline().get(sector_id).unwrap().clone(),
			);
		} else {
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
						// based on actor scale figure out how many possible neighbouring sectors might need to be explored
						let neighbours = {
							let mut n = Vec::new();
							for factor in
								0..=(map_dimensions.get_actor_scale() / FIELD_RESOLUTION as u32)
							{
								if let Some(n_sector) = map_dimensions.get_sector_id_from_ordinal(
									Ordinal::North,
									&SectorID::new(
										sector_id.get_column(),
										sector_id.get_row() - factor,
									),
								) {
									n.push(n_sector);
								}
							}
							n
						};
						if neighbours.is_empty() {
							// hit the edge of the world so actor can't fit through the gap
							add_to_be_marked(&mut marks_as_impassable, &processed);
							break 'ord;
						}
						// work through neighbours to see if a big enough gap exists
						for (count, n_sector) in neighbours.iter().enumerate() {
							// adjust sizing to step through neightbour sector
							// uses field resolution to account for previously
							// covered sectors in this list
							'inner: for x in 0..=map_dimensions.get_actor_scale() as usize
								- i - (count * FIELD_RESOLUTION)
							{
								if let Some(n_row) = 9_usize.checked_sub(x) {
									let field_cell = FieldCell::new(*column, n_row);
									update_processed(&mut processed, field_cell, n_sector);
									let value = self
										.get_baseline()
										.get(n_sector)
										.unwrap_or_else(|| panic!("Could not get baseline costfield {:?}, this can indicates that sector_resolution and/or actor_size are not set correctly", n_sector))
										.get_field_cell_value(field_cell);
									// hit impassable before exceeding scale therefore
									// gap too small for pathing
									if value == 255 {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									}
								} else {
									// hit the edge of the world while there's scale left to check
									// therefore actor cannot fit through the gap
									if count + 1 == neighbours.len() {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									} else {
										break 'inner;
									}
								}
							}
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
						// based on actor scale figure out how many possible neighbouring sectors might need to be explored
						let neighbours = {
							let mut n = Vec::new();
							for factor in
								0..=(map_dimensions.get_actor_scale() / FIELD_RESOLUTION as u32)
							{
								if let Some(n_sector) = map_dimensions.get_sector_id_from_ordinal(
									Ordinal::East,
									&SectorID::new(
										sector_id.get_column() + factor,
										sector_id.get_row(),
									),
								) {
									n.push(n_sector);
								}
							}
							n
						};
						if neighbours.is_empty() {
							// hit the edge of the world so actor can't fit through the gap
							add_to_be_marked(&mut marks_as_impassable, &processed);
							break 'ord;
						}
						// work through neighbours to see if a big enough gap exists
						for (count, n_sector) in neighbours.iter().enumerate() {
							// adjust sizing to step through neightbour sector
							// uses field resolution to account for previously
							// covered sectors in this list
							'inner: for x in 0..=map_dimensions.get_actor_scale() as usize
								- i - (count * FIELD_RESOLUTION)
							{
								if x < FIELD_RESOLUTION {
									let field_cell = FieldCell::new(x, *row);
									update_processed(&mut processed, field_cell, n_sector);
									let value = self
										.get_baseline()
										.get(n_sector)
										.unwrap()
										.get_field_cell_value(field_cell);
									// hit impassable before exceeding scale therefore
									// gap too small for pathing
									if value == 255 {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									}
								} else {
									// hit the edge of the world while there's scale left to check
									// therefore actor cannot fit through the gap
									if count + 1 == neighbours.len() {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									} else {
										break 'inner;
									}
								}
							}
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
						// based on actor scale figure out how many possible neighbouring sectors might need to be explored
						let neighbours = {
							let mut n = Vec::new();
							for factor in
								0..=(map_dimensions.get_actor_scale() / FIELD_RESOLUTION as u32)
							{
								if let Some(n_sector) = map_dimensions.get_sector_id_from_ordinal(
									Ordinal::South,
									&SectorID::new(
										sector_id.get_column(),
										sector_id.get_row() + factor,
									),
								) {
									n.push(n_sector);
								}
							}
							n
						};
						if neighbours.is_empty() {
							// hit the edge of the world so actor can't fit through the gap
							add_to_be_marked(&mut marks_as_impassable, &processed);
							break 'ord;
						}
						// work through neighbours to see if a big enough gap exists
						for (count, n_sector) in neighbours.iter().enumerate() {
							// adjust sizing to step through neightbour sector
							// uses field resolution to account for previously
							// covered sectors in this list
							'inner: for x in 0..=map_dimensions.get_actor_scale() as usize
								- i - (count * FIELD_RESOLUTION)
							{
								if x < FIELD_RESOLUTION {
									let field_cell = FieldCell::new(*column, x);
									update_processed(&mut processed, field_cell, n_sector);
									let value = self
										.get_baseline()
										.get(n_sector)
										.unwrap()
										.get_field_cell_value(field_cell);
									// hit impassable before exceeding scale therefore
									// gap too small for pathing
									if value == 255 {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									}
								} else {
									// hit the edge of the world while there's scale left to check
									// therefore actor cannot fit through the gap
									if count + 1 == neighbours.len() {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									} else {
										break 'inner;
									}
								}
							}
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
						// based on actor scale figure out how many possible neighbouring sectors might need to be explored
						let neighbours = {
							let mut n = Vec::new();
							for factor in
								0..=(map_dimensions.get_actor_scale() / FIELD_RESOLUTION as u32)
							{
								if let Some(n_sector) = map_dimensions.get_sector_id_from_ordinal(
									Ordinal::West,
									&SectorID::new(
										sector_id.get_column() - factor,
										sector_id.get_row(),
									),
								) {
									n.push(n_sector);
								}
							}
							n
						};
						if neighbours.is_empty() {
							// hit the edge of the world so actor can't fit through the gap
							add_to_be_marked(&mut marks_as_impassable, &processed);
							break 'ord;
						}
						// work through neighbours to see if a big enough gap exists
						for (count, n_sector) in neighbours.iter().enumerate() {
							// adjust sizing to step through neightbour sector
							// uses field resolution to account for previously
							// covered sectors in this list
							'inner: for x in 0..=map_dimensions.get_actor_scale() as usize
								- i - (count * FIELD_RESOLUTION)
							{
								if let Some(n_column) = 9_usize.checked_sub(x) {
									let field_cell = FieldCell::new(n_column, *row);
									update_processed(&mut processed, field_cell, n_sector);
									let value = self
										.get_baseline()
										.get(n_sector)
										.unwrap()
										.get_field_cell_value(field_cell);
									// hit impassable before exceeding scale therefore
									// gap too small for pathing
									if value == 255 {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									}
								} else {
									// hit the edge of the world while there's scale left to check
									// therefore actor cannot fit through the gap
									if count + 1 == neighbours.len() {
										add_to_be_marked(&mut marks_as_impassable, &processed);
										break 'ord;
									} else {
										break 'inner;
									}
								}
							}
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
	/// Create a [SectorCostFields] from a greyscale image where each pixel
	/// represents the cost of a [FieldCell]
	#[cfg(feature = "heightmap")]
	pub fn from_heightmap(map_dimensions: &MapDimensions, path: String) -> Self {
		use photon_rs::native::open_image;
		let img = open_image(&path).expect("Failed to open heightmap");
		let img_width = img.get_width();
		let img_height = img.get_height();
		// ensure the size of the heightmap actually represents the number of FieldCells required by the MapDimensions
		let hori_sector_count =
			map_dimensions.get_length() / map_dimensions.get_sector_resolution();
		let required_px_width = hori_sector_count * FIELD_RESOLUTION as u32;
		if img_width != required_px_width {
			panic!(
				"Heightmap has incorrect width, expected width of {} pixels, found {}",
				required_px_width, img_width
			);
		}
		let vert_sector_count = map_dimensions.get_depth() / map_dimensions.get_sector_resolution();
		let required_px_height = vert_sector_count * FIELD_RESOLUTION as u32;
		if img_height != required_px_height {
			panic!(
				"Heightmap has incorrect height, expected hieght of {} pixels, found {}",
				required_px_height, img_height
			);
		}
		// init the fields so we already have the required sectors inserted
		let mut sector_cost_fields = SectorCostFields::new(map_dimensions);
		// iter over the pixels in chunks creating CostFields
		let raw_pixels = img.get_raw_pixels();
		// raw pixels are arranged from the top left of the image and come in sets of either 3 or 4 (if alpha channel is inlcuded).
		// Each sequential set corresponds to Red, Green, Blue, (Alpha).
		// We want to convert these into a vector of tuples which can represent each field cell
		let len_if_alpha = img_height * img_height * 4;
		let chunk_size = {
			if len_if_alpha as usize == raw_pixels.len() {
				4
			} else {
				3
			}
		};
		let mut pixels_rgb: Vec<(u8, u8, u8)> = Vec::new();
		for rgb in raw_pixels.chunks(chunk_size) {
			let mut as_tuple = vec![(rgb[0], rgb[1], rgb[2])];
			pixels_rgb.append(&mut as_tuple);
		}
		// By chunking the list of pixel RGBAs based on the width of the image
		// we can iterate on the rows
		for (line_number, rgba_slice) in pixels_rgb.chunks(img_width as usize).enumerate() {
			let sector_row = line_number / FIELD_RESOLUTION;
			// chunk each row by resolution to give slices of pixels for each sector column
			for (sector_column, rgba_slice_slice) in rgba_slice.chunks(FIELD_RESOLUTION).enumerate()
			{
				let sector_id = SectorID::new(sector_column as u32, sector_row as u32);
				let field = sector_cost_fields
					.get_baseline_mut()
					.get_mut(&sector_id)
					.unwrap();
				// iter over the pixels in the row of the particular sector
				for (field_column, px) in rgba_slice_slice.iter().enumerate() {
					// calc row in the field
					let field_row = line_number - (FIELD_RESOLUTION * sector_row);
					let field_cell = FieldCell::new(field_column, field_row);
					// black (0, 0, 0, 255)
					// white (255, 255, 255, 255)
					// careful of u8 overflow
					let colour_avg = (px.0 as f32 + px.1 as f32 + px.2 as f32) / 3.0;
					let value = (255 - colour_avg as u8).clamp(1, 255);
					field.set_field_cell_value(value, field_cell);
				}
			}
		}
		// now that costs are popualated calcualte the scaled fields that will
		// be used in the algorithm
		sector_cost_fields.scale_all_costfields(map_dimensions);
		sector_cost_fields
	}
	/// From a list of meshes extract the outer edges of each mesh and project an (MxN) FieldCell representation of edges over the dimensions. The projections undergo two tests to see if a FieldCell sits inside a mesh (thereby being marked as pathable):
	/// - The top-right vertex of each field cell is tested for mesh edge intersections, a horizontal line is taken from the vertex point to max-x and if the line intersects mesh edges an odd number of times, or touches an edge an even number of times, then it is marked as potentially being within the mesh
	/// - From the marked FieldCells the four edges of each is then tested to see if it intersects any mesh edges, if so then it is overlapping a mesh boundary and so not fully inside the mesh, otherwise it is in the mesh and considered a pathable cell and given the cost `internal_cost` - all cells outside of the meshes are initialised with a cost of `external_cost`
	#[cfg(feature = "2d")]
	pub fn from_bevy_2d_meshes(
		map_dimensions: &MapDimensions,
		meshes: &Vec<(&Mesh, Vec2)>,
		internal_cost: u8,
		external_cost: u8,
	) -> Self {
		// init the fields so we already have the required sectors inserted
		let mut sector_cost_fields = SectorCostFields::new_with_cost(map_dimensions, external_cost);

		// Treat each FieldCell as its own polygon
		// to find if one polygon (A) is within another (B):
		// 1) Take a vertex of A (a corner of a FieldCell) and project a line
		// to the maximum x dimension - check to see if this line intersects
		// any of the edges of B (the supplied mesh).
		// If it intersects an even number of times (includes 0) then it is
		// outside polygon B.
		// If it intersects an odd number of times then it is a candiate and we
		// perform the next check
		// 2) Check each edge of A (FieldCell polygon) and see if any edges
		// intersect with the edges of B (the mesh). If an intersection is
		// found then the FieldCell overlaps the polygon and the FieldCell is
		// treated as impassable.
		// If no intersections are found then A is inside B.

		// store all mesh outer edges for field cell checks later
		let mut outer_edges = vec![];
		for (mesh, translation) in meshes {
			if let Some(mesh_vertices) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
				let vertex_points = mesh_vertices.as_float3().unwrap();
				// build each edge of each triangle in the mesh represented by index points
				let edge_indices = retrieve_mesh_edges(mesh, vertex_points);
				if !edge_indices.is_empty() {
					// collect edges that only belong to a single triangle (this means ignore internal edges, we only want the edges outlining the mesh), if any MeshEdge appears more than once we remove all occurances of it
					let copy = edge_indices.clone();
					for edge in edge_indices {
						let mut occurances = 0;
						for c in &copy {
							if edge == *c {
								occurances += 1;
							}
						}
						if occurances == 1 {
							// found outer edge
							// store edge line
							let start = vertex_points[edge.0];
							let end = vertex_points[edge.1];
							//NB: vertex points are relative to mesh so include
							// translation of the mesh to find global position
							let line = EdgeLine::build(
								Vec2::new(start[0] + translation.x, start[1] + translation.y),
								Vec2::new(end[0] + translation.x, end[1] + translation.y),
							);
							outer_edges.push(line);
						}
					}
				}
			}
		}
		// with the external edges of the mesh known we can
		// test to see if the field cell vertex intercepts any edge
		// an odd number of times to mark it as a candiate that
		// could be inside the mesh

		// convert FieldCell ID notation of origin top-left
		// into an f32 form where the origin is the center of
		// the world
		// iterate over all FieldCells, left to right, top to bottom

		// create a list of candiate row-col which are likely to be within the
		// mesh therefore pathable
		let mut candidates: Vec<(usize, usize)> =
			calc_field_cell_mesh_candidates(map_dimensions, &outer_edges);
		// to test whether an entire field cell is within the mesh we need to take each edge of the candidate field cells and test that none of them intersect with any mesh edges
		let failed_candidates: Vec<(usize, usize)> =
			identify_field_cells_that_intersect_mesh(map_dimensions, &candidates, &outer_edges);
		// from candidates and failed candidates identify the cells which are pathable
		for cell in failed_candidates.iter() {
			candidates.retain(|&c| c != *cell);
		}
		// candidates are now the pathable ones, determine how they are represented
		// in Sector and FieldCell notation to update the CostFields
		let field_cell_unit_size = map_dimensions.get_field_cell_unit_size();
		let offset_x = map_dimensions.get_length() as f32 / 2.0;
		let offset_y = map_dimensions.get_depth() as f32 / 2.0;
		for (row, col) in candidates {
			let x = col as f32 * field_cell_unit_size - offset_x + (field_cell_unit_size / 2.0);
			let y = row as f32 * -field_cell_unit_size + offset_y - (field_cell_unit_size / 2.0);
			let position = Vec2::new(x, y);
			if let Some((sector, field_cell)) =
				map_dimensions.get_sector_and_field_cell_from_xy(position)
			{
				sector_cost_fields.set_field_cell_value(
					sector,
					internal_cost,
					field_cell,
					map_dimensions,
				);
			}
		}
		sector_cost_fields.scale_all_costfields(map_dimensions);
		sector_cost_fields
	}
}
/// From a triple floating point representation of a mesh retreive a list of the edges as index pairs
fn retrieve_mesh_edges(mesh: &&Mesh, vertex_points: &[[f32; 3]]) -> Vec<MeshTriEdge<usize>> {
	use bevy::render::mesh::PrimitiveTopology;
	let indices = mesh.indices().unwrap();
	let indices_slice: Vec<usize> = indices.iter().collect();
	let mut edge_indices = vec![];
	match mesh.primitive_topology() {
		PrimitiveTopology::TriangleList => {
			for i in indices_slice.chunks(3) {
				edge_indices.push(MeshTriEdge(i[0], i[1]));
				edge_indices.push(MeshTriEdge(i[1], i[2]));
				edge_indices.push(MeshTriEdge(i[2], i[0]));
			}
		}
		PrimitiveTopology::TriangleStrip => {
			if let Some(triangle_count) = vertex_points.len().checked_sub(2) {
				for n in 0..triangle_count {
					if n % 2 == 0 {
						edge_indices.push(MeshTriEdge(indices_slice[n], indices_slice[n + 1]));
						edge_indices.push(MeshTriEdge(indices_slice[n + 1], indices_slice[n + 2]));
						edge_indices.push(MeshTriEdge(indices_slice[n + 2], indices_slice[n]));
					} else {
						edge_indices.push(MeshTriEdge(indices_slice[n + 1], indices_slice[n]));
						edge_indices.push(MeshTriEdge(indices_slice[n], indices_slice[n + 2]));
						edge_indices.push(MeshTriEdge(indices_slice[n + 2], indices_slice[n + 1]));
					}
				}
			} else {
				warn!("A TriangleStrip mesh has insufficient vertices");
			}
		}
		_ => {
			warn!("Mesh topology must be of TriangleList or TriangleStrip for use with Flowfields");
		}
	}
	edge_indices
}
/// Using a list of outer mesh edges iterate over every [FieldCell] and draw a horiontal line from the top-left position of a [FieldCell] box/square and count the number of times the line intersects an outer mesh edge. If the line intersects an edge an odd number of times then it means that the [FieldCell] is probably within the mesh. An even number of intersections means it passes into and out of the mesh and therefore must be a [FieldCell] that sits outside of the mesh edges
fn calc_field_cell_mesh_candidates(
	map_dimensions: &MapDimensions,
	outer_edges: &Vec<EdgeLine>,
) -> Vec<(usize, usize)> {
	let columns = map_dimensions.get_total_field_cell_columns();
	let rows = map_dimensions.get_total_field_cell_rows();
	let field_cell_unit_size = map_dimensions.get_field_cell_unit_size();
	let mut candidates: Vec<(usize, usize)> = vec![];
	for row in 0..rows {
		for col in 0..columns {
			// find coord of top left field cell corner
			let x1 = col as f32 * field_cell_unit_size - (map_dimensions.get_length() as f32 / 2.0);
			let y1 = row as f32 * -field_cell_unit_size + (map_dimensions.get_depth() as f32 / 2.0);

			//TODO what happens when two meshes are next to each other but a field cell overlaps their boundary -> treated as impassable currently

			// create a horizontal edge with constant y
			let hori = EdgeLine::build(
				Vec2::new(x1, y1),
				Vec2::new(map_dimensions.get_length() as f32 / 2.0, y1),
			);
			let mut count_intersections = 0;
			let mut count_touch = 0;
			for edge in outer_edges {
				match hori.does_intersect(edge) {
					Intersection::Intersect => {
						count_intersections += 1;
					}
					Intersection::Touch => {
						count_touch += 1;
					}
					Intersection::None => {}
				}
			}
			// if intersections is odd then the vertex is within the mesh
			// if it touches an even and non-zero number of times then it might be within mesh
			if count_intersections % 2 == 1 || count_touch > 0 && count_touch % 2 == 0 {
				candidates.push((row, col));
			}
		}
	}
	candidates
}
/// Using a list of [FieldCell] create an edge for each side of the cell/box and check to see if any edge intersects the outer edges of a mesh. If one of the four sides of a [FieldCell] intersects a mesh then that [FieldCell] is not wholly inside of the mesh. Return the list of [FieldCell] that intersect (thereby overlap) the outer edge of a mesh
fn identify_field_cells_that_intersect_mesh(
	map_dimensions: &MapDimensions,
	candidates: &[(usize, usize)],
	outer_edges: &Vec<EdgeLine>,
) -> Vec<(usize, usize)> {
	let field_cell_unit_size = map_dimensions.get_field_cell_unit_size();
	let mut failed_candidates: Vec<(usize, usize)> = vec![];
	for (row, col) in candidates.iter() {
		// to test whether the entire field cell is within the mesh we need to take each edge of the field cell and test that none of them intersect with any mesh edges.
		// Construct each edge of the square field cell:
		let offset_x = map_dimensions.get_length() as f32 / 2.0;
		let offset_y = map_dimensions.get_depth() as f32 / 2.0;
		// vertex: top-left
		let tl = Vec2::new(
			*col as f32 * field_cell_unit_size - offset_x,
			*row as f32 * -field_cell_unit_size + offset_y,
		);
		// vertex: top-right
		let tr = Vec2::new(
			*col as f32 * field_cell_unit_size - offset_x + field_cell_unit_size,
			*row as f32 * -field_cell_unit_size + offset_y,
		);
		// vertex: bottom-left
		let bl = Vec2::new(
			*col as f32 * field_cell_unit_size - offset_x,
			*row as f32 * -field_cell_unit_size + offset_y - field_cell_unit_size,
		);
		// vertex: bottom-right
		let br = Vec2::new(
			*col as f32 * field_cell_unit_size - offset_x + field_cell_unit_size,
			*row as f32 * -field_cell_unit_size + offset_y - field_cell_unit_size,
		);
		// edge: left up-down
		let edge_lud = EdgeLine::build(tl, bl);
		// edge: right up-down
		let edge_rud = EdgeLine::build(tr, br);
		// edge: bottom left-right
		let edge_blr = EdgeLine::build(bl, br);
		// edge: top left-right
		let edge_tlr = EdgeLine::build(tl, tr);
		// look for intersections
		let field_edges = [edge_lud, edge_rud, edge_blr, edge_tlr];
		for edge in outer_edges {
			// if an edge intersects any of the field edges then the field
			// cell is outside of the meshes. If an edge is parallel then
			// it's marked as failed
			for field_edge in field_edges.iter() {
				match edge.does_intersect(field_edge) {
					Intersection::Intersect => {
						failed_candidates.push((*row, *col));
						break;
					}
					Intersection::Touch => {
						failed_candidates.push((*row, *col));
					}
					_ => {}
				}
			}
		}
	}
	failed_candidates
}

/// Represents two points that form the edge between mech vertices
#[derive(Clone, Debug)]
struct MeshTriEdge<T: PartialEq>(T, T);
// custom impl so we can test whether two edges are teh same but with start and end coords swapped
impl<T: PartialEq> PartialEq for MeshTriEdge<T> {
	fn eq(&self, other: &Self) -> bool {
		(self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
	}
}

/// Defines whether an intersection has occured
#[derive(PartialEq, Debug)]
enum Intersection {
	/// Indicates that an edge meets and passes through another edge
	Intersect,
	/// Indicates that edges only touch one another, this is a special case of intersection
	Touch,
	/// Edge does not intersect
	None,
}

/// Represents the start and end coordinates of a line in space
#[derive(Debug)]
struct EdgeLine {
	/// Where the line starts
	start: Vec2,
	/// Where the line ends
	end: Vec2,
}

impl EdgeLine {
	/// Create an [`EdgeLine`] from two positions
	fn build(start: Vec2, end: Vec2) -> Self {
		EdgeLine { start, end }
	}
	/// Finds whether two edges intersect/touch
	fn does_intersect(&self, other: &EdgeLine) -> Intersection {
		//https://stackoverflow.com/questions/563198/how-do-you-detect-where-two-line-segments-intersect/565282#565282 (Ronald Goldman, published in Graphics Gems, page 304)
		let self_segment = self.end - self.start;
		let other_segment = other.end - other.start;

		let cross_segment = self_segment.perp_dot(other_segment);
		if cross_segment == 0.0 {
			// find whether paralell or collinear
			if (other.start - self.start).perp_dot(self_segment) == 0.0 {
				// collinear, check if they overlap
				let t_0 =
					(other.start - self.start).dot(self_segment) / (self_segment.dot(self_segment));
				let t_1 = t_0 + other_segment.dot(self_segment) / (self_segment.dot(self_segment));

				// if other_segment.dot(self_segment) < 0.0 {
				// 	if (t_0 <= 0.0 || t_0 >= 1.0) && (t_1 <= 0.0 || t_1 >= 1.0) {
				// 		// overlap
				// 	} else {
				// 		// disjoint
				// 		Intersection::None
				// 	}
				// } else {

				if (0.0..=1.0).contains(&t_0) && (0.0..=1.0).contains(&t_1) {
					// overlap
					Intersection::Touch
				} else {
					// disjoint
					Intersection::None
				}
			// }
			} else {
				// parallel, non-intersecting
				Intersection::None
			}
		} else {
			// may intersect, check if intersection point is on both segments
			let u = (other.start - self.start).perp_dot(self_segment) / cross_segment;
			let t = (other.start - self.start).perp_dot(other_segment) / cross_segment;
			if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&t) {
				// special case where an edge only touches the start/end of another edge
				let point = self.start + t * self_segment; //, other.start + u * other_segment);
											   //TODO? floating precision can throw off touch check
				if (point - other.start).length_squared() < f32::EPSILON
					|| (point - other.end).length_squared() < f32::EPSILON
				{
					Intersection::Touch
				} else {
					Intersection::Intersect
				}
			} else {
				Intersection::None
			}
		}
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use bevy::render::{
		mesh::{Indices, PrimitiveTopology},
		render_asset::RenderAssetUsages,
	};

	use super::*;
	#[test]
	#[cfg(feature = "ron")]
	fn sector_cost_fields_file_ron() {
		let map_dimensions = MapDimensions::new(1920, 1920, 640, 16.0);
		let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
		let _cost_fields = SectorCostFields::from_ron(path, &map_dimensions);
	}
	#[test]
	#[cfg(feature = "csv")]
	fn sector_cost_fields_file_csv() {
		let map_dimensions = MapDimensions::new(1920, 1920, 640, 16.0);
		let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
		let _cost_fields = SectorCostFields::from_csv_dir(&map_dimensions, path);
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
	#[test]
	fn intersect_para() {
		let edge1 = EdgeLine::build(Vec2::new(0.0, 0.0), Vec2::new(3.0, 3.0));
		let edge2 = EdgeLine::build(Vec2::new(-1.0, 0.0), Vec2::new(2.0, 3.0));
		assert_eq!(edge1.does_intersect(&edge2), Intersection::None);
	}
	#[test]
	fn intersect_yes() {
		let edge1 = EdgeLine::build(Vec2::new(0.0, 0.0), Vec2::new(3.0, 3.0));
		let edge2 = EdgeLine::build(Vec2::new(-1.0, 5.0), Vec2::new(3.0, 2.0));
		assert_eq!(edge1.does_intersect(&edge2), Intersection::Intersect);
	}
	#[test]
	fn intersect_yes_but_oob() {
		let edge1 = EdgeLine::build(Vec2::new(0.0, 0.0), Vec2::new(3.0, 3.0));
		let edge2 = EdgeLine::build(Vec2::new(-1.0, 5.0), Vec2::new(-0.5, 1.25));
		assert_eq!(edge1.does_intersect(&edge2), Intersection::None);
	}
	// #[test]
	// fn intersect_no() {
	// 	let edge1 = EdgeLine::build(Vec2::new(0.0, 0.0), Vec2::new(3.0, 3.0));
	// 	let edge2 = EdgeLine::build(Vec2::new(-1.0, 0.0), Vec2::new(2.0, 3.0));
	// 	assert!(!edge1.does_intersect(&edge2))
	// }
	#[test]
	fn mesh_edges_triangle_list() {
		let mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::default(),
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![
				[-960.0, 640.0, 0.0],
				[-960.0, 960.0, 0.0],
				[700.0, 960.0, 0.0],
				[900.0, 800.0, 0.0],
				[700.0, 640.0, 0.0],
			],
		)
		.with_inserted_indices(Indices::U32(vec![0, 1, 2, 2, 3, 4, 4, 2, 0]));
		let mesh_vertices = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
		let vertex_points = mesh_vertices.as_float3().unwrap();
		let result = retrieve_mesh_edges(&&mesh, vertex_points);
		let actual = vec![
			MeshTriEdge(0, 1),
			MeshTriEdge(1, 2),
			MeshTriEdge(2, 0),
			MeshTriEdge(2, 3),
			MeshTriEdge(3, 4),
			MeshTriEdge(4, 2),
			MeshTriEdge(4, 2),
			MeshTriEdge(2, 0),
			MeshTriEdge(0, 4),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn mesh_edges_triangle_strip() {
		let mesh = Mesh::new(
			PrimitiveTopology::TriangleStrip,
			RenderAssetUsages::default(),
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![
				[-192.0, 640.0, 0.0],
				[-192.0, -640.0, 0.0],
				[192.0, 640.0, 0.0],
				[192.0, -640.0, 0.0],
			],
		)
		.with_inserted_indices(Indices::U32(vec![0, 1, 2, 3]));
		let mesh_vertices = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
		let vertex_points = mesh_vertices.as_float3().unwrap();
		let result = retrieve_mesh_edges(&&mesh, vertex_points);
		let actual = vec![
			MeshTriEdge(0, 1),
			MeshTriEdge(1, 2),
			MeshTriEdge(2, 0),
			MeshTriEdge(2, 1),
			MeshTriEdge(1, 3),
			MeshTriEdge(3, 2),
		];
		assert_eq!(actual, result);
	}
	// #[test]
	// fn mesh_init_2d() {
	// 	let length = 1920;
	// 	let depth = 1920;
	// 	let sector_resolution = 320;
	// 	let actor_size = 16.0;
	// 	let map_dimensions = MapDimensions::new(length, depth, sector_resolution, actor_size);
	// 	let mesh = Mesh::new(
	// 		PrimitiveTopology::TriangleList,
	// 		RenderAssetUsages::default(),
	// 	)
	// 	.with_inserted_attribute(
	// 		Mesh::ATTRIBUTE_POSITION,
	// 		vec![
	// 			[-960.0, 640.0, 0.0],
	// 			[-960.0, 960.0, 0.0],
	// 			[700.0, 960.0, 0.0],
	// 			[900.0, 800.0, 0.0],
	// 			[700.0, 640.0, 0.0],
	// 		],
	// 	)
	// 	.with_inserted_indices(Indices::U32(vec![0, 1, 2, 2, 3, 4, 4, 2, 0]));
	// 	let meshes = vec![(&mesh, Vec2::new(0.0, 0.0))];
	// 	let internal_cost = 1;
	// 	let external_cost =  255;
	// 	let s_cost_field = SectorCostFields::from_bevy_2d_meshes(&map_dimensions, &meshes, internal_cost, external_cost);
	// 	let result = s_cost_field.get_scaled();
	// 	let actual = [];
	// 	assert_eq!(actual, result);
	// }
}
