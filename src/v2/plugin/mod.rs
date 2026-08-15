//! Defines the Bevy [Plugin] for FlowfieldTiles
//!

use std::sync::{Arc, RwLock};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crate::v2::{
	bundle::FlowFieldTiles,
	flowfields::{fields::FieldCell, sectors::SectorID},
};

pub struct FlowFieldTilesPlugin;

impl Plugin for FlowFieldTilesPlugin {
	fn build(&self, app: &mut App) {}
}

// handle taking cost updates
//
// handle path requests
//
//

fn cba() {
	let a = Arc::new(RwLock::new(8));

	let lock_data = a.clone();
	lock_data.write();
}

fn abc(flowfield_tiles: &mut FlowFieldTiles) {
	let test = Some(8);
	if let Some(v) = test {
		let thread_pool = AsyncComputeTaskPool::get();

		let a = thread_pool.spawn(async move {
			let costfields = flowfield_tiles.get_sector_cost_fields();
			let sector = &SectorID::new(1, 1);
			let field_cell = FieldCell::new(1, 1);
			let cost = 255;
			let dimensions = flowfield_tiles.get_dimensions();
			costfields.set_field_cost(sector, &field_cell, cost, dimensions);
		});
	}
}
