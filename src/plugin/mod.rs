//! Defines the Bevy [Plugin] for FlowfieldTiles
//!

use crate::prelude::*;
use bevy::prelude::*;

pub mod cost_layer;
pub mod flow_layer;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum OrderingSet {
	Tidy,
	Calculate,
}

pub struct FlowFieldTilesPlugin;

impl Plugin for FlowFieldTilesPlugin {
	#[cfg(not(tarpaulin_include))]
	fn build(&self, app: &mut App) {
		app.register_type::<Ordinal>()
			.register_type::<MapDimensions>()
			.register_type::<CostField>()
			.register_type::<Portals>()
			.register_type::<FlowField>()
			.register_type::<SectorID>()
			.register_type::<FieldCell>()
			.register_type::<RouteMetadata>()
			.register_type::<FlowFieldMetadata>()
			.add_event::<cost_layer::EventUpdateCostfieldsCell>()
			.add_event::<cost_layer::EventCleanCaches>()
			.add_event::<flow_layer::EventPathRequest>()
			.configure_sets(Update, (OrderingSet::Tidy, OrderingSet::Calculate).chain())
			.add_systems(
				Update,
				(
					(
						flow_layer::cleanup_old_routes,
						flow_layer::cleanup_old_flowfields,
					)
						.in_set(OrderingSet::Tidy),
					(
						cost_layer::process_costfields_updates,
						// cost_layer::rebuild_portals,
						// cost_layer::update_portal_graph,
						cost_layer::clean_cache,
						flow_layer::event_insert_route_queue,
						flow_layer::process_route_queue,
						flow_layer::create_queued_integration_fields,
						flow_layer::create_flow_fields,
					)
						.chain()
						.in_set(OrderingSet::Calculate),
				),
			);
	}
}
