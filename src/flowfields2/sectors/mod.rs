//! A map is split into a series of `MxN` sectors composed of various fields
//! used for path calculation
//!
//!

pub mod sector_cost;
pub mod sector_portals;

use bevy::prelude::*;

use crate::flowfields2::fields::FieldCell;
use crate::flowfields2::utilities::{FIELD_RESOLUTION, Ordinal};

/// Unique ID of a sector
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Reflect)]
pub struct SectorID {
	column: i32,
	row: i32,
}

impl SectorID {
	/// Create a new instance of [SectorID]
	pub fn new(column: i32, row: i32) -> Self {
		SectorID { column, row }
	}
	/// Get the sector `(column, row)` tuple
	pub fn get(&self) -> (i32, i32) {
		(self.column, self.row)
	}
	/// Get the sector column
	pub fn get_column(&self) -> i32 {
		self.column
	}
	/// Get the sector row
	pub fn get_row(&self) -> i32 {
		self.row
	}
}

/// The dimensions and scaling of the world
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Default, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct MapDimensions {
	/// The origin point of your game world. This is used to translate a point from Bevy global space into a [SectorID] and [FieldCell], and back
	///
	/// ## In 3d
	///
	/// This is taken as an `(x, z)` point in space
	///
	/// ## In 2d
	///
	/// This is taken as an `(x, y)` point in space
	origin: (f32, f32),
	/// Dimensions of the world
	///
	/// ## In 3d
	///
	/// This is taken as `(x, z)` length of the world, imagine a birds eye view of a world
	///
	/// ## In 2d
	///
	/// This is taken as the `(x, y)` length of the world
	size: (f32, f32),
	/// A unit of space, this forms the basis of the dimensions of a [FieldCell]
	/// and influences the number of sectors computed
	world_unit_size: f32,
	/// Actor size influences the expansion of [CostField] impassable cells to
	/// ensure that Actors avoid trying to path through small gaps which they
	/// can't fit through - hence an alternative route will be explored to go
	/// around small gaps
	actor_scale: u32,
}

impl MapDimensions {
	/// Create a new instance of [MapDimensions].
	pub fn new(
		origin: (f32, f32),
		size: (f32, f32),
		world_unit_size: f32,
		actor_size: f32,
	) -> Self {
		if size.0 <= 0.0 || size.1 <= 0.0 {
			panic!(
				"Size must be greater than 0.0, found size({}, {})",
				size.0, size.1
			)
		}
		if actor_size <= 0.0 {
			panic!("Actor size cannot be less than zero");
		}
		if world_unit_size <= 0.0 {
			panic!("World unit size must be more than zero")
		}
		let sector_len = world_unit_size * FIELD_RESOLUTION as f32;
		let sector_columns = size.0 / sector_len;
		let sector_rows = size.1 / sector_len;
		if sector_columns < 1.0 || sector_rows < 1.0 {
			panic!(
				"world unit size x {} must be an exact factor of `size`",
				FIELD_RESOLUTION
			);
		}
		if actor_size >= sector_len {
			panic!(
				"actor_size cannot be bigger than the length of a sector. Sector length {}, actor size {}",
				sector_len, actor_size
			);
		}
		let actor_scale = (actor_size / world_unit_size).ceil() as u32;
		if actor_scale >= 10 {
			panic!("Actors cannot be larger than an entire sector");
		}
		MapDimensions {
			origin,
			size,
			world_unit_size,
			actor_scale,
		}
	}
	pub fn get_size(&self) -> (f32, f32) {
		self.size
	}
	/// Number of `x` units in size
	pub fn get_length(&self) -> f32 {
		self.size.0
	}
	/// 2d: number of `y` units in size
	///
	/// 3d: number of `z` units in size
	pub fn get_depth(&self) -> f32 {
		self.size.1
	}
	pub fn get_unit_scale(&self) -> f32 {
		self.world_unit_size
	}
	pub fn get_actor_scale(&self) -> u32 {
		self.actor_scale
	}
	/// Based on `size` and unit size calculate the number of [`FieldCell`] columns across all sectors
	pub fn get_total_field_cell_columns(&self) -> usize {
		(self.get_length() / (self.world_unit_size * FIELD_RESOLUTION as f32)) as usize
			* FIELD_RESOLUTION
	}
	/// Based on `size` and unit size calculate the number of [`FieldCell`] rows across all sectors
	pub fn get_total_field_cell_rows(&self) -> usize {
		(self.get_depth() / (self.world_unit_size * FIELD_RESOLUTION as f32)) as usize
			* FIELD_RESOLUTION
	}

