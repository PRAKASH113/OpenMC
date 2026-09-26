//! Building the window at startup from [`crate::config::window`].
//!
//! Runs once, before the app starts. Nothing here executes per frame.

use bevy::prelude::*;
use bevy::window::{WindowMode, WindowResolution};

use super::FULLSCREEN_MODE;
use crate::config::window as config;

/// Builds the [`WindowPlugin`] that describes the primary window.
pub fn primary_window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: config::TITLE.to_string(),
            // `WindowResolution::new`'s parameters are named `physical_*`,
            // but at window *creation* — this code path only — Bevy hands
            // them to winit as a logical size when no scale factor override
            // is set (which we never set). Winit then converts using the
            // real monitor DPI, so the window matches config::WIDTH/HEIGHT
            // in points, the same visual size on any display. See
            // `config::window` and `window::toggles` for the runtime half of
            // this, where the same numbers must be handled differently.
            resolution: WindowResolution::new(config::WIDTH, config::HEIGHT),
            // Bevy asks whether to *draw* decorations, so this is the
            // inverse of borderless.
            decorations: !config::BORDERLESS,
            mode: starting_mode(),
            // Present mode is a property of the window in Bevy, not of a
            // render plugin, so it is applied here.
            present_mode: config::PRESENT_MODE,
            ..default()
        }),
        ..default()
    }
}

/// The window mode [`config::FULLSCREEN`] asks for.
fn starting_mode() -> WindowMode {
    if config::FULLSCREEN {
        FULLSCREEN_MODE
    } else {
        WindowMode::Windowed
    }
}
