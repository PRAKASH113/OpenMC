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

/// Move the way the player is facing, level with the ground.
pub const FORWARD: KeyCode = KeyCode::KeyW;

/// Move opposite the player's facing, level with the ground.
pub const BACKWARD: KeyCode = KeyCode::KeyS;

/// Strafe left.
pub const LEFT: KeyCode = KeyCode::KeyA;

/// Strafe right.
pub const RIGHT: KeyCode = KeyCode::KeyD;

/// On foot: jump. In Creative, double-tap to take off. While flying: rise,
/// and double-tap then hold to rise at double speed.
pub const UP: KeyCode = KeyCode::Space;

/// While flying: descend. In Creative, double-tap to stop flying and fall
/// back under gravity. Does nothing on foot yet.
pub const DOWN: KeyCode = KeyCode::ShiftLeft;

// -------------------------------------------------------------------- game

/// Pause, and resume again from paused.
pub const PAUSE: KeyCode = KeyCode::Escape;

/// Switch between the game modes (`config::player::GameMode`). F4, beside
/// where Minecraft keeps its own game-mode switcher (F3+F4). Works in every
/// mode — it's how you get back out of Survival.
pub const TOGGLE_GAME_MODE: KeyCode = KeyCode::F4;

// ------------------------------------------------------------------ window

/// Toggle the OS title bar and border on and off.
pub const TOGGLE_BORDERLESS: KeyCode = KeyCode::F10;

/// Toggle fullscreen. Leaving it restores the configured window size.
pub const TOGGLE_FULLSCREEN: KeyCode = KeyCode::F11;

// -------------------------------------------------------- control settings

/// Radians of view rotation per pixel of mouse movement.
///
/// Raise for a twitchier feel. This is the setting most worth tuning
/// against how it actually plays rather than reasoning about.
pub const LOOK_SENSITIVITY: f32 = 0.002;

/// Movement speed on foot, in blocks per second. Minecraft walks at about 4.3.
pub const WALK_SPEED: f32 = 4.3;

/// Movement speed while flying, in blocks per second, on every axis.
pub const FLY_SPEED: f32 = 12.0;

/// How much faster the player rises after a double-tap of [`UP`] while
/// flying, for as long as it stays held.
pub const FAST_ASCENT_MULTIPLIER: f32 = 2.0;

/// Speed multiplier applied while sprinting, on foot or flying.
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

/// How quickly two presses of the same key must follow each other to count
/// as a double-tap — for sprint (when [`SPRINT_MODE`] is
/// [`SprintMode::DoubleTap`]) and for taking off, landing, and fast ascent
/// with [`UP`] and [`DOWN`].
pub const DOUBLE_TAP_WINDOW: f32 = 0.3;

/// Held to sprint when [`SPRINT_MODE`] is [`SprintMode::Hold`].
///
/// Left Control rather than Left Shift: [`DOWN`] already holds Left Shift,
/// and Left Control is the traditional "sprint" modifier this is replacing.
pub const SPRINT_HOLD_KEY: KeyCode = KeyCode::ControlLeft;

// ------------------------------------------------------------------- debug
//
// Two kinds of key here. The testing tools (F5–F9, and the modifier) only
// work in Creative — see `config::player::GameMode`. The state jumps (1–3)
// work in every mode: they stand in for a menu that doesn't exist yet, and
// `3` is the only way into a world at all, so gating them on a mode that
// only exists in-game would lock the player out.

/// Jump straight to the loading state. Debug builds only.
#[cfg(debug_assertions)]
pub const DEBUG_GOTO_LOADING: KeyCode = KeyCode::Digit1;

/// Jump straight to the menu. Debug builds only.
#[cfg(debug_assertions)]
pub const DEBUG_GOTO_MENU: KeyCode = KeyCode::Digit2;

/// Jump straight into a world. Debug builds only.
#[cfg(debug_assertions)]
pub const DEBUG_GOTO_INGAME: KeyCode = KeyCode::Digit3;

/// Toggle chunk locking: freezes whichever chunks are currently loaded, so
/// none unload and none newly load however far the player wanders. Testing
/// only — inert unless [`crate::config::debug::TESTING_TOOLS_ENABLED`] is
/// `true`. See `world::debug`.
#[cfg(debug_assertions)]
pub const TOGGLE_CHUNK_LOCK: KeyCode = KeyCode::F9;

/// Toggle wireframe rendering on every chunk mesh, to check the greedy
/// mesher is actually producing the quads it should. Testing only — inert
/// unless [`crate::config::debug::TESTING_TOOLS_ENABLED`] is `true`. See
/// `render::debug`.
#[cfg(debug_assertions)]
pub const TOGGLE_WIREFRAME: KeyCode = KeyCode::F8;

/// Cycle the chunk-bounds grid drawn around whichever chunk the player is
/// currently in: none, an outline, or an outline with axis lines through
/// its centre and across each face. Testing only — inert unless
/// [`crate::config::debug::TESTING_TOOLS_ENABLED`] is `true`. See
/// `render::debug`.
#[cfg(debug_assertions)]
pub const TOGGLE_CHUNK_GRID: KeyCode = KeyCode::F7;

/// Held alongside [`TOGGLE_CHUNK_GRID`] to lock the grid to its current
/// chunk instead of cycling its display mode — it then stops following the
/// player, the same way [`TOGGLE_CHUNK_LOCK`] freezes chunk loading. A
/// dedicated modifier rather than reusing [`SPRINT_HOLD_KEY`] (Left
/// Control): this is an unrelated debug gesture, not a second meaning for a
/// gameplay key. Testing only, same as the key it modifies.
#[cfg(debug_assertions)]
pub const DEBUG_MODIFIER: KeyCode = KeyCode::AltLeft;

/// Toggle the sea-level marker: a grid at absolute world height `0` across
/// every loaded chunk column, showing where `CHUNKS_BELOW_SEA_LEVEL` starts.
/// Testing only — inert unless
/// [`crate::config::debug::TESTING_TOOLS_ENABLED`] is `true`. See
/// `render::debug`.
#[cfg(debug_assertions)]
pub const TOGGLE_SEA_LEVEL_LINE: KeyCode = KeyCode::F6;

/// Toggle the player's terrain collision off (pass straight through blocks)
/// or back on. Testing only — inert unless
/// [`crate::config::debug::TESTING_TOOLS_ENABLED`] is `true`, and only takes
/// effect in [`crate::config::player::GameMode::Creative`], the same as
/// every other key here. See `player::debug`.
#[cfg(debug_assertions)]
pub const TOGGLE_COLLISION: KeyCode = KeyCode::F5;

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
            ("TOGGLE_GAME_MODE", TOGGLE_GAME_MODE),
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
            ("TOGGLE_CHUNK_LOCK", TOGGLE_CHUNK_LOCK),
            ("TOGGLE_WIREFRAME", TOGGLE_WIREFRAME),
            ("TOGGLE_CHUNK_GRID", TOGGLE_CHUNK_GRID),
            ("DEBUG_MODIFIER", DEBUG_MODIFIER),
            ("TOGGLE_SEA_LEVEL_LINE", TOGGLE_SEA_LEVEL_LINE),
            ("TOGGLE_COLLISION", TOGGLE_COLLISION),
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
