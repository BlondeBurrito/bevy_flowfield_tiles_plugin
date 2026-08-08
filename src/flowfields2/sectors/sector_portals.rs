//! A map is split into a series of `MxN` sectors where each has a number of
//! [Portals] for indicating points that can be used to path to neighbouring
//! sectors
//!
//!

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::flowfields2::{
	portal::portals::Portals,
	sectors::{MapDimensions, SectorID, sector_cost::SectorCostFields},
	utilities::FIELD_RESOLUTION,
};

/// Keys represent unique sector IDs and are in the format of `(column, row)` when considering a
/// grid of sectors across the map. The sectors begin in the top left of the map (-x_max, -z_max)
/// and values are the [Portals] associated with that sector
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct SectorPortals(BTreeMap<SectorID, Portals>);

impl SectorPortals {
	/// Create a new instance of [SectorPortals] with default [Portals]
	pub fn new(map_length: f32, map_depth: f32, world_unit_size: f32) -> Self {
		let mut map = BTreeMap::new();
		let column_count = map_length / (world_unit_size * FIELD_RESOLUTION as f32);
		let row_count = map_depth / (world_unit_size * FIELD_RESOLUTION as f32);
		for m in 0..column_count as i32 {
			for n in 0..row_count as i32 {
				map.insert(SectorID::new(m, n), Portals::default());
			}
		}
		SectorPortals(map)
	}
	/// Get a reference the map of [Portals]
	pub fn get(&self) -> &BTreeMap<SectorID, Portals> {
		&self.0
	}
	/// Get a mutable reference the map of [Portals]
	pub fn get_mut(&mut self) -> &mut BTreeMap<SectorID, Portals> {
		&mut self.0
	}
	/// Whenever a [CostField] is updated the [Portals] for that sector and neighbouring sectors
	/// need to be recalculated
	pub fn update_portals(
		&mut self,
		changed_cost_field_id: SectorID,
		sector_cost_fields: &SectorCostFields,
		map_dimensions: &MapDimensions,
	) -> &mut Self {
		let mut changed = map_dimensions.get_ids_of_neighbouring_sectors(&changed_cost_field_id);
		changed.push(changed_cost_field_id);
		for id in changed.iter() {
			self.get_mut().get_mut(id).unwrap().recalculate_portals(
				sector_cost_fields,
				id,
				map_dimensions,
			);
		}
		self
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {}
