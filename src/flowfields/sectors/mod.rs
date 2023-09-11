//! A map is split into a series of `MxN` sectors composed of various fields
//! used for path calculation
//!
//!

pub mod sector_cost;
pub mod sector_portals;

use crate::prelude::*;
use bevy::prelude::*;

/// Unique ID of a sector
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Reflect)]
pub struct SectorID((u32, u32));

impl SectorID {
	/// Create a new instance of [SectorID]
	pub fn new(column: u32, row: u32) -> Self {
		SectorID((column, row))
	}
	/// Get the sector `(column, row)` tuple
	pub fn get(&self) -> (u32, u32) {
		self.0
	}
	/// Get the sector column
	pub fn get_column(&self) -> u32 {
		self.0 .0
	}
	/// Get the sector row
	pub fn get_row(&self) -> u32 {
		self.0 .1
	}
}

/// The dimensions of the world
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Default, Clone, Copy, Reflect)]
pub struct MapDimensions {
	/// Dimensions of the world
	///
	/// ## In 3d
	///
	/// This is taken as `(x, z)` dimensions of the world
	///
	/// ## In 2d
	///
	/// This is taken as the `(x, y)` pixel dimensions of the world
	size: (u32, u32),
	/// The factor by which the `size` of world will be divided by to produce
	/// the number of sectors. The world dimensions must be perfectly divisible
	/// by this number.
	///
	/// ## In 3d
	///
	/// This is the number of units that define a sector.
	///
	/// For a world size of `(30, 30)` and resolution `10` then there will be
	/// `3x3` sectors created where each field within a sector represents a
	/// `1x1` unit area in 3d space.
	///
	/// For a world size of `(30, 30)` and resolution `3` then there will be
	/// `10x10` sectors created where each field within a sector represents a
	/// `0.3x0.3` unit area in 3d space.
	///
	/// ## In 2d
	///
	/// This is the number of pixels that define the length/height of a sector
	/// (square sectors so the same number).
	///
	/// For a world size of `(1920, 1920)` and resolution `640` then there will
	/// be `3x3` sectors created where each field within a sector represents a
	/// `64x64` pixel area in 2d space.
	///
	/// For a world size of `(1920, 1920)` and resolution `64` then there will
	/// be `30x30` sectors created where each field within a sector represents
	/// a `6.4x6.4` pixel area in 2d space.
	sector_resolution: u32,
	/// Actor size influences the expansion of [CostField] impassable cells to
	/// ensure that Actors avoid trying to path through small gaps between `255`
	/// cells which they wouldn't be able to fit through - hence an alternative
	/// route will be explored to go around small gaps
	///
	/// ## 3d
	///
	/// For a `(30, 30)` world with resolution `10` there would be `3x3`
	/// Sectors, each 10 units in length and depth. A Sector uses
	/// [FIELD_RESOLUTION] to create an `(m, n)` array of [FieldCell]. So each
	/// cell within a field represents a `1x1` unit area - an actor size is
	/// used to produce a scaling factor based on the unit area of a cell
	///
	/// ## 2d
	///
	/// For a `(1920, 1920)` world with resolution `640` there would be `3x3`
	/// Sectors, each `640` pixels in length and depth. A Sector uses
	/// [FIELD_RESOLUTION] to create an `(m, n)` array of [FieldCell]. So each
	/// cell within a field represents a `64x64` pixel area - an actor size is
	/// used to produce a scaling factor based on the unit area ofa  cell
	actor_scale: u32,
}

impl MapDimensions {
	/// Create a new instance of [MapDimensions]. In 2d the dimensions should
	/// be measured by the number of sprites that fit into the `x` (length) and
	/// `y` (depth) axes. For 3d the recommendation is for a `unit` of space to
	/// be 1 meter, thereby the world is `x` (length) meters by `z` (depth)
	/// meters
	pub fn new(length: u32, depth: u32, sector_resolution: u32, actor_size: f32) -> Self {
		let length_rem = length % sector_resolution;
		let depth_rem = depth % sector_resolution;
		if length_rem > 0 || depth_rem > 0 {
			panic!(
				"Map dimensions `({}, {})` cannot support sectors, dimensions must be exact factors of {}",
				length, depth, sector_resolution
			);
		}
		if actor_size < 0.0 {
			panic!("Actor size cannot be less than zero");
		}
		let actor_scale = (actor_size / (sector_resolution as f32 / 10.0)).ceil() as u32;
		MapDimensions {
			size: (length, depth),
			sector_resolution,
			actor_scale,
		}
	}
	pub fn get_size(&self) -> (u32, u32) {
		self.size
	}
	pub fn get_length(&self) -> u32 {
		self.size.0
	}
	pub fn get_depth(&self) -> u32 {
		self.size.1
	}
	pub fn get_sector_resolution(&self) -> u32 {
		self.sector_resolution
	}
	pub fn get_actor_scale(&self) -> u32 {
		self.actor_scale
	}

