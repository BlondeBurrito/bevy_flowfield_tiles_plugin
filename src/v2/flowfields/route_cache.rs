//! When an actor requests a path and the floe fields are net yet ready they are given temporary simpler path based on portal-to-portal pathing
//!
//! The [RouteCache] stores the high level routes describing a path from sector-to-sector via portals
//!

use bevy::prelude::*;

use crate::v2::flowfields::{portal::PortalWindow, sectors::SectorID};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct RouteCache;

struct Route {
	/// Indicates this route is out of date and an attempt will be made to regenerate it
	is_dirty: bool,
	source_sector: SectorID,
	source_portal: PortalWindow,
	target_sector: SectorID,
	target_portal: PortalWindow,
}

struct RoutePath(Vec<Vec3>);

struct PathBroker;

impl PathBroker {
	async fn get() -> Option<PathId> {
		None
	}
}

enum PathId {
	Route,
	Flow,
}

async fn example() -> Option<PathId> {
	let p = PathBroker::get();
	p.await
}

/// Describes and keys into a pathable segment of FlowFields
pub struct RouteStep {
	sector: SectorID,
	/// Goal [FieldCell] as an index
	goal: usize,
	/// If Some then the actual goal is a [PortalWindow]. If None then the sector is the end goal sector
	portal: Option<PortalWindow>,
}

impl RouteStep {
	/// Init [RouteStep]
	pub fn new(sector: &SectorID, goal: usize, portal: Option<PortalWindow>) -> Self {
		RouteStep {
			sector: *sector,
			goal,
			portal,
		}
	}
	/// Get a reference to the sector
	pub fn get_sector(&self) -> &SectorID {
		&self.sector
	}
	/// Get the goal of the step
	pub fn get_goal(&self) -> usize {
		self.goal
	}
	/// Get the portal of the step
	pub fn portal(&self) -> &Option<PortalWindow> {
		&self.portal
	}
}
