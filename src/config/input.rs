//! Key bindings and control settings.
//!
//! Every key the game reads is named here and nowhere else, so the whole set
//! is visible at once. The test at the bottom fails if two actions are bound
//! to the same key, so that is caught rather than left to careful reading.
//!
//! This is a settings surface like [`crate::app::config`] — plain values,
//! meant to be read and edited. Values that are *not* player preferences
//! stay with the code that owns them: camera draw order is a rendering
//! detail, and the pitch clamp is a safety limit, not a taste setting.

use bevy::input::keyboard::KeyCode;

// ---------------------------------------------------------------- movement

/// Move the way the camera is facing.
pub const FORWARD: KeyCode = KeyCode::KeyW;

/// Move opposite the camera's facing.
pub const BACKWARD: KeyCode = KeyCode::KeyS;

/// Strafe left.
pub const LEFT: KeyCode = KeyCode::KeyA;

/// Strafe right.
pub const RIGHT: KeyCode = KeyCode::KeyD;

/// Rise, regardless of where the camera points.
pub const UP: KeyCode = KeyCode::Space;

/// Descend, regardless of where the camera points.
pub const DOWN: KeyCode = KeyCode::ControlLeft;

/// Hold to move at [`SPRINT_MULTIPLIER`] times normal speed.
pub const SPRINT: KeyCode = KeyCode::ShiftLeft;

// -------------------------------------------------------------------- game

/// Pause, and resume again from paused.
pub const PAUSE: KeyCode = KeyCode::Escape;

// ------------------------------------------------------------------ window

/// Toggle the OS title bar and border on and off.
pub const TOGGLE_BORDERLESS: KeyCode = KeyCode::F10;

/// Toggle fullscreen. Leaving it restores the configured window size.
pub const TOGGLE_FULLSCREEN: KeyCode = KeyCode::F11;

// -------------------------------------------------------- control settings

/// Radians of camera rotation per pixel of mouse movement.
///
/// Raise for a twitchier feel. This is the setting most worth tuning
/// against how it actually plays rather than reasoning about.
pub const LOOK_SENSITIVITY: f32 = 0.002;

/// Movement speed in world units per second.
pub const MOVE_SPEED: f32 = 12.0;

/// Speed multiplier applied while [`SPRINT`] is held.
pub const SPRINT_MULTIPLIER: f32 = 3.0;

// ------------------------------------------------------------------- debug

/// Jump straight to the loading state. Debug builds only.
#[cfg(debug_assertions)]
pub const DEBUG_GOTO_LOADING: KeyCode = KeyCode::Digit1;

/// Jump straight to the menu. Debug builds only.
#[cfg(debug_assertions)]
pub const DEBUG_GOTO_MENU: KeyCode = KeyCode::Digit2;

/// Jump straight into a world. Debug builds only.
#[cfg(debug_assertions)]
pub const DEBUG_GOTO_INGAME: KeyCode = KeyCode::Digit3;

#[cfg(test)]
mod tests {
    use super::*;

    /// Every binding above, paired with its name for the failure message.
    ///
    /// This list is maintained by hand, so it is only as complete as
    /// whoever last added a binding — add new keys here as well as above.
    /// Keeping the constants plainly readable was judged worth that cost
    /// over a macro that generates both.
    const ALL: &[(&str, KeyCode)] = &[
        ("FORWARD", FORWARD),
        ("BACKWARD", BACKWARD),
        ("LEFT", LEFT),
        ("RIGHT", RIGHT),
        ("UP", UP),
        ("DOWN", DOWN),
        ("SPRINT", SPRINT),
        ("PAUSE", PAUSE),
        ("TOGGLE_BORDERLESS", TOGGLE_BORDERLESS),
        ("TOGGLE_FULLSCREEN", TOGGLE_FULLSCREEN),
        ("DEBUG_GOTO_LOADING", DEBUG_GOTO_LOADING),
        ("DEBUG_GOTO_MENU", DEBUG_GOTO_MENU),
        ("DEBUG_GOTO_INGAME", DEBUG_GOTO_INGAME),
    ];

    #[test]
    fn no_key_is_bound_to_two_actions() {
        for (index, (name, key)) in ALL.iter().enumerate() {
            for (other_name, other_key) in &ALL[index + 1..] {
                assert_ne!(
                    key, other_key,
                    "`{name}` and `{other_name}` are both bound to {key:?}"
                );
            }
        }
    }
}