	/// From a global position in 2D `x, y` calculate the sector ID that point resides in
	#[cfg(feature = "2d")]
	pub fn get_sector_id_from_xy(&self, position: Vec2) -> Option<SectorID> {
		let sector_len = self.world_unit_size * FIELD_RESOLUTION as f32;
		// find the global point in space for Sector (0, 0) based on the global origin
		let top_left = Vec2::new(self.size.0 / -2.0, self.size.1 / 2.0)
			+ Vec2::new(self.origin.0, self.origin.1);
		// find the bottom corner
		let bottom_right = Vec2::new(self.size.0 / 2.0, self.size.1 / -2.0)
			+ Vec2::new(self.origin.0, self.origin.1);
		// ensure position is within bounds
		if position.x < top_left.x
			|| position.x > bottom_right.x
			|| position.y > top_left.y
			|| position.y < bottom_right.y
		{
			error!(
				"Position is out of bounds of MapDimensions, x {}, y {}, cannot calculate SectorID. Is the actor outside of the map or trying to request route outside of it?",
				position.x, position.y
			);
			//TODO use Result instead
			return None;
		}
		// get the vector size from fields origin to position
		let to_pos = (position - top_left).abs();
		// the lengths of this vector when divided by the size of a sector reveal the sector ID
		let col = (to_pos.x / sector_len).floor();
		let row = (to_pos.y / sector_len).floor();
		Some(SectorID::new(col as i32, row as i32))
	}

	/// Get the `(x,y)` coordinates of the top left corner of a sector in global space
	#[cfg(feature = "2d")]
	pub fn get_sector_corner_xy(&self, sector_id: SectorID) -> Vec2 {
		let sector_len = self.world_unit_size * FIELD_RESOLUTION as f32;
		// find the global point in space for Sector (0, 0) based on the global origin
		let top_left = Vec2::new(self.size.0 / -2.0, self.size.1 / 2.0)
			+ Vec2::new(self.origin.0, self.origin.1);

		let relative_offset = Vec2::new(
			sector_id.get_column() as f32 * sector_len,
			sector_id.get_row() as f32 * -sector_len,
		);

		top_left + relative_offset
	}
	//TODO return Result
	/// From a 2d position get the sector and field cell it resides in
	#[cfg(feature = "2d")]
	pub fn get_sector_and_field_cell_from_xy(
		&self,
		position: Vec2,
	) -> Option<(SectorID, FieldCell)> {
		if let Some(sector_id) = self.get_sector_id_from_xy(position) {
			let sector_corner_origin = self.get_sector_corner_xy(sector_id);
			let field_id_0 =
				((position.x - sector_corner_origin.x) / self.world_unit_size).floor() as usize;
			let field_id_1 =
				((-position.y + sector_corner_origin.y) / self.world_unit_size).floor() as usize;
			let field_id = FieldCell::new(field_id_0, field_id_1);
			return Some((sector_id, field_id));
		}
		None
	}
	/// From a field cell within a Sector retrieve the 2d Vec2 of its
	/// position. If the position sits outside of the world then [None] is
	/// returned
	#[cfg(feature = "2d")]
	pub fn get_xy_from_field_sector(&self, sector: SectorID, field: FieldCell) -> Option<Vec2> {
		let sector_xy = self.get_sector_corner_xy(sector);
		let f_col = field.get_column() as f32;
		let f_row = field.get_row() as f32;

		let f_offset = Vec2::new(f_col * self.world_unit_size, -f_row * self.world_unit_size);
		// point in space for top-left corner of the field cell
		let point = sector_xy + f_offset;
		if point.x.abs() > self.origin.0 + self.get_length() / 2.0
			|| point.y.abs() > self.origin.1 + self.get_depth() / 2.0
		{
			None
		} else {
			Some(point)
		}
	}

