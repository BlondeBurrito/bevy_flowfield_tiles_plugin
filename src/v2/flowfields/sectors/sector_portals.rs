//! A map is split into a series of `MxN` sectors where each has a number of
//! [Portals] for indicating points that can be used to path to neighbouring
//! sectors
//!

use bevy::prelude::*;
use std::collections::BTreeMap;

use crate::v2::flowfields::{dimensions::Dimensions, sectors::SectorID};

// /// Keys represent unique sector IDs and are in the format of `(column, row)` when considering a
// /// grid of sectors across the map. The sectors begin in the top left of the map (-x_max, -z_max)
// /// and values are the [Portals] associated with that sector
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[derive(Component, Clone, Debug, Reflect)]
// #[reflect(Component)]
// pub struct SectorPortals(BTreeMap<SectorID, Portals>);

// impl SectorPortals {
// 	/// Init empty [SectorPortals]
// 	pub fn new(dimensions: Dimensions) -> Self {
// 		let mut map = BTreeMap::new();
// 		for col in 0..dimensions.get_sector_column_count() {
// 			for row in 0..dimensions.get_sector_row_count() {
// 				map.insert(SectorID::new(col as i32, row as i32), Portal::default());
// 			}
// 		}
// 		SectorPortals(map)
// 	}
// }
