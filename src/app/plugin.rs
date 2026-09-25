//! The composition root: every part of the game, registered in one place.

use bevy::prelude::*;

use crate::camera::CameraPlugin;
use crate::ingame::InGamePlugin;
use crate::loading::LoadingPlugin;
use crate::menu::MenuPlugin;
use crate::paused::PausedPlugin;

use crate::utils::log::log_plugin;
use crate::utils::window::{WindowControlPlugin, primary_window_plugin};

use super::states::GameStatePlugin;

/// Assembles the whole game.
///
/// Adding a domain to the game means adding its plugin here and nowhere
/// else.
pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Must come first. It installs `StatesPlugin`, which `GameStatePlugin`
        // needs in place before it calls `init_state` — registering them the
        // other way round is not a compile error, just a runtime warning and
        // a state machine that never transitions.
        app.add_plugins(
            DefaultPlugins
                .set(primary_window_plugin())
                .set(log_plugin()),
        );

        // Infrastructure every state relies on.
        app.add_plugins((GameStatePlugin, CameraPlugin, WindowControlPlugin));

        // One plugin per game state.
        app.add_plugins((LoadingPlugin, MenuPlugin, InGamePlugin, PausedPlugin));
    }
}