	/// From a field cell within a Sector retrieve the 2d (x-z) Vec3 of its
	/// position. If the position is outside of the world then [None] is
	/// returned
	///
	/// The `y` coordinate is defaulted to `0.0`.
	#[cfg(feature = "3d")]
	pub fn get_xyz_from_field_sector(&self, sector: SectorID, field: FieldCell) -> Option<Vec3> {
		let sector_xyz = self.get_sector_corner_xyz(sector);
		let f_col = field.get_column() as f32;
		let f_row = field.get_row() as f32;

		let f_offset = Vec3::new(
			f_col * self.world_unit_size,
			0.0,
			f_row * self.world_unit_size,
		);
		// point in space for top-left corner of the field cell
		let point = sector_xyz + f_offset;
		if point.x.abs() > self.origin.0 + self.get_length() / 2.0
			|| point.z.abs() > self.origin.1 + self.get_depth() / 2.0
		{
			None
		} else {
			Some(point)
		}
	}

	/// From a position in `x, y, z` space and the dimensions of the map calculate
	/// the sector ID that point resides in
	#[cfg(feature = "3d")]
	pub fn get_sector_id_from_xyz(&self, position: Vec3) -> Option<SectorID> {
		let sector_len = self.world_unit_size * FIELD_RESOLUTION as f32;
		// find the global point in space for Sector (0, 0) based on the global origin
		let top_left = Vec2::new(self.size.0 / -2.0, self.size.1 / -2.0)
			+ Vec2::new(self.origin.0, self.origin.1);
		// find the bottom corner
		let bottom_right = Vec2::new(self.size.0 / 2.0, self.size.1 / 2.0)
			+ Vec2::new(self.origin.0, self.origin.1);
		// ensure position is within bounds
		if position.x < top_left.x
			|| position.x > bottom_right.x
			|| position.z < top_left.y
			|| position.z > bottom_right.y
		{
			error!(
				"Position is out of bounds of MapDimensions, x {}, z {}, cannot calculate SectorID. Is the actor outside of the map or trying to request route outside of it?",
				position.x, position.z
			);
			//TODO use Result instead
			return None;
		}
		// get the vector size from fields origin to position
		let to_pos = (position.xz() - top_left).abs();
		// the lengths of this vector when divided by the size of a sector reveal the sector ID
		let col = (to_pos.x / sector_len).floor();
		let row = (to_pos.y / sector_len).floor();
		Some(SectorID::new(col as i32, row as i32))
	}

	/// Calculate the `x, y, z` coordinates at the top-left corner of a sector based on map dimensions
	#[cfg(feature = "3d")]
	pub fn get_sector_corner_xyz(&self, sector_id: SectorID) -> Vec3 {
		let sector_len = self.world_unit_size * FIELD_RESOLUTION as f32;
		// find the global point in space for Sector (0, 0) based on the global origin
		let top_left = Vec3::new(self.size.0 / -2.0, 0.0, self.size.1 / -2.0)
			+ Vec3::new(self.origin.0, 0.0, self.origin.1);

		let relative_offset = Vec3::new(
			sector_id.get_column() as f32 * sector_len,
			0.0,
			sector_id.get_row() as f32 * -sector_len,
		);

		top_left + relative_offset
	}
	//TODO return Result
	/// From a point in 3D space calculate what Sector and field cell it resides in
	#[cfg(feature = "3d")]
	pub fn get_sector_and_field_cell_from_xyz(
		&self,
		position: Vec3,
	) -> Option<(SectorID, FieldCell)> {
		if let Some(sector_id) = self.get_sector_id_from_xyz(position) {
			let sector_corner_origin = self.get_sector_corner_xyz(sector_id);
			let field_id_0 =
				((position.x - sector_corner_origin.x) / self.world_unit_size).floor() as usize;
			let field_id_1 =
				((position.z - sector_corner_origin.z) / self.world_unit_size).floor() as usize;
			let field_id = FieldCell::new(field_id_0, field_id_1);
			return Some((sector_id, field_id));
		}
		None
	}

