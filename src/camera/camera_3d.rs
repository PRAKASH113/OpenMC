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

/// Movement speed in units per second.
const MOVE_SPEED: f32 = 12.0;

/// Multiplier applied while the sprint key is held.
const SPRINT_MULTIPLIER: f32 = 3.0;

/// Radians of rotation per pixel of mouse movement.
const LOOK_SENSITIVITY: f32 = 0.002;

/// How close to straight up/down the pitch may get.
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
    let Ok((mut transform, mut angles)) = camera.single_mut() else {
        return;
    };

    let mut moved = false;
    for event in motion.read() {
        angles.yaw -= event.delta.x * LOOK_SENSITIVITY;
        angles.pitch =
            (angles.pitch - event.delta.y * LOOK_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        moved = true;
    }

    // Skip the quaternion rebuild on frames with no mouse input, which is
    // most of them when the player is standing still.
    if moved {
        transform.rotation = Quat::from_euler(EulerRot::YXZ, angles.yaw, angles.pitch, 0.0);
    }
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
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };

    let forward = *transform.forward();
    let right = *transform.right();

    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keys.pressed(KeyCode::Space) {
        direction += Vec3::Y;
    }
    if keys.pressed(KeyCode::ControlLeft) {
        direction -= Vec3::Y;
    }

    // Normalising a zero vector yields NaN, which would poison the
    // transform permanently.
    let Some(direction) = direction.try_normalize() else {
        return;
    };

    let speed = if keys.pressed(KeyCode::ShiftLeft) {
        MOVE_SPEED * SPRINT_MULTIPLIER
    } else {
        MOVE_SPEED
    };

    transform.translation += direction * speed * time.delta_secs();
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
