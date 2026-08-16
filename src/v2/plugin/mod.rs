//! Defines the Bevy [Plugin] for FlowfieldTiles
//!

use bevy::{
	prelude::*,
	tasks::{AsyncComputeTaskPool, futures::check_ready},
};

use crate::v2::{
	bundle::FlowFieldTiles,
	flowfields::fields::{flow_field::FlowField, integration_field::IntegrationField},
};

/// Sets to group plugin logic
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum OrderingSet {
	Calculate,
}

pub struct FlowFieldTilesPlugin;

impl Plugin for FlowFieldTilesPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(PostUpdate, OrderingSet::Calculate);
		app.add_systems(
			PostUpdate,
			(process_costfield_update_queue, process_flow_queue).in_set(OrderingSet::Calculate),
		);
	}
}

/// Process the queue of [CostField] updates and schedule portal recalculation
fn process_costfield_update_queue(mut query: Query<&mut FlowFieldTiles>) {
	for mut flowfield_tiles in &mut query {
		// if flowfields are currently being generated then skip until they are done
		if flowfield_tiles.flow_gen_task.is_some() {
			return;
		}

		// if a portal update is still in process then come back later
		if let Some(portal_task) = &mut flowfield_tiles.portal_update_task {
			if let Some(sector) = check_ready(portal_task) {
				flowfield_tiles.portal_update_task = None;
				// use the sector to identify existing flows for removal
				// as they may now be out of date
				let sectors = sector.get_surrounding_sectors();
				flowfield_tiles
					.flowfield_cache
					.remove_steps_with_sectors(&sectors);
				// wipe out any queued steps for flow gen as they might be out of date
				let mut write_flow_queue = flowfield_tiles.flow_queue.write().unwrap();
				write_flow_queue.retain(|r| {
					for a in r {
						if sectors.contains(a.get_sector()) {
							return false;
						}
					}
					true
				});
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

				// let the update be handled in the background so as not to block the main
				// thread
				let thread_pool = AsyncComputeTaskPool::get();
				let task = thread_pool.spawn(async move {
					let mut costfields = costfields.write().unwrap();
					costfields.set_field_cost(&sector, &field_cell, cost, &dimensions);
					sector
				});
				// store the task so it can be polled/checked for completion
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

				// let the update be handled in the background so as not to block the main
				// thread
				let thread_pool = AsyncComputeTaskPool::get();
				let task = thread_pool.spawn(async move {
					let costfields = costfields.read().unwrap();
					let mut portals = portals.write().unwrap();
					portals.update_portals(&sector, &*costfields);
					sector
				});
				// store the task so it can be polled/checked for completion
				flowfield_tiles.portal_update_task = Some(task);
			}
		}
	}
}

/// Process the queue of routes and generate [FlowField]s
fn process_flow_queue(mut query: Query<&mut FlowFieldTiles>) {
	for mut flowfield_tiles in &mut query {
		// only proceed with generating flows if no updates are currently active for
		// costfields/portals
		if flowfield_tiles.costfield_update_task.is_some()
			|| flowfield_tiles.portal_update_task.is_some()
		{
			return;
		}
		// if an existing flow generation has finished add it to the cache
		if let Some(mut poll) = flowfield_tiles.flow_gen_task.as_mut() {
			if let Some(fields) = check_ready(&mut poll) {
				for (step, flowfield) in fields {
					flowfield_tiles.flowfield_cache.insert(&step, flowfield);
				}
				flowfield_tiles.flow_gen_task = None;
				return;
			}
		}

		// add groups of routes to the queue
		let costfields = flowfield_tiles.sector_cost_fields.clone();
		let queue = flowfield_tiles.flow_queue.clone();

		let thread_pool = AsyncComputeTaskPool::get();
		let task = thread_pool.spawn(async move {
			let read_costfields = costfields.read().unwrap();
			let mut write_queue = queue.write().unwrap();

			// if a route is waiting process it
			let mut generated = vec![];
			if let Some(route) = write_queue.pop_front() {
				for step in route.iter() {
					let sector = step.get_sector();
					let scaled_costfields = read_costfields.get_scaled_costs();
					let scaled_costfield = scaled_costfields.get(sector).unwrap();

					let mut integrationfield = IntegrationField::init(scaled_costfield, step);
					integrationfield.build(scaled_costfield);

					let mut flowfield = FlowField::new(step, &integrationfield);
					flowfield.build(&integrationfield);

					generated.push((*step, flowfield));
				}
			}
			generated
		});
		flowfield_tiles.flow_gen_task = Some(task);
	}
}
