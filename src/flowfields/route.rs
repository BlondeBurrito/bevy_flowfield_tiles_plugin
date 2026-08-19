//! When an actor requests a path and the floe fields are net yet ready they
//! are given temporary simpler path based on portal-to-portal pathing
//!
//! The [RouteStep] describes the portal location within a particular sector
//!

use crate::flowfields::{portal::PortalWindow, sectors::SectorID};

/// Describes and keys into a pathable segment of FlowFields
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteStep {
	/// The sector the step refers to
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
