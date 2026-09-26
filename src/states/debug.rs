//! Debug shortcuts for moving between states. Debug builds only.
//!
//! This is tooling rather than part of the state machine: it jumps straight
//! to a state, bypassing every real transition, so each screen can be checked
//! before the states drive themselves. The whole module is behind
//! `#[cfg(debug_assertions)]` where it is declared, so nothing here reaches a
//! release build.
//!
//! Delete this module once real transitions exist.

use bevy::prelude::*;

use super::GameState;
use crate::config::input;

/// Which key jumps to which state.
///
/// [`super::InGameState::Paused`] is deliberately absent — it is only
/// reachable by pausing while in a world, which is the whole point of it
/// being a sub-state.
///
/// Jumping to [`GameState::Loading`] bounces straight back to the menu,
/// because loading currently has nothing to wait for. That is still useful:
/// it exercises the `Loading -> Menu` exit.
const JUMPS: [(KeyCode, GameState); 3] = [
    (input::DEBUG_GOTO_LOADING, GameState::Loading),
    (input::DEBUG_GOTO_MENU, GameState::Menu),
    (input::DEBUG_GOTO_INGAME, GameState::InGame),
];

/// Applies the [`JUMPS`] shortcuts.
pub(super) fn jump_to_state(
    keys: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<GameState>>,
) {
    for (key, state) in JUMPS {
        if keys.just_pressed(key) {
            next.set(state);
            return;
        }
    }
}
