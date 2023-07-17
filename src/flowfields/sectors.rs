//! A map is split into a series of `MxN` sectors composed of various fields used for path calculation
//!
//!

use std::collections::BTreeMap;

use crate::prelude::*;
use bevy::prelude::*;
//TODO: is this needed?
/// Shared behaviour of a sector
trait Sector {}

/// Keys represent unique sector IDs and are in the format of `(column, row)` when considering a
/// grid of sectors across the map. The sectors begin in the top left of the map (-x_max, -z_max)
/// and values are the [CostField] associated with that sector
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component)]
pub struct SectorCostFields(BTreeMap<(u32, u32), CostField>);

impl SectorCostFields {
	/// Create a new instance of [SectorCostFields] based on the map dimensions containing [CostField]
	pub fn new(map_x_dimension: u32, map_z_dimension: u32) -> Self {
		let mut map = BTreeMap::new();
		let column_count = map_x_dimension / SECTOR_RESOLUTION as u32;
		let row_count = map_z_dimension / SECTOR_RESOLUTION as u32;
		for m in 0..column_count {
			for n in 0..row_count {
				map.insert((m, n), CostField::default());
			}
		}
		SectorCostFields(map)
	}
	/// Get a reference to the map of sectors and [CostField]
	pub fn get(&self) -> &BTreeMap<(u32, u32), CostField> {
		&self.0
	}
	/// Get a mutable reference to the map of sectors and [CostField]
	pub fn get_mut(&mut self) -> &mut BTreeMap<(u32, u32), CostField> {
		&mut self.0
	}
	/// From a `ron` file generate the [SectorCostFields]
	#[cfg(feature = "ron")]
	pub fn from_file(path: String) -> Self {
		let file = std::fs::File::open(path).expect("Failed opening CostField file");
		let fields: SectorCostFields = match ron::de::from_reader(file) {
			Ok(fields) => fields,
			Err(e) => panic!("Failed deserializing SectorCostFields: {}", e),
		};
		fields
	}
}

/// Keys represent unique sector IDs and are in the format of `(column, row)` when considering a
/// grid of sectors across the map. The sectors begin in the top left of the map (-x_max, -z_max)
/// and values are the [Portals] associated with that sector
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component)]
pub struct SectorPortals(BTreeMap<(u32, u32), Portals>);

impl SectorPortals {
	/// Create a new instance of [SectorPortals] with default [Portals]
	pub fn new(map_x_dimension: u32, map_z_dimension: u32) -> Self {
		let mut map = BTreeMap::new();
		let column_count = map_x_dimension / SECTOR_RESOLUTION as u32;
		let row_count = map_z_dimension / SECTOR_RESOLUTION as u32;
		for m in 0..column_count {
			for n in 0..row_count {
				map.insert((m, n), Portals::default());
			}
		}
		SectorPortals(map)
	}
	/// Get a reference the map of [Portals]
	pub fn get(&self) -> &BTreeMap<(u32, u32), Portals> {
		&self.0
	}
	/// Get a mutable reference the map of [Portals]
	pub fn get_mut(&mut self) -> &mut BTreeMap<(u32, u32), Portals> {
		&mut self.0
	}
	/// Whenever a [CostField] is updated the [Portals] for that sector and neighbouring sectors
	/// need to be recalculated
	pub fn update_portals(
		&mut self,
		changed_cost_field_id: (u32, u32),
		sector_cost_fields: &SectorCostFields,
		map_x_dimension: u32,
		map_z_dimension: u32,
	) -> &mut Self {
		let mut changed = get_ids_of_neighbouring_sectors(
			&changed_cost_field_id,
			map_x_dimension,
			map_z_dimension,
		);
		changed.push(changed_cost_field_id);
		for id in changed.iter() {
			self.get_mut().get_mut(id).unwrap().recalculate_portals(
				sector_cost_fields,
				id,
				map_x_dimension,
				map_z_dimension,
			);
		}
		self
	}
}

