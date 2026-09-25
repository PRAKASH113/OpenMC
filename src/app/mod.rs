//! Application assembly: the state machine, and the plugin that wires every
//! part of the game together.
//!
//! Only the shape of the app lives here. Configurable values are in
//! [`crate::config`], and the finished adapters that build the window and
//! configure logging are in [`crate::utils`]. Gameplay domains live in their
//! own top-level modules and are registered from [`plugin::AppPlugin`].

// Both submodules are `app`'s own business — they reach each other through
// `super::`. The only things the rest of the crate needs are the two state
// types, re-exported below so there is exactly one path to them.
mod plugin;
mod states;

pub(crate) use states::{GameState, InGameState};

use bevy::prelude::*;

/// Builds the app and runs it until the window closes.
pub fn run() -> AppExit {
    App::new().add_plugins(plugin::AppPlugin).run()
}
