//! The paused state: gameplay suspended with the world still loaded.
//!
//! Keyed off [`InGameState::Paused`], so this can only ever appear over a
//! loaded world. Escape is handled in [`crate::ingame`], which owns the
//! toggle in both directions.

pub mod screen;

use bevy::prelude::*;

use crate::app::InGameState;

/// Owns what the game shows while paused.
pub struct PausedPlugin;

impl Plugin for PausedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(InGameState::Paused), screen::spawn)
            .add_systems(OnExit(InGameState::Paused), screen::despawn);
    }
}
