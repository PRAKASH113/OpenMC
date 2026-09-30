//! Mouse look: turning mouse movement into where the player looks.

use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

// Aliased: inside `crate::input`, a bare `input::` would read as this module.
use crate::config::input as controls;
use crate::player::{LookAngles, Player};

/// How close to straight up/down the view may pitch.
///
/// A safety limit rather than a preference — past vertical the view flips
/// over — so it lives with the control that enforces it, not in config.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

/// Turns mouse movement into the player's look angles, and turns the player's
/// body to match.
///
/// Only yaw reaches the body: the model turns to face where the player looks
/// but never tilts forward or back. Pitch only moves the camera, which
/// `camera::follow` places from these same angles.
pub(super) fn look(
    mut motion: MessageReader<MouseMotion>,
    mut player: Query<(&mut Transform, &mut LookAngles), With<Player>>,
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

    let Ok((mut transform, mut angles)) = player.single_mut() else {
        return;
    };

    angles.yaw -= delta.x * controls::LOOK_SENSITIVITY;
    angles.pitch =
        (angles.pitch - delta.y * controls::LOOK_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    transform.rotation = Quat::from_rotation_y(angles.yaw);
}
