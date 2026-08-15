//! Defines the Bevy [Plugin] for FlowfieldTiles
//!

use bevy::{
	prelude::*,
	tasks::{AsyncComputeTaskPool, futures::check_ready},
};

use crate::v2::bundle::FlowFieldTiles;

pub struct FlowFieldTilesPlugin;

impl Plugin for FlowFieldTilesPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PostUpdate, process_update_queue);
	}
}

// handle taking cost updates
//
// handle path requests
//
//

/// Process the queue of [CostField] updates and schedule portal recalculation
fn process_update_queue(mut query: Query<&mut FlowFieldTiles>) {
	for mut flowfield_tiles in &mut query {
		// if a portal update is still in process then come back later
		if let Some(portal_task) = &mut flowfield_tiles.portal_update_task {
			if portal_task.is_finished() {
				flowfield_tiles.portal_update_task = None;
				//TODO use the sector to identify is_dirty routes and flows
			} else {
				return;
			}
		}
		// if no task is already being processed set up another one if possible
		if flowfield_tiles.costfield_update_task.is_none() {
			if let Some(item) = flowfield_tiles.costfield_update_queue.pop_front() {
				// get the parameters to facilitate updating a costfield
				//
				// increment arc
				let costfields = flowfield_tiles.get_sector_cost_fields_mut().clone();
				let dimensions = flowfield_tiles.get_dimensions().clone();
				// clone so ownership can be moved inside the async pool
				let sector = item.sector().clone();
				let field_cell = item.cell().clone();
				let cost = item.cost();

				let thread_pool = AsyncComputeTaskPool::get();
				let task = thread_pool.spawn(async move {
					let mut costfields = costfields.write().unwrap();
					costfields.set_field_cost(&sector, &field_cell, cost, &dimensions);
					sector
				});
				flowfield_tiles.costfield_update_task = Some(task);
			}
		} else {
			// see if the costfield update is done
			let mut cost_task = flowfield_tiles.costfield_update_task.as_mut().unwrap();
			if let Some(sector) = check_ready(&mut cost_task) {
				// clear the costfield update task for a future tick to use
				flowfield_tiles.costfield_update_task = None;
				// costfields are updated, time to schedule a portal update
				//
				// increment arcs
				let costfields = flowfield_tiles.get_sector_cost_fields_mut().clone();
				let portals = flowfield_tiles.portals.clone();

				let thread_pool = AsyncComputeTaskPool::get();
				let task = thread_pool.spawn(async move {
					let costfields = costfields.read().unwrap();
					let mut portals = portals.write().unwrap();
					portals.update_portals(&sector, &*costfields);
					sector
				});
				flowfield_tiles.portal_update_task = Some(task);
			}
		}
	}
}
