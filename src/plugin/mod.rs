//! Defines the Bevy [Plugin] for FlowfieldTiles
//!

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
		app.add_event::<cost_layer::EventUpdateCostfieldsCell>()
			.add_event::<cost_layer::EventRebuildSectorPortals>()
			.add_event::<cost_layer::EventUpdatePortalGraph>()
			.add_event::<flow_layer::EventPathRequest>()
			// .configure_sets(
			// 	Update,
			// 	// chain() will ensure sets run in the order they are listed
			// 	(OrderingSet::Tidy, OrderingSet::Calculate).chain(),
			// )
			// .add_systems(
			// 	Update,
			// 	(
			// 		(flow_layer::cleanup_old_routes,
			// 		flow_layer::cleanup_old_flowfields,).in_set(OrderingSet::Tidy),
			// 		(cost_layer::process_costfields_updates,
			// 		cost_layer::rebuild_portals,
			// 		cost_layer::update_portal_graph,
			// 		flow_layer::handle_path_requests,
			// 		flow_layer::generate_flow_fields,).chain().in_set(OrderingSet::Calculate)
			// 	),
			// );
			.add_systems(
				Update,
				(
					cost_layer::process_costfields_updates,
					cost_layer::rebuild_portals,
					cost_layer::update_portal_graph,
					flow_layer::handle_path_requests,
					flow_layer::generate_flow_fields,
				)
					.chain(),
			);
		// .add_systems(
		// 	Update,
		// 	(
		// 		flow_layer::handle_path_requests,
		// 		flow_layer::generate_flow_fields,
		// 	)
		// 		.chain(),
		// );
	}
}
