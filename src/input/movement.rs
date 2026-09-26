//! Movement: turning held keys into flight.
//!
//! Free flight for now — movement follows where the camera is pointing, with
//! no gravity or collision. When a player body with physics arrives, this
//! stays the place that reads *intent* from the keys, and the physics that
//! acts on it belongs to the player, not here.

use bevy::prelude::*;

use crate::camera::WorldCamera;
// Aliased: inside `crate::input`, a bare `input::` would read as this module.
use crate::config::input as controls;

/// The keys a double-tap of any one of can engage sprint.
///
/// Only consulted when [`controls::SPRINT_MODE`] is
/// [`controls::SprintMode::DoubleTap`].
const MOVEMENT_KEYS: [KeyCode; 4] = [
    controls::FORWARD,
    controls::BACKWARD,
    controls::LEFT,
    controls::RIGHT,
];

/// Moves the camera from keyboard input.
///
/// Sprint is one of two things depending on [`controls::SPRINT_MODE`]: a
/// double-tap of a movement key that stays engaged until every movement key
/// is released, or a plain held key. `last_tap` only means anything in the
/// double-tap case, but it is declared unconditionally — the mode is a
/// config constant, not something that changes at runtime, so there is
/// nothing to gain from wiring the system's parameters to it.
pub(super) fn fly(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera: Query<&mut Transform, With<WorldCamera>>,
    mut last_tap: Local<Option<(KeyCode, f32)>>,
    mut sprinting: Local<bool>,
) {
    // Read the keys before anything else. Standing still is the common case,
    // and it should cost six key lookups and nothing more: no query, and none
    // of the quaternion maths below. That maths matters at opt-level 0 in
    // particular — glam's functions are `#[inline]`, so they are compiled
    // into this crate unoptimised rather than taking Bevy's opt-level 3.
    let intent = Vec3::new(
        axis(&keys, controls::RIGHT, controls::LEFT),
        axis(&keys, controls::UP, controls::DOWN),
        axis(&keys, controls::FORWARD, controls::BACKWARD),
    );
    if intent == Vec3::ZERO {
        // No movement key is held, so sprint has nothing to stay engaged
        // for — the next tap starts a fresh double-tap window rather than
        // chaining onto whatever was held before.
        *sprinting = false;
        return;
    }

    match controls::SPRINT_MODE {
        controls::SprintMode::Hold => *sprinting = keys.pressed(controls::SPRINT_HOLD_KEY),
        // A fresh double-tap of the same movement key engages sprint. It
        // stays engaged until every movement key is released (the check
        // above), the same way Minecraft's sprint toggle works, rather than
        // needing to be re-triggered every frame.
        controls::SprintMode::DoubleTap => {
            let now = time.elapsed_secs();
            for key in MOVEMENT_KEYS {
                if !keys.just_pressed(key) {
                    continue;
                }
                if is_double_tap(key, now, *last_tap) {
                    *sprinting = true;
                }
                *last_tap = Some((key, now));
            }
        }
    }

    let Ok(mut transform) = camera.single_mut() else {
        return;
    };

    let direction =
        *transform.right() * intent.x + Vec3::Y * intent.y + *transform.forward() * intent.z;

    // Normalising a zero vector yields NaN, which would poison the transform
    // permanently. `intent` is non-zero here and pitch is clamped short of
    // vertical, so an exactly-zero direction should not occur — but the check
    // costs nothing and a NaN camera is unrecoverable, so it stays.
    let Some(direction) = direction.try_normalize() else {
        return;
    };

    let speed = if *sprinting {
        controls::MOVE_SPEED * controls::SPRINT_MULTIPLIER
    } else {
        controls::MOVE_SPEED
    };

    transform.translation += direction * speed * time.delta_secs();
}

/// Whether pressing `key` right now counts as a double-tap of the same key,
/// given when it (or another key) was last tapped.
fn is_double_tap(key: KeyCode, now: f32, last_tap: Option<(KeyCode, f32)>) -> bool {
    matches!(
        last_tap,
        Some((last_key, last_time))
            if last_key == key && now - last_time < controls::DOUBLE_TAP_WINDOW
    )
}

/// One movement axis from a pair of opposing keys.
///
/// `1.0` when only `positive` is held, `-1.0` when only `negative` is, and
/// `0.0` when neither or both are — holding both cancels rather than letting
/// one silently win.
fn axis(keys: &ButtonInput<KeyCode>, positive: KeyCode, negative: KeyCode) -> f32 {
    match (keys.pressed(positive), keys.pressed(negative)) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(keys: &[KeyCode]) -> ButtonInput<KeyCode> {
        let mut input = ButtonInput::default();
        for &key in keys {
            input.press(key);
        }
        input
    }

    #[test]
    fn axis_is_zero_with_no_keys() {
        assert_eq!(axis(&held(&[]), KeyCode::KeyW, KeyCode::KeyS), 0.0);
    }

    #[test]
    fn axis_follows_a_single_key() {
        assert_eq!(
            axis(&held(&[KeyCode::KeyW]), KeyCode::KeyW, KeyCode::KeyS),
            1.0
        );
        assert_eq!(
            axis(&held(&[KeyCode::KeyS]), KeyCode::KeyW, KeyCode::KeyS),
            -1.0
        );
    }

    #[test]
    fn opposing_keys_cancel() {
        let both = held(&[KeyCode::KeyW, KeyCode::KeyS]);
        assert_eq!(axis(&both, KeyCode::KeyW, KeyCode::KeyS), 0.0);
    }

    #[test]
    fn no_previous_tap_is_not_a_double_tap() {
        assert!(!is_double_tap(KeyCode::KeyW, 1.0, None));
    }

    #[test]
    fn same_key_within_the_window_is_a_double_tap() {
        let last = Some((KeyCode::KeyW, 1.0));
        assert!(is_double_tap(
            KeyCode::KeyW,
            1.0 + controls::DOUBLE_TAP_WINDOW - 0.01,
            last
        ));
    }

    #[test]
    fn same_key_outside_the_window_is_not_a_double_tap() {
        let last = Some((KeyCode::KeyW, 1.0));
        assert!(!is_double_tap(
            KeyCode::KeyW,
            1.0 + controls::DOUBLE_TAP_WINDOW + 0.01,
            last
        ));
    }

    #[test]
    fn a_different_key_is_not_a_double_tap() {
        let last = Some((KeyCode::KeyW, 1.0));
        assert!(!is_double_tap(KeyCode::KeyA, 1.05, last));
    }
}
