//! Changing the window at runtime: the borderless and fullscreen keys.
//!
//! Runs for the life of the game, but each system only executes on the frame
//! its key is pressed — the key check is a run condition, not an `if` inside
//! the system.

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};

use super::FULLSCREEN_MODE;
use crate::config::input;
use crate::config::window as config;

/// Handles the runtime window toggles.
pub struct WindowControlPlugin;

impl Plugin for WindowControlPlugin {
    fn build(&self, app: &mut App) {
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

/// Adds and removes the OS title bar and border.
fn toggle_borderless(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = window.single_mut() else {
        return;
    };

    window.decorations = !window.decorations;
}

/// Switches between fullscreen and a window at the configured size.
fn toggle_fullscreen(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = window.single_mut() else {
        return;
    };

    window.mode = match window.mode {
        WindowMode::Windowed => FULLSCREEN_MODE,
        // Any fullscreen variant returns to a window, restoring the
        // configured size rather than whatever the display happened to be.
        //
        // `.set()`, not `resolution = WindowResolution::new(..)`. `set()`
        // treats its arguments as *logical* pixels and multiplies by the
        // window's current (OS-reported) scale factor to get the physical
        // size — the same conversion winit does when it first creates the
        // window at this logical size. `WindowResolution::new(..)` builds a
        // fresh `WindowResolution` with `scale_factor` reset to its default
        // of 1.0, and a live resolution change is applied as an exact
        // physical pixel request — so on any monitor with OS scaling above
        // 100%, replacing the resolution wholesale silently produces a
        // smaller window than the one the player started with.
        _ => {
            window
                .resolution
                .set(config::WIDTH as f32, config::HEIGHT as f32);
            WindowMode::Windowed
        }
    };
}
