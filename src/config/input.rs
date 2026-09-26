//! Key bindings and control settings.
//!
//! Every key the game reads is named here and nowhere else, so the whole set
//! is visible at once. The test at the bottom fails if two actions are bound
//! to the same key, so that is caught rather than left to careful reading.
//!
//! This is a settings surface like [`crate::config::window`] — plain values,
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
pub const DOWN: KeyCode = KeyCode::ShiftLeft;

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

/// Speed multiplier applied while sprinting.
pub const SPRINT_MULTIPLIER: f32 = 3.0;

/// The two ways sprint can be triggered.
///
/// Which one is live is picked below by [`SPRINT_MODE`], a `const` — so
/// only one variant is ever actually constructed, and the compiler flags
/// the other as dead code. That's the point: switching mode means editing
/// this source and rebuilding, not a runtime choice, so the "dead" variant
/// really is just the one nothing currently selects.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SprintMode {
    /// Double-tap any of [`FORWARD`], [`BACKWARD`], [`LEFT`], or [`RIGHT`]
    /// within [`DOUBLE_TAP_WINDOW`]. Sprint then stays engaged, the way
    /// Minecraft's toggle does, until every movement key is released.
    DoubleTap,
    /// Hold [`SPRINT_HOLD_KEY`].
    Hold,
}

/// Which of [`SprintMode`]'s two triggers is active.
///
/// A settings-surface constant like the ones around it, not a runtime
/// option — there is no settings menu yet for a player to flip this
/// themselves. Change it here and rebuild to switch styles.
pub const SPRINT_MODE: SprintMode = SprintMode::DoubleTap;

/// How quickly two presses of the same movement key must follow each other
/// to count as a double-tap. Only read when [`SPRINT_MODE`] is
/// [`SprintMode::DoubleTap`].
pub const DOUBLE_TAP_WINDOW: f32 = 0.3;

/// Held to sprint when [`SPRINT_MODE`] is [`SprintMode::Hold`].
///
/// Left Control rather than Left Shift: [`DOWN`] already holds Left Shift,
/// and Left Control is the traditional "sprint" modifier this is replacing.
pub const SPRINT_HOLD_KEY: KeyCode = KeyCode::ControlLeft;

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
    fn all_bindings() -> Vec<(&'static str, KeyCode)> {
        let mut all = vec![
            ("FORWARD", FORWARD),
            ("BACKWARD", BACKWARD),
            ("LEFT", LEFT),
            ("RIGHT", RIGHT),
            ("UP", UP),
            ("DOWN", DOWN),
            ("SPRINT_HOLD_KEY", SPRINT_HOLD_KEY),
            ("PAUSE", PAUSE),
            ("TOGGLE_BORDERLESS", TOGGLE_BORDERLESS),
            ("TOGGLE_FULLSCREEN", TOGGLE_FULLSCREEN),
        ];

        // The debug keys only exist in debug builds, so they can only clash
        // with anything there — and naming them unconditionally is what
        // broke `cargo test --release`.
        #[cfg(debug_assertions)]
        all.extend([
            ("DEBUG_GOTO_LOADING", DEBUG_GOTO_LOADING),
            ("DEBUG_GOTO_MENU", DEBUG_GOTO_MENU),
            ("DEBUG_GOTO_INGAME", DEBUG_GOTO_INGAME),
        ]);

        all
    }

    #[test]
    fn no_key_is_bound_to_two_actions() {
        let all = all_bindings();
        for (index, (name, key)) in all.iter().enumerate() {
            for (other_name, other_key) in &all[index + 1..] {
                assert_ne!(
                    key, other_key,
                    "`{name}` and `{other_name}` are both bound to {key:?}"
                );
            }
        }
    }
}