	/// A sector has up to four neighbours. Based on the ID of the sector and the dimensions
	/// of the map retrieve the IDs neighbouring sectors
	pub fn get_ids_of_neighbouring_sectors(self, sector_id: &SectorID) -> Vec<SectorID> {
		Ordinal::get_sector_neighbours(
			sector_id,
			self.get_length(),
			self.get_depth(),
			self.get_unit_scale(),
		)
	}

	/// A sector has up to four neighbours. Based on the ID of the sector and the dimensions
	/// of the map retrieve the IDs neighbouring sectors and the [Ordinal] direction from the
	/// current sector that that sector is found in
	pub fn get_ordinal_and_ids_of_neighbouring_sectors(
		&self,
		sector_id: &SectorID,
	) -> Vec<(Ordinal, SectorID)> {
		Ordinal::get_sector_neighbours_with_ordinal(
			sector_id,
			self.get_length(),
			self.get_depth(),
			self.get_unit_scale(),
		)
	}
	/// From an [Ordinal] get the ID of a neighbouring sector. Returns [None]
	/// if the sector would be out of bounds
	pub fn get_sector_id_from_ordinal(
		&self,
		ordinal: Ordinal,
		sector_id: &SectorID,
	) -> Option<SectorID> {
		let sector_column_limit =
			(self.get_length() / (self.world_unit_size * FIELD_RESOLUTION as f32)) as i32 - 1;
		let sector_row_limit =
			(self.get_depth() / (self.world_unit_size * FIELD_RESOLUTION as f32)) as i32 - 1;
		match ordinal {
			Ordinal::North => {
				if sector_id.get_row() - 1 >= 0 {
					Some(SectorID::new(
						sector_id.get_column(),
						sector_id.get_row() - 1,
					))
				} else {
					None
				}
			}
			Ordinal::East => {
				if sector_id.get_column() + 1 <= sector_column_limit {
					Some(SectorID::new(
						sector_id.get_column() + 1,
						sector_id.get_row(),
					))
				} else {
					None
				}
			}
			Ordinal::South => {
				if sector_id.get_row() + 1 <= sector_row_limit {
					Some(SectorID::new(
						sector_id.get_column(),
						sector_id.get_row() + 1,
					))
				} else {
					None
				}
			}
			Ordinal::West => {
				if sector_id.get_column() - 1 >= 0 {
					Some(SectorID::new(
						sector_id.get_column() - 1,
						sector_id.get_row(),
					))
				} else {
					None
				}
			}
			Ordinal::NorthEast => {
				if sector_id.get_row() - 1 >= 0 {
					if sector_id.get_column() + 1 <= sector_column_limit {
						Some(SectorID::new(
							sector_id.get_column() + 1,
							sector_id.get_row() - 1,
						))
					} else {
						None
					}
				} else {
					None
				}
			}
			Ordinal::SouthEast => {
				if sector_id.get_row() + 1 <= sector_row_limit {
					if sector_id.get_column() + 1 <= sector_column_limit {
						Some(SectorID::new(
							sector_id.get_column() + 1,
							sector_id.get_row() + 1,
						))
					} else {
						None
					}
				} else {
					None
				}
			}
			Ordinal::SouthWest => {
				if sector_id.get_row() + 1 <= sector_row_limit {
					if sector_id.get_column() - 1 >= 0 {
						Some(SectorID::new(
							sector_id.get_column() - 1,
							sector_id.get_row() + 1,
						))
					} else {
						None
					}
				} else {
					None
				}
			}
			Ordinal::NorthWest => {
				if sector_id.get_row() - 1 >= 0 {
					if sector_id.get_column() - 1 >= 0 {
						Some(SectorID::new(
							sector_id.get_column() - 1,
							sector_id.get_row() - 1,
						))
					} else {
						None
					}
				} else {
					None
				}
			}
			Ordinal::Zero => {
				error!("`get_sector_id_from_ordinal` should never be called with `Ordinal::Zero`");
				None
			}
		}
	}
	// /// From a list of meshes find the maximum and minimum x-y dimensions across all meshes to represent the size of the world as an MxN set of Flowfields
	// #[cfg(feature = "2d")]
	// pub fn from_bevy_2d_meshes(meshes: &Vec<&Mesh>, sector_resolution: u32, actor_size: f32) -> Self {
	// 	let mut min_x = None;
	// 	let mut max_x = None;
	// 	let mut min_y = None;
	// 	let mut max_y = None;

