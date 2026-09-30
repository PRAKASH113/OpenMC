//! Testing-only: toggling the player's terrain collision off entirely
//! (noclip), for moving straight through blocks while testing.
//!
//! Compiled out of release entirely, and inert even in a debug build unless
//! [`crate::config::debug::TESTING_TOOLS_ENABLED`] is `true` — see
//! `world::debug` for why both gates exist; this module follows the
//! identical shape, plus the `in_creative` gate every testing hotkey in the
//! game now carries (see `config::debug::TESTING_TOOLS_ENABLED`'s doc
//! comment).

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

use super::in_creative;
use super::physics::CollisionEnabled;
use crate::config::debug as debug_config;
use crate::config::input;

/// Registers the collision-toggle hotkey, if testing tools are enabled.
///
/// A plain `if` around registration, not a run condition on the system: with
/// testing tools off, the toggle system is never added to the schedule at
/// all, instead of being added and skipping its body every frame the key
/// isn't pressed.
pub(super) fn register(app: &mut App) {
    if debug_config::TESTING_TOOLS_ENABLED {
        app.add_systems(
            Update,
            toggle_collision
                .run_if(input_just_pressed(input::TOGGLE_COLLISION).and_then(in_creative)),
        );
    }
}

fn toggle_collision(mut collision: ResMut<CollisionEnabled>) {
    collision.0 = !collision.0;
    info!(
        "player: collision {}",
        if collision.0 { "enabled" } else { "disabled" }
    );
}
