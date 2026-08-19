//! Stores generated [FlowField]s
//!

use std::collections::BTreeMap;

use crate::flowfields::{fields::flow_field::FlowField, route::RouteStep, sectors::SectorID};

/// Stores generated flowfields that actors can read
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct FlowFieldCache {
	/// Cache of [FlowFields] keyed based on goal/exit portal [RouteStep]
	cache: BTreeMap<RouteStep, FlowField>,
}

impl FlowFieldCache {
	/// Get a reference to the [FlowField] cache map
	pub fn get_cache(&self) -> &BTreeMap<RouteStep, FlowField> {
		&self.cache
	}
	/// Insert a [FlowField] into the cache
	pub fn insert(&mut self, route_step: &RouteStep, flowfield: FlowField) {
		self.cache.insert(*route_step, flowfield);
	}
	/// Remove a [FlowField] from the cache
	pub fn remove(&mut self, route_step: &RouteStep) {
		self.cache.remove(route_step);
	}
	//TODO give this a better name to avoid confusion with get_cache
	/// Get a [FlowField] for a particular [RouteStep] if it exists/has been
	/// generated
	pub fn get(&self, route_step: &RouteStep) -> Option<&FlowField> {
		self.cache.get(route_step)
	}
	/// Using a slice of sectors remove any [FlowField]s that correspond to the sectors
	pub fn remove_steps_with_sectors(&mut self, sectors: &[SectorID]) {
		let mut keys = vec![];
		for step in self.cache.keys() {
			if sectors.contains(step.get_sector()) {
				keys.push(*step);
			}
		}
		for step in keys {
			self.cache.remove(&step);
		}
	}
}
