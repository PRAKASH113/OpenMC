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

/// Moves the camera from keyboard input.
pub(super) fn fly(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera: Query<&mut Transform, With<WorldCamera>>,
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
        return;
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

    let speed = if keys.pressed(controls::SPRINT) {
        controls::MOVE_SPEED * controls::SPRINT_MULTIPLIER
    } else {
        controls::MOVE_SPEED
    };

    transform.translation += direction * speed * time.delta_secs();
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
}