	// 	for mesh in meshes {
	// 		let vert_attrib = mesh.attribute(Mesh::ATTRIBUTE_POSITION);
	// 		if let Some(attrib) = vert_attrib {
	// 			if let Some(vertices) = attrib.as_float3() {
	// 				for vertex in vertices {
	// 					let x = vertex[0];
	// 					let y = vertex[1];
	// 					if min_x.is_none() {
	// 						min_x = Some(x);
	// 					} else if min_x.unwrap() > x {
	// 							min_x = Some(x);
	// 						}
	// 					if max_x.is_none() {
	// 						max_x = Some(x);
	// 					} else if max_x.unwrap() < x {
	// 							max_x = Some(x);
	// 					}
	// 					if min_y.is_none() {
	// 						min_y = Some(y);
	// 					} else if min_y.unwrap() > y {
	// 						min_y = Some(y);
	// 					}
	// 					if max_y.is_none() {
	// 						max_y = Some(y);
	// 					} else if max_y.unwrap() < y {
	// 						max_y = Some(y);
	// 					}
	// 				}
	// 			} else {
	// 				warn!("A mesh cannot represent its vertices in `as_float3` format, it cannot be used to create flowfields");
	// 			}
	// 		} else {
	// 			warn!("A mesh has no vertices, it cannot be used to create flowfields");
	// 		}
	// 	}

