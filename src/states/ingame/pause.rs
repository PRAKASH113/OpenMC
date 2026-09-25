//! Pausing and resuming.
//!
//! Escape toggles between [`InGameState::Playing`] and
//! [`InGameState::Paused`]. This lives in `ingame` rather than `paused`
//! because both directions of the toggle only exist while a world is
//! loaded — `paused` owns what a paused game *shows*, not how it is reached.

use bevy::prelude::*;

use crate::states::InGameState;

/// Flips between playing and paused.
///
/// Only runs on the frame the pause key is pressed while in a world — both
/// conditions are applied where it is registered, in [`super::InGamePlugin`].
/// Reads the current sub-state rather than tracking its own flag, so it
/// cannot disagree with the state machine.
pub fn toggle(current: Res<State<InGameState>>, mut next: ResMut<NextState<InGameState>>) {
    next.set(match current.get() {
        InGameState::Playing => InGameState::Paused,
        InGameState::Paused => InGameState::Playing,
    });
}
