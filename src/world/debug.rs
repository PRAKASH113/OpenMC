//! Testing-only: freezing chunk loading so a fixed set of chunks can be
//! inspected without the world changing under you.
//!
//! Compiled out of release entirely, and inert even in a debug build unless
//! [`crate::config::debug::TESTING_TOOLS_ENABLED`] is `true` — see that
//! constant for why both gates exist.

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

use super::ChunkLock;
use crate::config::debug as debug_config;
use crate::config::input;

/// Registers the chunk-lock hotkey, if testing tools are enabled.
///
/// A plain `if` around registration rather than a run condition on the
/// system: with testing tools off, the toggle system is never added to the
/// schedule at all, instead of being added and skipping its body every
/// frame the key isn't pressed.
pub(super) fn register(app: &mut App) {
    if debug_config::TESTING_TOOLS_ENABLED {
        app.add_systems(
            Update,
            toggle_chunk_lock.run_if(input_just_pressed(input::TOGGLE_CHUNK_LOCK)),
        );
    }
}

/// Flips [`ChunkLock`]. While locked, [`super::load_chunks_around_player`]
/// does not run at all, so whichever chunks happened to be loaded the
/// moment this was switched on stay loaded, however far the player wanders,
/// and nothing new is generated.
fn toggle_chunk_lock(mut lock: ResMut<ChunkLock>) {
    lock.0 = !lock.0;
    info!(
        "world: chunk lock {}",
        if lock.0 {
            "ENGAGED — loading and unloading frozen"
        } else {
            "released"
        }
    );
}
