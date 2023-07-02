//! Defines the Bevy [Plugin] for FlowfieldTiles
//!

use bevy::prelude::*;

pub mod cost_layer;

pub struct FlowFieldTilesPlugin;

impl Plugin for FlowFieldTilesPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<cost_layer::EventUpdateCostfieldsCell>()
			.add_event::<cost_layer::EventRebuildSectorPortals>()
			.add_event::<cost_layer::EventUpdatePortalGraph>()
			.add_systems(Update, (cost_layer::process_costfields_updates,))
			.add_systems(Update, (cost_layer::rebuild_portals, cost_layer::update_portal_graph).chain());
	}
}

