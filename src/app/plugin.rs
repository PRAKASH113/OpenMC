//! The composition root: every part of the game, registered in one place.

use bevy::prelude::*;

use crate::camera::CameraPlugin;
use crate::states::GameStatePlugin;
use crate::utils::log::log_plugin;
use crate::utils::window::{WindowControlPlugin, primary_window_plugin};

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
                .set(log_plugin())
                // Engine plugins this game has no use for yet. Disabling at
                // runtime costs no Bevy recompile, unlike dropping the Cargo
                // feature, and both are leaf plugins — nothing else in Bevy
                // depends on them, so removing them cannot break startup.
                //
                // Audio: there is no sound. Left enabled it opens an output
                // device and keeps an audio thread running all session.
                //
                // Gilrs: the gamepad backend. There is no controller support,
                // and left enabled it polls the OS for gamepad events every
                // frame. Re-enable it when controller support is built.
                //
                // Coupling: when the `audio` feature is later dropped in
                // Cargo.toml, `AudioPlugin` stops existing and the line below
                // stops compiling. Delete it at the same time.
                .disable::<bevy::audio::AudioPlugin>()
                .disable::<bevy::gilrs::GilrsPlugin>(),
        );

        // Our domains. `GameStatePlugin` brings in every state itself, so
        // adding a state never touches this file — only `states/mod.rs`.
        app.add_plugins((GameStatePlugin, CameraPlugin, WindowControlPlugin));
    }
}
