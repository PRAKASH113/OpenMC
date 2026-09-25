//! The paused sub-state: gameplay suspended with the world still loaded.
//!
//! Keyed off [`InGameState::Paused`], so this can only ever appear over a
//! loaded world — and it lives inside `ingame/` for the same reason. The
//! Escape toggle is in the parent module, which owns both directions of it.

mod screen;

use bevy::prelude::*;

use crate::states::InGameState;

/// Owns what the game shows while paused.
pub struct PausedPlugin;

impl Plugin for PausedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(InGameState::Paused), screen::spawn)
            .add_systems(OnExit(InGameState::Paused), screen::despawn);
    }
}
