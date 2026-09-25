//! The 3D camera that renders the world, and the controls that fly it.
//!
//! Unlike the UI camera this one is tied to a loaded world: it is spawned on
//! entering [`GameState::InGame`] and despawned on leaving, so no 3D view is
//! rendered while the player is in a menu.
//!
//! Its controls run only during [`InGameState::Playing`], which is what
//! makes pausing actually freeze the view rather than merely drawing an
//! overlay on top of a still-moving camera.

use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::app::{GameState, InGameState};
use crate::config::input;

/// Marks the camera that renders the world.
#[derive(Component)]
pub struct WorldCamera;

/// Accumulated look angles, in radians.
///
/// Stored rather than read back from the transform: recovering Euler angles
/// from a quaternion every frame is lossy and drifts, and it makes clamping
/// pitch awkward.
#[derive(Component, Default)]
pub struct LookAngles {
    /// Rotation around the vertical axis. Unbounded — it wraps naturally.
    yaw: f32,
    /// Rotation around the horizontal axis, clamped to just under straight
    /// up and down so the view can never flip over.
    pitch: f32,
}

/// Draw order for the world camera.
///
/// Lower than the UI camera's, so the interface renders on top of the world.
const ORDER: isize = 0;

/// Where the camera starts, looking back at the origin.
const START_POSITION: Vec3 = Vec3::new(8.0, 6.0, 16.0);

/// How close to straight up/down the pitch may get.
///
/// A safety limit rather than a preference — past vertical the view flips
/// over — so it stays here instead of in [`crate::config::input`].
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

/// Spawns the world camera and flies it.
pub struct Camera3dPlugin;

impl Plugin for Camera3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_world_camera)
            .add_systems(OnExit(GameState::InGame), despawn_world_camera)
            // Only while playing: paused means the camera holds still.
            .add_systems(Update, (look, fly).run_if(in_state(InGameState::Playing)))
            .add_systems(OnEnter(InGameState::Playing), grab_cursor)
            .add_systems(OnExit(InGameState::Playing), release_cursor);
    }
}

fn spawn_world_camera(mut commands: Commands) {
    let transform = Transform::from_translation(START_POSITION).looking_at(Vec3::ZERO, Vec3::Y);

    // Recover the starting angles from the initial look-at so the first
    // mouse movement continues from where the camera is pointing instead of
    // snapping.
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: ORDER,
            ..default()
        },
        // Geometry edges genuinely alias here, unlike on the UI camera.
        Msaa::Sample4,
        transform,
        LookAngles { yaw, pitch },
        WorldCamera,
    ));
}

fn despawn_world_camera(mut commands: Commands, cameras: Query<Entity, With<WorldCamera>>) {
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
}

/// Turns mouse movement into camera rotation.
fn look(
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

    angles.yaw -= delta.x * input::LOOK_SENSITIVITY;
    angles.pitch =
        (angles.pitch - delta.y * input::LOOK_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, angles.yaw, angles.pitch, 0.0);
}

/// Moves the camera from keyboard input.
///
/// Free flight for now — movement follows where the camera is pointing,
/// with no gravity or collision. Both arrive with the voxel world.
fn fly(
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
        axis(&keys, input::RIGHT, input::LEFT),
        axis(&keys, input::UP, input::DOWN),
        axis(&keys, input::FORWARD, input::BACKWARD),
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

    let speed = if keys.pressed(input::SPRINT) {
        input::MOVE_SPEED * input::SPRINT_MULTIPLIER
    } else {
        input::MOVE_SPEED
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

/// Locks the cursor to the window and hides it, so mouse movement turns the
/// camera instead of wandering onto another monitor.
fn grab_cursor(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };

    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

/// Gives the cursor back, so a paused player can use their desktop.
fn release_cursor(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };

    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
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