// /// Keys represent unique sector IDs and are in the format of `(column, row)` when considering a
// /// grid of sectors across the map. The sectors begin in the top left of the map (-x_max, -z_max)
// /// and values are the [IntegrationField] associated with that sector
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Component)]
// pub struct SectorIntegrationFields(BTreeMap<(u32, u32), IntegrationField>);

// impl SectorIntegrationFields {
// 	/// Create a new instance of [SectorIntegrationFields] based on the map dimensions containing [IntegrationField]
// 	pub fn new(map_x_dimension: u32, map_z_dimension: u32) -> Self {
// 		let mut map = BTreeMap::new();
// 		let column_count = map_x_dimension / SECTOR_RESOLUTION as u32;
// 		let row_count = map_z_dimension / SECTOR_RESOLUTION as u32;
// 		for m in 0..column_count {
// 			for n in 0..row_count {
// 				map.insert((m, n), IntegrationField::default());
// 			}
// 		}
// 		SectorIntegrationFields(map)
// 	}
// 	/// Get a reference to the map of sectors and [IntegrationField]
// 	pub fn get(&self) -> &BTreeMap<(u32, u32), IntegrationField> {
// 		&self.0
// 	}
// 	/// Get a mutable reference to the map of sectors and [IntegrationField]
// 	pub fn get_mut(&mut self) -> &mut BTreeMap<(u32, u32), IntegrationField> {
// 		&mut self.0
// 	}
// }

