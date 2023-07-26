//! A map is split into a series of `MxN` sectors composed of various fields used for path calculation
//!
//!

use std::collections::BTreeMap;

use crate::prelude::*;
use bevy::prelude::*;

/// Keys represent unique sector IDs and are in the format of `(column, row)` when considering a
/// grid of sectors across the map. The sectors begin in the top left of the map (-x_max, -z_max)
/// and values are the [Portals] associated with that sector
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Clone)]
pub struct SectorPortals(BTreeMap<SectorID, Portals>);

impl SectorPortals {
	/// Create a new instance of [SectorPortals] with default [Portals]
	pub fn new(map_x_dimension: u32, map_z_dimension: u32, sector_resolution: u32) -> Self {
		let mut map = BTreeMap::new();
		let column_count = map_x_dimension / sector_resolution;
		let row_count = map_z_dimension / sector_resolution;
		for m in 0..column_count {
			for n in 0..row_count {
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
		map_dimensions: &MapDimensions
	) -> &mut Self {
		let mut changed = map_dimensions.get_ids_of_neighbouring_sectors(
			&changed_cost_field_id,
		);
		changed.push(changed_cost_field_id);
		for id in changed.iter() {
			self.get_mut().get_mut(id).unwrap().recalculate_portals(
				sector_cost_fields,
				id,
				map_dimensions
			);
		}
		self
	}
}

// #[rustfmt::skip]
#[cfg(test)]
mod tests {}
