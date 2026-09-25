//! Application assembly: startup configuration, the window, logging, the
//! state machine, and the plugin that wires everything together.
//!
//! Gameplay domains live in their own top-level modules and are registered
//! from [`plugin::AppPlugin`]. Nothing gameplay-related belongs here.

// Every submodule is `app`'s own business — they reach each other through
// `super::`. The single thing the rest of the crate needs is `GameState`,
// re-exported below so there is exactly one path to it.
mod config;
mod log;
mod plugin;
mod states;
mod window;

pub(crate) use states::{GameState, InGameState};

use bevy::prelude::*;

/// Builds the app and runs it until the window closes.
pub fn run() -> AppExit {
    App::new().add_plugins(plugin::AppPlugin).run()
}
