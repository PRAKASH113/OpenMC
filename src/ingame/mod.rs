//! The in-game state: a loaded world.
//!
//! Owns the scene and the pause toggle. The camera that renders it lives in
//! [`crate::camera`], and what a paused game looks like lives in
//! [`crate::paused`] — this module owns the world itself.

mod pause;
mod scene;

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

use crate::app::GameState;
use crate::config::input;

/// Owns what the game shows and runs during play.
pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), scene::spawn)
            .add_systems(OnExit(GameState::InGame), scene::despawn)
            .add_systems(
                Update,
                // Gated on `GameState::InGame`, not `InGameState::Playing`:
                // pausing and resuming are the same keypress, so gating on
                // `Playing` would make the pause one-way. The key check is
                // part of the condition, so the system is skipped outright on
                // every frame the key is not pressed.
                pause::toggle
                    .run_if(in_state(GameState::InGame).and_then(input_just_pressed(input::PAUSE))),
            );
    }
}