	/// From a position in 2D `x, y` space with an origin at `(0, 0)` and the
	/// dimensions (pixels) of the map, calculate the sector ID that point resides in
	///
	/// `pixel_scale` refers to the dimensions of your map sprites, not that their `x` and `y` dimensions must be the same, i.e a square shape
	#[cfg(feature = "2d")]
	pub fn get_sector_id_from_xy(&self, position: Vec2) -> Option<SectorID> {
		if position.x < -((self.get_length() / 2) as f32)
			|| position.x > (self.get_length() / 2) as f32
			|| position.y < -((self.get_depth() / 2) as f32)
			|| position.y > (self.get_depth() / 2) as f32
		{
			error!("OOB pos, x {}, y {}", position.x, position.y);
			return None;
		}
		let x_sector_count = self.get_length() / self.get_sector_resolution();
		let y_sector_count = self.get_depth() / self.get_sector_resolution();
		// The 2D world is centred at origin (0, 0). The sector grid has an origin in the top
		// left at 2D world coords of (-map_x * pixel_scale / 2, 0, map_y * pixel_scale / 2).
		// To translate the 2D world
		// coords into a new coordinate system with a (0, 0) origin in the top left we add
		// half the map dimension to each psition coordinatem
		let x_origin = position.x + (self.get_length() / 2) as f32;
		let y_origin = (self.get_depth() / 2) as f32 - position.y;
		// the grid IDs follow a (column, row) convention, by dividing the repositioned dimension
		// by the sector grid sizes and rounding down we determine the sector indices
		let mut column = (x_origin / (self.get_sector_resolution() as f32)).floor() as u32;
		let mut row = (y_origin / (self.get_sector_resolution() as f32)).floor() as u32;
		// safety for x-y being at the exact limits of map size
		if column >= x_sector_count {
			column = x_sector_count - 1;
		}
		if row >= y_sector_count {
			row = y_sector_count - 1;
		}
		Some(SectorID::new(column, row))
	}

