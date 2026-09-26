//! Player controls: what the player's input does to the world view.
//!
//! Each file is one control — [`look`] turns the mouse into view rotation,
//! [`movement`] turns keys into flight, and [`cursor`] captures the mouse so
//! looking works. They all steer the world camera, which lives in
//! [`crate::camera`]; that module owns the camera, this one owns the controls.
//!
//! Two related things deliberately live elsewhere:
//!
//! - **Which key does what** is a setting, so the bindings are in
//!   [`crate::config::input`]. This module reads them.
//! - **One-shot key actions** belong to whatever they change, with the key as
//!   a run condition: Escape (pause) is in `states::ingame::pause`, and the
//!   F10/F11 window toggles are in `window::toggles`. This module is for
//!   controls that *interpret* input continuously, every frame.
//!
//! Every control here runs only while [`InGameState::Playing`], which is what
//! makes pausing freeze the view rather than draw an overlay over a camera
//! that is still moving.

mod cursor;
mod look;
mod movement;

use bevy::prelude::*;

use crate::states::InGameState;

/// Registers every player control.
///
/// Named `GameInputPlugin` rather than `InputPlugin` because Bevy already has
/// one of those — the same reason the state machine is `GameStatePlugin`.
pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (look::look, movement::fly).run_if(in_state(InGameState::Playing)),
        )
        .add_systems(OnEnter(InGameState::Playing), cursor::grab)
        .add_systems(OnExit(InGameState::Playing), cursor::release);
    }
}
