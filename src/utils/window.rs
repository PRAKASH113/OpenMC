//! Turns [`crate::config::window`] into Bevy's window settings, and handles the
//! runtime window toggles.
//!
//! This is the only place that knows the engine's window vocabulary, so the
//! config stays a plain list of values. The config constants describe how
//! the window *starts*; the systems here change it afterwards by mutating
//! the live `Window` component, which is how Bevy expects window changes to
//! be made.

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode, WindowResolution};

use crate::config::input;
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

/// Handles the runtime window toggles.
pub struct WindowControlPlugin;

impl Plugin for WindowControlPlugin {
    fn build(&self, app: &mut App) {
        // The key check is a run condition rather than an `if` inside the
        // system, so on every frame without that keypress — nearly all of
        // them — the system is skipped entirely: no window query, no body.
        //
        // Not gated on any state: being able to leave fullscreen should not
        // depend on where the player happens to be in the game.
        app.add_systems(
            Update,
            (
                toggle_borderless.run_if(input_just_pressed(input::TOGGLE_BORDERLESS)),
                toggle_fullscreen.run_if(input_just_pressed(input::TOGGLE_FULLSCREEN)),
            ),
        );
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

/// Borderless fullscreen rather than exclusive: it alt-tabs instantly and
/// does not change the display's video mode, which is what players expect
/// from a modern game. Exclusive fullscreen would be
/// `WindowMode::Fullscreen(..)`.
const FULLSCREEN_MODE: WindowMode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);

/// Adds and removes the OS title bar and border.
///
/// Only runs on the frame the toggle key is pressed.
fn toggle_borderless(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = window.single_mut() else {
        return;
    };

    window.decorations = !window.decorations;
}

/// Switches between fullscreen and a window at the configured size.
///
/// Only runs on the frame the toggle key is pressed.
fn toggle_fullscreen(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = window.single_mut() else {
        return;
    };

    window.mode = match window.mode {
        WindowMode::Windowed => FULLSCREEN_MODE,
        // Any fullscreen variant returns to a window, restoring the
        // configured size rather than whatever the display happened to be.
        _ => {
            window.resolution = WindowResolution::new(config::WIDTH, config::HEIGHT);
            WindowMode::Windowed
        }
    };
}
