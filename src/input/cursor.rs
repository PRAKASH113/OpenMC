//! Mouse capture: locking the cursor to the window while playing.
//!
//! Exists to serve mouse look — without it, moving the mouse would carry the
//! cursor off the window instead of turning the view.

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

/// Locks the cursor to the window and hides it, so mouse movement turns the
/// view instead of wandering onto another monitor.
pub(super) fn grab(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };

    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

/// Gives the cursor back, so a paused player can use their desktop.
pub(super) fn release(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };

    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}
