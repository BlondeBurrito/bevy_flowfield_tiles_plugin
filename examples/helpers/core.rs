//! Main helpers for setting up examples
//!

use bevy::prelude::*;

/// Labels the actor to enable getting its [Transform] easily
#[allow(dead_code)]
#[derive(Component)]
pub struct Actor;

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[allow(dead_code)]
#[derive(Component)]
pub struct FieldCellLabel(pub usize, pub usize);

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[allow(dead_code)]
#[derive(Component)]
pub struct SectorLabel(pub i32, pub i32);

/// Labels the actor to distinguish it from others
#[allow(dead_code)]
#[derive(Component)]
pub struct ActorA;

/// Labels the actor to distinguish it from others
#[allow(dead_code)]
#[derive(Component)]
pub struct ActorB;