/// A sector has up to four neighbours. Based on the ID of the sector and the dimensions
/// of the map retrieve the IDs neighbouring sectors
pub fn get_ids_of_neighbouring_sectors(
	sector_id: &(u32, u32),
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> Vec<(u32, u32)> {
	//top left                     // top right
	// has 2 valid neighbours      // has two valid neighbours
	// ___________                 // ___________
	// | x       |                 // |       x |
	// |x        |                 // |        x|
	// |         |                 // |         |
	// |         |                 // |         |
	// |_________|                 // |_________|
	// bottom right                // bottom left sector
	// has two valid neighbours    // has two valid neighbours
	// ___________                 // ___________
	// |         |                 // |         |
	// |         |                 // |         |
	// |         |                 // |         |
	// |        x|                 // |x        |
	// |_______x_|                 // |_x_______|
	// northern row minus          // eastern column minus
	// corners have three          // corners have three
	// valid neighbours            // valid neighbours
	// ___________                 // ___________
	// |x       x|                 // |        x|
	// |  xxxxx  |                 // |       x |
	// |         |                 // |       x |
	// |         |                 // |       x |
	// |_________|                 // |________x|
	// southern row minus          // western column minus
	// corners have three          // corners have three
	// valid neighbours            // valid neighbours
	// ___________                 // ___________
	// |         |                 // |x        |
	// |         |                 // | x       |
	// |         |                 // | x       |
	// | xxxxxxx |                 // | x       |
	// |x       x|                 // |x________|
	// all other sectors not along an edge of the map have four valid sectors for portals
	// ___________
	// |         |
	// |    x    |
	// |   x x   |
	// |    x    |
	// |_________|
	Ordinal::get_sector_neighbours(sector_id, map_x_dimension, map_z_dimension)
}

/// A sector has up to four neighbours. Based on the ID of the sector and the dimensions
/// of the map retrieve the IDs neighbouring sectors and the [Ordinal] direction from the
/// current sector that that sector is found in
pub fn get_ordinal_and_ids_of_neighbouring_sectors(
	sector_id: &(u32, u32),
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> Vec<(Ordinal, (u32, u32))> {
	//top left                     // top right
	// has 2 valid neighbours      // has two valid neighbours
	// ___________                 // ___________
	// | x       |                 // |       x |
	// |x        |                 // |        x|
	// |         |                 // |         |
	// |         |                 // |         |
	// |_________|                 // |_________|
	// bottom right                // bottom left sector
	// has two valid neighbours    // has two valid neighbours
	// ___________                 // ___________
	// |         |                 // |         |
	// |         |                 // |         |
	// |         |                 // |         |
	// |        x|                 // |x        |
	// |_______x_|                 // |_x_______|
	// northern row minus          // eastern column minus
	// corners have three          // corners have three
	// valid neighbours            // valid neighbours
	// ___________                 // ___________
	// |x       x|                 // |        x|
	// |  xxxxx  |                 // |       x |
	// |         |                 // |       x |
	// |         |                 // |       x |
	// |_________|                 // |________x|
	// southern row minus          // western column minus
	// corners have three          // corners have three
	// valid neighbours            // valid neighbours
	// ___________                 // ___________
	// |         |                 // |x        |
	// |         |                 // | x       |
	// |         |                 // | x       |
	// | xxxxxxx |                 // | x       |
	// |x       x|                 // |x________|
	// all other sectors not along an edge of the map have four valid sectors for portals
	// ___________
	// |         |
	// |    x    |
	// |   x x   |
	// |    x    |
	// |_________|
	Ordinal::get_sector_neighbours_with_ordinal(sector_id, map_x_dimension, map_z_dimension)
}

/// From the position of a `cell_id`, if it sits along a boundary, return the [Ordinal] of that boundary. Note that if the `cell_id` is in a field corner then it'll have two boundaries. Note that if the `cell_id` is not in fact along a boundary then this will panic
pub fn get_boundary_ordinal_from_grid_cell(cell_id: &(usize, usize)) -> Vec<Ordinal> {
	let mut boundaries = Vec::new();
	if cell_id.1 == 0 {
		boundaries.push(Ordinal::North);
	}
	if cell_id.0 == FIELD_RESOLUTION - 1 {
		boundaries.push(Ordinal::East);
	}
	if cell_id.1 == FIELD_RESOLUTION - 1 {
		boundaries.push(Ordinal::South);
	}
	if cell_id.0 == 0 {
		boundaries.push(Ordinal::West);
	}
	if !boundaries.is_empty() {
		boundaries
	} else {
		panic!("Grid cell {:?} does not sit along the boundary", cell_id);
	}
}
/// From a position in 2D `x, y` space with an origin at `(0, 0)` and the
/// dimensions (pixels) of the map, calculate the sector ID that point resides in
///
/// `pixel_scale` refers to the dimensions of your map sprites, not that their `x` and `y` dimensions must be the same, i.e a square shape
pub fn get_sector_id_from_xy(
	position: Vec2,
	x_dimension_pixels: u32,
	y_dimension_pixels: u32,
	pixel_scale: f32,
) -> Option<(u32, u32)> {
	if position.x < -((x_dimension_pixels / 2) as f32)
		|| position.x > (x_dimension_pixels / 2) as f32
		|| position.y < -((y_dimension_pixels / 2) as f32)
		|| position.y > (y_dimension_pixels / 2) as f32
	{
		error!("OOB pos, x {}, y {}", position.x, position.y);
		return None;
	}
	let x_sector_count = x_dimension_pixels / SECTOR_RESOLUTION as u32;
	let y_sector_count = y_dimension_pixels / SECTOR_RESOLUTION as u32;
	// The 2D world is centred at origin (0, 0). The sector grid has an origin in the top
	// left at 2D world coords of (-map_x * pixel_scale / 2, 0, map_y * pixel_scale / 2).
	// To translate the 2D world
	// coords into a new coordinate system with a (0, 0) origin in the top left we add
	// half the map dimension to each psition coordinatem
	let x_origin = position.x + (x_dimension_pixels / 2) as f32;
	let y_origin = (y_dimension_pixels / 2) as f32 - position.y;
	// the grid IDs follow a (column, row) convention, by dividing the repositioned dimension
	// by the sector grid sizes and rounding down we determine the sector indices
	let mut column = (x_origin / (pixel_scale * SECTOR_RESOLUTION as f32)).floor() as u32;
	let mut row = (y_origin / (pixel_scale * SECTOR_RESOLUTION as f32)).floor() as u32;
	// safety for x-y being at the exact limits of map size
	if column >= x_sector_count {
		column = x_sector_count - 1;
	}
	if row >= y_sector_count {
		row = y_sector_count - 1;
	}
	Some((column, row))
}
/// Get the `(x,y)` coordinates of the top left corner of a sector in real space
pub fn get_sector_xy_at_top_left(
	sector_id: (u32, u32),
	map_x_dimension: u32,
	map_y_dimension: u32,
	pixel_scale: f32,
) -> Vec2 {
	// x sector-grid origin begins in the negative
	let x_origin = -(map_x_dimension as f32) / 2.0;
	let sprite_length_of_sector = pixel_scale * SECTOR_RESOLUTION as f32;
	let x = x_origin + sector_id.0 as f32 * sprite_length_of_sector;
	// y sector grid origin begins in the positive
	let y_origin = map_y_dimension as f32 / 2.0;
	let y = y_origin - sector_id.1 as f32 * sprite_length_of_sector;
	Vec2::new(x, y)
}

// pub fn get_field_cell_from_xy(
// 	position: Vec2,
// 	map_x_dimension: u32,
// 	map_y_dimension: u32,
// 	pixel_scale: f32,
// 	sector_id: (u32, u32),
// ) {
// 	let sector_id = get_sector_id_from_xy(position, map_x_dimension, map_y_dimension, pixel_scale);
// 	let sector_corner_origin =
// 		get_sector_xy_at_top_left(position, map_x_dimension, map_y_dimension, pixel_scale);
// }

pub fn get_sector_and_field_id_from_xy(
	position: Vec2,
	map_x_dimension: u32,
	map_y_dimension: u32,
	pixel_scale: f32,
) -> Option<((u32, u32), (usize, usize))> {
	if let Some(sector_id) =
		get_sector_id_from_xy(position, map_x_dimension, map_y_dimension, pixel_scale)
	{
		let sector_corner_origin =
			get_sector_xy_at_top_left(sector_id, map_x_dimension, map_y_dimension, pixel_scale);
		let field_id_0 = ((position.x - sector_corner_origin.x) / pixel_scale).floor() as usize;
		let field_id_1 =
			((-position.y + sector_corner_origin.y) / pixel_scale).floor() as usize;
		let field_id = (field_id_0, field_id_1);
		return Some((sector_id, field_id));
	}
	None
}
//TODO fix me
/// From a position in `x, y, z` space and the dimensions of the map calcualte
/// the sector ID that point resides in
pub fn get_sector_id_from_xyz(
	position: Vec3,
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> (u32, u32) {
	//TODO test whether position is outside map dimensions
	let x_sector_count = map_x_dimension / SECTOR_RESOLUTION as u32;
	let z_sector_count = map_z_dimension / SECTOR_RESOLUTION as u32;
	// The 3D world is centred at origin (0, 0, 0). The sector grid has an origin in the top
	// left at 3D world coords of (-map_x / 2, 0, -map_z / 2). To translate the 3D world
	// coords into a new coordinate system with a (0, 0, 0) origin in the top left we add
	// half the map dimension to each psition coordinatem
	let x_origin = position.x + (map_x_dimension / 2) as f32;
	let z_origin = position.z + (map_z_dimension / 2) as f32;
	// the grid IDs follow a (column, row) convention, by dividing the repositioned dimension
	// by the sector grid sizes and rounding down we determine the sector indices
	let mut column = (x_origin / SECTOR_RESOLUTION as f32).floor() as u32;
	let mut row = (z_origin / SECTOR_RESOLUTION as f32).floor() as u32;
	// safety for x-y being at the exact limits of map size
	if column >= x_sector_count {
		column = x_sector_count - 1;
	}
	if row >= z_sector_count {
		row = z_sector_count - 1;
	}
	(column, row)
}
//TODO fix me
pub fn get_field_cell_from_xyz(
	position: Vec3,
	sector_id: (u32, u32),
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> (usize, usize) {
	let origin_of_sector =
		get_xyz_at_sector_top_left_from_sector_id(sector_id, map_x_dimension, map_z_dimension);

	let mut column = ((origin_of_sector.x - position.x).abs()).floor() as usize;
	let mut row = ((origin_of_sector.z - position.z).abs()).floor() as usize;

	if column >= FIELD_RESOLUTION {
		column = FIELD_RESOLUTION - 1;
	}
	if row >= FIELD_RESOLUTION {
		row = FIELD_RESOLUTION - 1;
	}
	(column, row)
}
//TODO doesn;t work
/// From a point in 3D space calcualte what Sector and field cell it resides in
pub fn get_sector_and_field_cell_from_xyz(
	position: Vec3,
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> ((u32, u32), (usize, usize)) {
	let sector_id = get_sector_id_from_xyz(position, map_x_dimension, map_z_dimension);
	let field_cell = get_field_cell_from_xyz(position, sector_id, map_x_dimension, map_z_dimension);
	(sector_id, field_cell)
}
//TODO fix and test me
/// Calculate the `x, y, z` coordinates at the top-left corner of a sector based on map dimensions
pub fn get_xyz_at_sector_top_left_from_sector_id(
	sector_id: (u32, u32),
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> Vec3 {
	let x = (sector_id.0 as i32 * SECTOR_RESOLUTION as i32 - (map_x_dimension / 2) as i32) as f32;
	let z = (sector_id.1 as i32 * SECTOR_RESOLUTION as i32 - (map_z_dimension / 2) as i32) as f32;
	Vec3::new(x, 0.0, z)
}
//TODO fix and test me
/// Calculate the `x, y, z` coordinates at the top-left corner of a sector based on map dimensions
pub fn get_xyz_sector_centre_from_sector_id(
	sector_id: (u32, u32),
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> Vec3 {
	let x = (sector_id.0 as i32 * SECTOR_RESOLUTION as i32 - (map_x_dimension / 2) as i32) as f32
		+ (SECTOR_RESOLUTION / 2) as f32;
	let z = (sector_id.1 as i32 * SECTOR_RESOLUTION as i32 - (map_z_dimension / 2) as i32) as f32
		+ (SECTOR_RESOLUTION / 2) as f32;
	Vec3::new(x, 0.0, z)
}
//TODO fix and test me
/// Calculate the real world `x, y, z` coordinates at the cetnre of a field cell within a sector based on map dimensions
pub fn get_xyz_from_field_cell_within_sector(
	sector_id: (u32, u32),
	field_id: (usize, usize),
	map_x_dimension: u32,
	map_z_dimension: u32,
) -> Vec3 {
	let sector_xyz =
		get_xyz_at_sector_top_left_from_sector_id(sector_id, map_x_dimension, map_z_dimension);
	let x_offset = (field_id.0 + 1) as f32 * 0.5;
	let z_offset = (field_id.1 + 1) as f32 * 0.5;

	Vec3::new(sector_xyz.x + x_offset, 0.0, sector_xyz.z + z_offset)
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn sector_costfields_top_left_sector_id_from_xyz() {
		let map_x_dimension = 20;
		let map_z_dimension = 20;
		let position = Vec3::new(-5.0, 0.0, -5.0);
		let result = get_sector_id_from_xyz(position, map_x_dimension, map_z_dimension);
		let actual: (u32, u32) = (0, 0);
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_costfields_top_right_sector_id_from_xyz() {
		let map_x_dimension = 20;
		let map_z_dimension = 20;
		let position = Vec3::new(5.0, 0.0, -5.0);
		let result = get_sector_id_from_xyz(position, map_x_dimension, map_z_dimension);
		let actual: (u32, u32) = (1, 0);
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_costfields_bottom_right_sector_id_from_xyz() {
		let map_x_dimension = 20;
		let map_z_dimension = 20;
		let position = Vec3::new(5.0, 0.0, 5.0);
		let result = get_sector_id_from_xyz(position, map_x_dimension, map_z_dimension);
		let actual: (u32, u32) = (1, 1);
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_costfields_bottom_left_sector_id_from_xyz() {
		let map_x_dimension = 20;
		let map_z_dimension = 20;
		let position = Vec3::new(-5.0, 0.0, 5.0);
		let result = get_sector_id_from_xyz(position, map_x_dimension, map_z_dimension);
		let actual: (u32, u32) = (0, 1);
		assert_eq!(actual, result);
	}
	#[test]
	fn get_northern_sector_neighbours() {
		let sector_id = (4, 0);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ids_of_neighbouring_sectors(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(5, 0), (4, 1), (3, 0)];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_eastern_sector_neighbours() {
		let sector_id = (19, 3);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ids_of_neighbouring_sectors(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(19, 2), (19, 4), (18, 3)];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_southern_sector_neighbours() {
		let sector_id = (5, 19);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ids_of_neighbouring_sectors(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(5, 18), (6, 19), (4, 19)];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_western_sector_neighbours() {
		let sector_id = (0, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ids_of_neighbouring_sectors(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(0, 4), (1, 5), (0, 6)];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_centre_sector_neighbours() {
		let sector_id = (5, 7);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ids_of_neighbouring_sectors(&sector_id, map_x_dimension, map_z_dimension);
		let actual = vec![(5, 6), (6, 7), (5, 8), (4, 7)];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_northern_sector_neighbours_with_drection() {
		let sector_id = (4, 0);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ordinal_and_ids_of_neighbouring_sectors(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = vec![
			(Ordinal::East, (5, 0)),
			(Ordinal::South, (4, 1)),
			(Ordinal::West, (3, 0)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_eastern_sector_neighbours_with_drection() {
		let sector_id = (19, 3);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ordinal_and_ids_of_neighbouring_sectors(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = vec![
			(Ordinal::North, (19, 2)),
			(Ordinal::South, (19, 4)),
			(Ordinal::West, (18, 3)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_southern_sector_neighbours_with_drection() {
		let sector_id = (5, 19);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ordinal_and_ids_of_neighbouring_sectors(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = vec![
			(Ordinal::North, (5, 18)),
			(Ordinal::East, (6, 19)),
			(Ordinal::West, (4, 19)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_western_sector_neighbours_with_drection() {
		let sector_id = (0, 5);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ordinal_and_ids_of_neighbouring_sectors(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = vec![
			(Ordinal::North, (0, 4)),
			(Ordinal::East, (1, 5)),
			(Ordinal::South, (0, 6)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn get_centre_sector_neighbours_with_drection() {
		let sector_id = (5, 7);
		let map_x_dimension = 200;
		let map_z_dimension = 200;
		let result = get_ordinal_and_ids_of_neighbouring_sectors(
			&sector_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = vec![
			(Ordinal::North, (5, 6)),
			(Ordinal::East, (6, 7)),
			(Ordinal::South, (5, 8)),
			(Ordinal::West, (4, 7)),
		];
		assert_eq!(actual, result);
	}
	#[test]
	fn sector_xyz_corner_zero() {
		let sector_id = (0, 0);
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let result =
			get_xyz_at_sector_top_left_from_sector_id(sector_id, map_x_dimension, map_z_dimension);
		let actual = Vec3::new(-15.0, 0.0, -15.0);
		assert_eq!(actual, result)
	}
	#[test]
	fn sector_xyz_corner_centre() {
		let sector_id = (1, 1);
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let result =
			get_xyz_at_sector_top_left_from_sector_id(sector_id, map_x_dimension, map_z_dimension);
		let actual = Vec3::new(-5.0, 0.0, -5.0);
		assert_eq!(actual, result)
	}
	#[test]
	fn sector_xyz_centre_zero() {
		let sector_id = (0, 0);
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let result =
			get_xyz_sector_centre_from_sector_id(sector_id, map_x_dimension, map_z_dimension);
		let actual = Vec3::new(-10.0, 0.0, -10.0);
		assert_eq!(actual, result)
	}
	#[test]
	fn sector_xyz_centre_centre() {
		let sector_id = (1, 1);
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let result =
			get_xyz_sector_centre_from_sector_id(sector_id, map_x_dimension, map_z_dimension);
		let actual = Vec3::new(0.0, 0.0, 0.0);
		assert_eq!(actual, result)
	}
	#[test]
	fn field_xyz() {
		let sector_id = (0, 0);
		let field_id = (0, 0);
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let result = get_xyz_from_field_cell_within_sector(
			sector_id,
			field_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = Vec3::new(-14.5, 0.0, -14.5);
		assert_eq!(actual, result)
	}
	#[test]
	fn field_xyz2() {
		let sector_id = (1, 1);
		let field_id = (4, 4);
		let map_x_dimension = 30;
		let map_z_dimension = 30;
		let result = get_xyz_from_field_cell_within_sector(
			sector_id,
			field_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = Vec3::new(-2.5, 0.0, -2.5);
		assert_eq!(actual, result)
	}
	#[test]
	fn field_xyz3() {
		let sector_id = (2, 3);
		let field_id = (0, 0);
		let map_x_dimension = 100;
		let map_z_dimension = 100;
		let result = get_xyz_from_field_cell_within_sector(
			sector_id,
			field_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = Vec3::new(-29.5, 0.0, -19.5);
		assert_eq!(actual, result)
	}
	#[test]
	fn field_xyz4() {
		let sector_id = (2, 3);
		let field_id = (3, 6);
		let map_x_dimension = 100;
		let map_z_dimension = 100;
		let result = get_xyz_from_field_cell_within_sector(
			sector_id,
			field_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = Vec3::new(-28.0, 0.0, -16.5);
		assert_eq!(actual, result)
	}
	#[test]
	fn field_xyz5() {
		let sector_id = (4, 4);
		let field_id = (9, 9);
		let map_x_dimension = 100;
		let map_z_dimension = 100;
		let result = get_xyz_from_field_cell_within_sector(
			sector_id,
			field_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = Vec3::new(-5.0, 0.0, -5.0);
		assert_eq!(actual, result)
	}
	#[test]
	fn field_xyz6() {
		let sector_id = (2, 2);
		let field_id = (5, 5);
		let map_x_dimension = 100;
		let map_z_dimension = 100;
		let result = get_xyz_from_field_cell_within_sector(
			sector_id,
			field_id,
			map_x_dimension,
			map_z_dimension,
		);
		let actual = Vec3::new(-27.0, 0.0, -27.0);
		assert_eq!(actual, result)
	}
	#[test]
	#[cfg(feature = "ron")]
	fn sector_cost_fields_file() {
		let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
		let _cost_fields = SectorCostFields::from_file(path);
	}
	#[test]
	fn sector_from_xy_none() {
		let dimensions = (1280, 1280);
		let pixel_scale = 64.0;
		let position = Vec2::new(-1500.0, 0.0);
		let result = get_sector_id_from_xy(position, dimensions.0, dimensions.1, pixel_scale);

		assert!(result.is_none());
	}
	#[test]
	fn sector_from_xy() {
		let dimensions = (1280, 1280);
		let pixel_scale = 64.0;
		let position = Vec2::new(530.0, 75.0);
		let result = get_sector_id_from_xy(position, dimensions.0, dimensions.1, pixel_scale);
		let actual = (1, 0);
		assert_eq!(actual, result.unwrap());
	}
}
