//! The in-game state: a loaded world.
//!
//! Owns the scene, the pause toggle, and its own sub-states. `paused/` lives
//! inside this folder because `Paused` is a sub-state of `InGame` — the
//! world stays loaded underneath it. The camera that renders the world is in
//! [`crate::camera`].

mod pause;
mod paused;
mod scene;

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

use crate::config::input;
use crate::states::GameState;

use paused::PausedPlugin;

/// Owns what the game shows and runs during play, including its sub-states.
pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PausedPlugin);

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
