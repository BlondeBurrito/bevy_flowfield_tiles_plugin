//! Defines the Bevy [Plugin] for FlowfieldTiles
//!

use bevy::prelude::*;

pub mod cost_layer;
pub mod flow_layer;

pub struct FlowFieldTilesPlugin;

impl Plugin for FlowFieldTilesPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<cost_layer::EventUpdateCostfieldsCell>()
			.add_event::<cost_layer::EventRebuildSectorPortals>()
			.add_event::<cost_layer::EventUpdatePortalGraph>()
			.add_event::<flow_layer::EventPathRequest>()
			.add_systems(
				Update,
				(
					cost_layer::process_costfields_updates,
					cost_layer::rebuild_portals,
					cost_layer::update_portal_graph,
				)
					.chain(),
			)
			.add_systems(
				Update,
				(
					flow_layer::handle_path_requests,
					flow_layer::generate_flow_fields,
				)
					.chain(),
			);
	}
}
