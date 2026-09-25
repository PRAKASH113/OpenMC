//! Pausing and resuming.
//!
//! Escape toggles between [`InGameState::Playing`] and
//! [`InGameState::Paused`]. This lives in `ingame` rather than `paused`
//! because both directions of the toggle only exist while a world is
//! loaded — `paused` owns what a paused game *shows*, not how it is reached.

use bevy::prelude::*;

use crate::app::InGameState;
use crate::config::input;

/// Flips between playing and paused on Escape.
///
/// Reads the current sub-state rather than tracking its own flag, so it
/// cannot disagree with the state machine.
pub fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    current: Res<State<InGameState>>,
    mut next: ResMut<NextState<InGameState>>,
) {
    if !keys.just_pressed(input::PAUSE) {
        return;
    }

    next.set(match current.get() {
        InGameState::Playing => InGameState::Paused,
        InGameState::Paused => InGameState::Playing,
    });
}
