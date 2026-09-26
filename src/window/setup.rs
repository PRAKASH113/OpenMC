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
            // Physical pixels, not logical ones: on a display with OS
            // scaling the window is exactly this many real pixels, so it
            // appears smaller than a "1280-wide" window would elsewhere.
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
