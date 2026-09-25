//! Turns [`super::config`] into Bevy's window settings.
//!
//! This is the only place that knows the engine's window vocabulary, so the
//! config stays a plain list of values.

use bevy::prelude::*;
use bevy::window::WindowResolution;

use super::config;

/// Builds the [`WindowPlugin`] that describes the primary window.
pub fn primary_window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: config::TITLE.to_string(),
            // Physical pixels, not logical ones: on a display with OS
            // scaling the window is exactly this many real pixels, so it
            // appears smaller than a "1280-wide" window would elsewhere.
            resolution: WindowResolution::new(config::WIDTH, config::HEIGHT),
            // Bevy asks whether to *draw* decorations, so this is the
            // inverse of borderless.
            decorations: !config::BORDERLESS,
            // Present mode is a property of the window in Bevy, not of a
            // render plugin, so it is applied here.
            present_mode: config::PRESENT_MODE,
            ..default()
        }),
        ..default()
    }
}