	// 	if min_x.is_some() && max_x.is_some() && min_y.is_some() && max_y.is_some() {
	// 		let length = (max_x.unwrap() - min_x.unwrap()) as u32;
	// 		let depth = (max_y.unwrap() - min_y.unwrap()) as u32;
	// 		MapDimensions::new(length, depth, sector_resolution, actor_size)
	// 	} else {
	// 		panic!("Unable to determine world size from meshes");
	// 	}
	// }
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn sector_costfields_top_left_sector_id_from_xyz() {
		let map_dimensions = MapDimensions::new(20, 20, 10, 1.0);
		let position = Vec3::new(-5.0, 0.0, -5.0);
		let result = map_dimensions.get_sector_id_from_xyz(position).unwrap();
		let actual: SectorID = SectorID::new(0, 0);
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_costfields_top_right_sector_id_from_xyz() {
		let map_dimensions = MapDimensions::new(20, 20, 10, 1.0);
		let position = Vec3::new(5.0, 0.0, -5.0);
		let result = map_dimensions.get_sector_id_from_xyz(position).unwrap();
		let actual: SectorID = SectorID::new(1, 0);
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_costfields_bottom_right_sector_id_from_xyz() {
		let map_dimensions = MapDimensions::new(20, 20, 10, 1.0);
		let position = Vec3::new(5.0, 0.0, 5.0);
		let result = map_dimensions.get_sector_id_from_xyz(position).unwrap();
		let actual: SectorID = SectorID::new(1, 1);
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_costfields_bottom_left_sector_id_from_xyz() {
		let map_dimensions = MapDimensions::new(20, 20, 10, 1.0);
		let position = Vec3::new(-5.0, 0.0, 5.0);
		let result = map_dimensions.get_sector_id_from_xyz(position).unwrap();
		let actual: SectorID = SectorID::new(0, 1);
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_fieldcell_id_from_xyz() {
		let map_dimensions = MapDimensions::new(300, 300, 100, 1.0);
		let position = Vec3::new(0.0, 0.0, 0.0);
		let result = map_dimensions
			.get_sector_and_field_cell_from_xyz(position)
			.unwrap();
		let actual = FieldCell::new(5, 5);
		assert_eq!(actual, result.1);
	}
	#[test]
	fn sector_fieldcell_id_from_xyz_small() {
		let map_dimensions = MapDimensions::new(25, 50, 5, 1.0);
		let position = Vec3::new(0.0, 0.0, 0.0);
		let result = map_dimensions
			.get_sector_and_field_cell_from_xyz(position)
			.unwrap();
		let actual_sector = SectorID::new(2, 5);
		let actual_field = FieldCell::new(5, 0);
		assert_eq!(actual_sector, result.0);
		assert_eq!(actual_field, result.1);
	}
	#[test]
	fn sector_fieldcell_id_from_xyz_large() {
		let map_dimensions = MapDimensions::new(290, 290, 290, 1.0);
		let position = Vec3::new(0.0, 0.0, 0.0);
		let result = map_dimensions
			.get_sector_and_field_cell_from_xyz(position)
			.unwrap();
		let actual_sector = SectorID::new(0, 0);
		let actual_field = FieldCell::new(5, 5);
		assert_eq!(actual_sector, result.0);
		assert_eq!(actual_field, result.1);
	}
	#[test]
	fn sector_from_xy_none() {
		let map_dimensions = MapDimensions::new(1280, 1280, 640, 16.0);
		let position = Vec2::new(-1500.0, 0.0);
		let result = map_dimensions.get_sector_id_from_xy(position);

		assert!(result.is_none());
	}
	#[test]
	fn sector_from_xy() {
		let map_dimensions = MapDimensions::new(1280, 1280, 640, 16.0);
		let position = Vec2::new(530.0, 75.0);
		let result = map_dimensions.get_sector_id_from_xy(position);
		let actual = SectorID::new(1, 0);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_xyz_corner_zero() {
		let sector_id = SectorID::new(0, 0);
		let map_dimensions = MapDimensions::new(30, 30, 10, 1.0);
		let result = map_dimensions.get_sector_corner_xyz(sector_id);
		let actual = Vec3::new(-15.0, 0.0, -15.0);
		assert_eq!(actual, result)
	}
	#[test]
	fn sector_xyz_corner_centre() {
		let sector_id = SectorID::new(1, 1);
		let map_dimensions = MapDimensions::new(30, 30, 10, 1.0);
		let result = map_dimensions.get_sector_corner_xyz(sector_id);
		let actual = Vec3::new(-5.0, 0.0, -5.0);
		assert_eq!(actual, result)
	}
	#[test]
	fn get_northern_sector_neighbours() {
		let sector_id = SectorID::new(4, 0);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			SectorID::new(5, 0),
			SectorID::new(4, 1),
			SectorID::new(3, 0),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_eastern_sector_neighbours() {
		let sector_id = SectorID::new(19, 3);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			SectorID::new(19, 2),
			SectorID::new(19, 4),
			SectorID::new(18, 3),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_southern_sector_neighbours() {
		let sector_id = SectorID::new(5, 19);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			SectorID::new(5, 18),
			SectorID::new(6, 19),
			SectorID::new(4, 19),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_western_sector_neighbours() {
		let sector_id = SectorID::new(0, 5);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			SectorID::new(0, 4),
			SectorID::new(1, 5),
			SectorID::new(0, 6),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_centre_sector_neighbours() {
		let sector_id = SectorID::new(5, 7);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			SectorID::new(5, 6),
			SectorID::new(6, 7),
			SectorID::new(5, 8),
			SectorID::new(4, 7),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_northern_sector_neighbours_with_direction() {
		let sector_id = SectorID::new(4, 0);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			(Ordinal::East, SectorID::new(5, 0)),
			(Ordinal::South, SectorID::new(4, 1)),
			(Ordinal::West, SectorID::new(3, 0)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_eastern_sector_neighbours_with_direction() {
		let sector_id = SectorID::new(19, 3);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			(Ordinal::North, SectorID::new(19, 2)),
			(Ordinal::South, SectorID::new(19, 4)),
			(Ordinal::West, SectorID::new(18, 3)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_southern_sector_neighbours_with_direction() {
		let sector_id = SectorID::new(5, 19);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			(Ordinal::North, SectorID::new(5, 18)),
			(Ordinal::East, SectorID::new(6, 19)),
			(Ordinal::West, SectorID::new(4, 19)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_western_sector_neighbours_with_direction() {
		let sector_id = SectorID::new(0, 5);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			(Ordinal::North, SectorID::new(0, 4)),
			(Ordinal::East, SectorID::new(1, 5)),
			(Ordinal::South, SectorID::new(0, 6)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_centre_sector_neighbours_with_direction() {
		let sector_id = SectorID::new(5, 7);
		let map_dimensions = MapDimensions::new(200, 200, 10, 1.0);
		let result = map_dimensions.get_ordinal_and_ids_of_neighbouring_sectors(&sector_id);
		let actual = vec![
			(Ordinal::North, SectorID::new(5, 6)),
			(Ordinal::East, SectorID::new(6, 7)),
			(Ordinal::South, SectorID::new(5, 8)),
			(Ordinal::West, SectorID::new(4, 7)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_id_ordinal_north() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 1);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::North, &sector_id);
		let actual = SectorID::new(1, 0);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_id_ordinal_east() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 1);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::East, &sector_id);
		let actual = SectorID::new(2, 1);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_id_ordinal_south() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 1);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::South, &sector_id);
		let actual = SectorID::new(1, 2);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_id_ordinal_west() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 1);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::West, &sector_id);
		let actual = SectorID::new(0, 1);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_id_ordinal_northeast() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 1);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::NorthEast, &sector_id);
		let actual = SectorID::new(2, 0);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_id_ordinal_southeast() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 1);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::SouthEast, &sector_id);
		let actual = SectorID::new(2, 2);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_id_ordinal_southwest() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 1);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::SouthWest, &sector_id);
		let actual = SectorID::new(0, 2);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_id_ordinal_northwest() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 1);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::NorthWest, &sector_id);
		let actual = SectorID::new(0, 0);
		assert_eq!(actual, result.unwrap());
	}
	#[test]
	fn sector_id_ordinal_oob() {
		let map_dimensions = MapDimensions::new(300, 300, 10, 0.5);
		let sector_id = SectorID::new(1, 0);
		let result = map_dimensions.get_sector_id_from_ordinal(Ordinal::North, &sector_id);
		assert!(result.is_none())
	}
	#[test]
	fn get_xy() {
		let map_dimensions = MapDimensions::new(1920, 1920, 640, 16.0);
		let sector_id = SectorID::new(2, 1);
		let field_id = FieldCell::new(6, 2);
		let actual = Vec2::new(736.0, 160.0);
		let result = map_dimensions
			.get_xy_from_field_sector(sector_id, field_id)
			.unwrap();
		assert_eq!(actual, result);
	}
	#[test]
	fn get_xyz() {
		let map_dimensions = MapDimensions::new(30, 30, 10, 0.5);
		let sector_id = SectorID::new(2, 1);
		let field_id = FieldCell::new(6, 2);
		let actual = Vec3::new(11.5, 0.0, -2.5);
		let result = map_dimensions
			.get_xyz_from_field_sector(sector_id, field_id)
			.unwrap();
		assert_eq!(actual, result);
	}
	// #[test]
	// fn from_2d_meshes() {
	// 	let mut meshes = vec![];
	// 	let mesh1 = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
	// 	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![
	// 		[0.0, 0.0, 0.0],
	// 		[0.0, 10.0, 0.0],
	// 		[10.0, 10.0, 0.0],
	// 	])
	// 	.with_inserted_indices(Indices::U32(vec![0, 1, 2]));
	// 	meshes.push(&mesh1);
	// 	let mesh2 = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
	// 	.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![
	// 		[0.0, 0.0, 0.0],
	// 		[-20.0, 0.0, 0.0],
	// 		[-20.0, -10.0, 0.0],
	// 	])
	// 	.with_inserted_indices(Indices::U32(vec![0, 1, 2]));
	// 	meshes.push(&mesh2);
	// 	let sector_resolution = 10;
	// 	let actor_size = 32.0;
	// 	let result = MapDimensions::from_bevy_2d_meshes(&meshes, sector_resolution, actor_size);
	// 	let result_size = result.size;
	// 	let actual_size = (30, 20);
	// 	assert_eq!(actual_size, result_size);
	// }
}
