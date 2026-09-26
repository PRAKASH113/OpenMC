//! Mouse look: turning mouse movement into view rotation.

use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::camera::{LookAngles, WorldCamera};
// Aliased: inside `crate::input`, a bare `input::` would read as this module.
use crate::config::input as controls;

/// How close to straight up/down the view may pitch.
///
/// A safety limit rather than a preference — past vertical the view flips
/// over — so it lives with the control that enforces it, not in config.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

/// Turns mouse movement into camera rotation.
pub(super) fn look(
    mut motion: MessageReader<MouseMotion>,
    mut camera: Query<(&mut Transform, &mut LookAngles), With<WorldCamera>>,
) {
    // Several motion events can arrive in one frame; only their sum matters.
    // Summing first means one angle update and one quaternion rebuild per
    // frame, however many events there were.
    let delta: Vec2 = motion.read().map(|event| event.delta).sum();

    // No mouse movement is the common case. Return before touching the
    // query, so a still mouse costs nothing beyond draining an empty queue.
    if delta == Vec2::ZERO {
        return;
    }

    let Ok((mut transform, mut angles)) = camera.single_mut() else {
        return;
    };

    angles.yaw -= delta.x * controls::LOOK_SENSITIVITY;
    angles.pitch =
        (angles.pitch - delta.y * controls::LOOK_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, angles.yaw, angles.pitch, 0.0);
}