	/// Get the `(x,y)` coordinates of the top left corner of a sector in real space
	#[cfg(feature = "2d")]
	pub fn get_sector_corner_xy(&self, sector_id: SectorID) -> Vec2 {
		// x sector-grid origin begins in the negative
		let x_origin = -(self.get_length() as f32) / 2.0;
		let x = x_origin + sector_id.get_column() as f32 * self.get_sector_resolution() as f32;
		// y sector grid origin begins in the positive
		let y_origin = self.get_depth() as f32 / 2.0;
		let y = y_origin - sector_id.get_row() as f32 * self.get_sector_resolution() as f32;
		Vec2::new(x, y)
	}
	/// From a 2d position get the sector and field cell it resides in
	#[cfg(feature = "2d")]
	pub fn get_sector_and_field_id_from_xy(&self, position: Vec2) -> Option<(SectorID, FieldCell)> {
		if let Some(sector_id) = self.get_sector_id_from_xy(position) {
			let sector_corner_origin = self.get_sector_corner_xy(sector_id);
			let pixel_sector_field_ratio =
				self.get_sector_resolution() as f32 / FIELD_RESOLUTION as f32;
			let field_id_0 =
				((position.x - sector_corner_origin.x) / pixel_sector_field_ratio).floor() as usize;
			let field_id_1 = ((-position.y + sector_corner_origin.y) / pixel_sector_field_ratio)
				.floor() as usize;
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
		// the sector grid always begins in the top left
		// from real-space origin of (0,0) find the position of SectorID(0,0) in real space
		let sector_grid_origin_offset = {
			Vec2::new(
				self.get_length() as f32 / -2.0,
				self.get_depth() as f32 / 2.0,
			)
		};
		// the sector grid starts top left at (0,0), based on the sector we want find its origin
		// with how many units make up a sector and and sector mXn ID
		// NB: use a negative Y here, as row ID goes from 0..n it's approaching the negative Y of real space
		let sector_origin = Vec2::new(
			(sector.get_column() * self.get_sector_resolution()) as f32,
			(sector.get_row() * self.get_sector_resolution()) as f32 * -1.0,
		);
		// now we know the real-space coordinates of the top left corner of the sector
		let xy_of_sector_top_left = sector_grid_origin_offset + sector_origin;

		// determine the unit size of a field cell
		let cell_size = self.get_sector_resolution() as f32 / FIELD_RESOLUTION as f32;
		// from a cell origin of (0, 0) find the cell position relative to the field grid
		// NB: we add half of the cell size to each coord to obtain the centre position of the cell
		// NB: use negative Y here, as row ID goes form 0..n it's approaching negative Y of real-space
		let cell_position = Vec2::new(
			field.get_column() as f32 * cell_size + cell_size / 2.0,
			(field.get_row() as f32 * cell_size + cell_size / 2.0) * -1.0,
		);

		let real_space_pos = xy_of_sector_top_left + cell_position;
		// ensure not outside world
		if real_space_pos.x.abs() > self.get_length() as f32 / 2.0
			|| real_space_pos.y.abs() > self.get_depth() as f32 / 2.0
		{
			None
		} else {
			Some(real_space_pos)
		}
	}

	/// From a field cell within a Sector retrieve the 2d (x-z) Vec3 of its
	/// position. If the position is outside of the world then [None] is
	/// returned
	///
	/// The `y` coordinate is defaulted to `0.0`.
	#[cfg(feature = "3d")]
	pub fn get_xyz_from_field_sector(&self, sector: SectorID, field: FieldCell) -> Option<Vec3> {
		// the sector grid always begins in the top left
		// from real-space origin of (0,0,0) find the position of SectorID(0,0) in real space
		let sector_grid_origin_offset = {
			Vec3::new(
				self.get_length() as f32 / -2.0,
				0.0,
				self.get_depth() as f32 / -2.0,
			)
		};
		// the sector grid starts top left at (0,0), based on the sector we want find its origin
		// with how many units make up a sector and and sector mXn ID
		let sector_origin = Vec3::new(
			(sector.get_column() * self.get_sector_resolution()) as f32,
			0.0,
			(sector.get_row() * self.get_sector_resolution()) as f32,
		);
		// now we know the real-space coordinates of the top left corner of the sector
		let xyz_of_sector_top_left = sector_grid_origin_offset + sector_origin;

		// determine the unit size of a field cell
		let cell_size = self.get_sector_resolution() as f32 / FIELD_RESOLUTION as f32;
		// from a cell origin of (0, 0) find the cell position relative to the field grid
		// NB: we add half of the cell size to each coord to obtain the centre position of the cell
		let cell_position = Vec3::new(
			field.get_column() as f32 * cell_size + cell_size / 2.0,
			0.0,
			field.get_row() as f32 * cell_size + cell_size / 2.0,
		);

		let real_space_pos = xyz_of_sector_top_left + cell_position;
		// ensure not outside world
		if real_space_pos.x.abs() > self.get_length() as f32 / 2.0
			|| real_space_pos.z.abs() > self.get_depth() as f32 / 2.0
		{
			None
		} else {
			Some(real_space_pos)
		}
	}

	/// From a position in `x, y, z` space and the dimensions of the map calculate
	/// the sector ID that point resides in
	#[cfg(feature = "3d")]
	pub fn get_sector_id_from_xyz(&self, position: Vec3) -> Option<SectorID> {
		if position.x < -((self.get_length() / 2) as f32)
			|| position.x > (self.get_length() / 2) as f32
			|| position.z < -((self.get_depth() / 2) as f32)
			|| position.z > (self.get_depth() / 2) as f32
		{
			error!("OOB pos, x {}, z {}", position.x, position.z);
			return None;
		}
		let x_sector_count = self.get_length() / self.get_sector_resolution();
		let z_sector_count = self.get_depth() / self.get_sector_resolution();
		// The 3D world is centred at origin (0, 0, 0). The sector grid has an origin in the top
		// left at 2D world coords of (-map_x / 2, 0, map_z / 2).
		// To translate the 3D world
		// coords into a new coordinate system with a (0, 0, 0) origin in the top left we add
		// half the map dimension to each psition coordinatem
		let x_origin = position.x + (self.get_length() / 2) as f32;
		let z_origin = (self.get_depth() / 2) as f32 + position.z;
		// the grid IDs follow a (column, row) convention, by dividing the repositioned dimension
		// by the sector grid sizes and rounding down we determine the sector indices
		let mut column = (x_origin / (self.get_sector_resolution() as f32)).floor() as u32;
		let mut row = (z_origin / (self.get_sector_resolution() as f32)).floor() as u32;
		// safety for x-z being at the exact limits of map size
		if column >= x_sector_count {
			column = x_sector_count - 1;
		}
		if row >= z_sector_count {
			row = z_sector_count - 1;
		}
		Some(SectorID::new(column, row))
	}

	/// Calculate the `x, y, z` coordinates at the top-left corner of a sector based on map dimensions
	#[cfg(feature = "3d")]
	pub fn get_sector_corner_xyz(&self, sector_id: SectorID) -> Vec3 {
		// x sector-grid origin begins in the negative
		let x_origin = -(self.get_length() as f32) / 2.0;
		let x = x_origin + sector_id.get_column() as f32 * self.get_sector_resolution() as f32;
		// z sector grid origin begins in the negative
		let z_origin = -(self.get_depth() as f32) / 2.0;
		let z = z_origin + sector_id.get_row() as f32 * self.get_sector_resolution() as f32;
		Vec3::new(x, 0.0, z)
	}

	/// From a point in 3D space calcualte what Sector and field cell it resides in
	#[cfg(feature = "3d")]
	pub fn get_sector_and_field_cell_from_xyz(
		&self,
		position: Vec3,
	) -> Option<(SectorID, FieldCell)> {
		if let Some(sector_id) = self.get_sector_id_from_xyz(position) {
			let sector_corner_origin = self.get_sector_corner_xyz(sector_id);
			let field_id_0 = (position.x - sector_corner_origin.x).floor() as usize;
			let field_id_1 = (position.z - sector_corner_origin.z).floor() as usize;
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
			self.get_sector_resolution(),
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
			self.get_sector_resolution(),
		)
	}
	/// From an [Ordinal] get the ID of a neighbouring sector. Returns [None]
	/// if the sector would be out of bounds
	pub fn get_sector_id_from_ordinal(
		&self,
		ordinal: Ordinal,
		sector_id: &SectorID,
	) -> Option<SectorID> {
		match ordinal {
			Ordinal::North => sector_id
				.get_row()
				.checked_sub(1)
				.map(|row| SectorID::new(sector_id.get_column(), row)),
			Ordinal::East => {
				if sector_id.get_column() + 1 < self.get_length() / self.get_sector_resolution() - 1
				{
					Some(SectorID::new(
						sector_id.get_column() + 1,
						sector_id.get_row(),
					))
				} else {
					None
				}
			}
			Ordinal::South => {
				if sector_id.get_row() + 1 < self.get_depth() / self.get_sector_resolution() - 1 {
					Some(SectorID::new(
						sector_id.get_column(),
						sector_id.get_row() + 1,
					))
				} else {
					None
				}
			}
			Ordinal::West => sector_id
				.get_column()
				.checked_sub(1)
				.map(|column| SectorID::new(column, sector_id.get_row())),
			Ordinal::NorthEast => {
				if let Some(row) = sector_id.get_row().checked_sub(1) {
					if sector_id.get_column() + 1
						< self.get_length() / self.get_sector_resolution() - 1
					{
						Some(SectorID::new(sector_id.get_column() + 1, row))
					} else {
						None
					}
				} else {
					None
				}
			}
			Ordinal::SouthEast => {
				if sector_id.get_row() + 1 < self.get_depth() / self.get_sector_resolution() - 1 {
					if sector_id.get_column() + 1
						< self.get_length() / self.get_sector_resolution() - 1
					{
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
				if sector_id.get_row() + 1 < self.get_depth() / self.get_sector_resolution() - 1 {
					sector_id
						.get_column()
						.checked_sub(1)
						.map(|column| SectorID::new(column, sector_id.get_row() + 1))
				} else {
					None
				}
			}
			Ordinal::NorthWest => {
				if let Some(row) = sector_id.get_row().checked_sub(1) {
					sector_id
						.get_column()
						.checked_sub(1)
						.map(|column| SectorID::new(column, row))
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
	fn get_northern_sector_neighbours_with_drection() {
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
	fn get_eastern_sector_neighbours_with_drection() {
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
	fn get_southern_sector_neighbours_with_drection() {
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
	fn get_western_sector_neighbours_with_drection() {
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
	fn get_centre_sector_neighbours_with_drection() {
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
}
