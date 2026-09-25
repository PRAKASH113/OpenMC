//! The in-game state: a loaded world.
//!
//! Owns the scene and the pause toggle. The camera that renders it lives in
//! [`crate::camera`], and what a paused game looks like lives in
//! [`crate::paused`] — this module owns the world itself.

mod pause;
mod scene;

use bevy::prelude::*;

use crate::app::GameState;

/// Owns what the game shows and runs during play.
pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), scene::spawn)
            .add_systems(OnExit(GameState::InGame), scene::despawn)
            // Runs in both sub-states: pausing and resuming are the same
            // keypress, so gating this on `Playing` would make the pause
            // one-way.
            .add_systems(Update, pause::toggle.run_if(in_state(GameState::InGame)));
    }
}
