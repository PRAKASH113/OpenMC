//! Player controls: what the player's input does to the player.
//!
//! Each file is one control — [`look`] turns the mouse into where the player
//! looks, [`movement`] turns keys into what the player wants to do (walk,
//! jump, fly), and [`cursor`] captures the mouse so looking works. They all steer the player, which lives in
//! [`crate::player`]; that module owns the player, this one owns the
//! controls. The camera follows the player on its own (`camera::follow`) and
//! is never touched from here.
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
//! makes pausing freeze the player (and so the view) rather than draw an
//! overlay over a world that is still moving.

mod cursor;
mod look;
mod movement;

use bevy::prelude::*;

use crate::player::PlayerPhysics;
use crate::states::InGameState;

/// Registers every player control.
///
/// Named `GameInputPlugin` rather than `InputPlugin` because Bevy already has
/// one of those — the same reason the state machine is `GameStatePlugin`.
pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        // Look before movement, so movement's facing is this frame's; both
        // before the player's physics, so it acts on this frame's keys.
        app.add_systems(
            Update,
            (look::look, movement::read_movement)
                .chain()
                .before(PlayerPhysics)
                .run_if(in_state(InGameState::Playing)),
        )
        .add_systems(OnEnter(InGameState::Playing), cursor::grab)
        .add_systems(OnExit(InGameState::Playing), cursor::release);
    }
}
