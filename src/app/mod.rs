//! Application assembly: the plugin that wires every part of the game
//! together, and the function that runs it.
//!
//! Only the shape of the app lives here. The states and their machine are in
//! [`crate::states`], configurable values in [`crate::config`], and the
//! finished adapters that build the window and configure logging in
//! [`crate::utils`]. Every domain is registered from [`plugin::AppPlugin`].

mod plugin;

use bevy::prelude::*;

/// Builds the app and runs it until the window closes.
pub fn run() -> AppExit {
    App::new().add_plugins(plugin::AppPlugin).run()
}
