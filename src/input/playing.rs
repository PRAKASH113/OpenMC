use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::app::states::PauseState;
use crate::config::controls::{KeyBinds, MOUSE_SENSITIVITY, MOVE_SPEED};
use crate::playing::{ChunkTile, ChunkWireframe, PlayerCamera};

const MAX_PITCH: f32 = 1.54; // just under 90 degrees, avoids gimbal flip at the poles

/// Escape toggles Playing <-> Paused. Only scheduled while `GameState::Playing`
/// (see `InputPlugin`), which is also the only time `PauseState` exists at all.
pub fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    pause_state: Res<State<PauseState>>,
    mut next_pause_state: ResMut<NextState<PauseState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_pause_state.set(match pause_state.get() {
            PauseState::Unpaused => PauseState::Paused,
            PauseState::Paused => PauseState::Unpaused,
        });
    }
}

pub fn movement_and_look(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    binds: Res<KeyBinds>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    camera_query: Single<(&mut Transform, &mut PlayerCamera)>,
) {
    let (mut transform, mut camera) = camera_query.into_inner();

    let delta = mouse_motion.delta;
    if delta != Vec2::ZERO {
        camera.yaw -= delta.x * MOUSE_SENSITIVITY;
        camera.pitch -= delta.y * MOUSE_SENSITIVITY;
        camera.pitch = camera.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        transform.rotation = Quat::from_axis_angle(Vec3::Y, camera.yaw)
            * Quat::from_axis_angle(Vec3::X, camera.pitch);
    }

    let forward = transform.forward();
    let right = transform.right();

    let bindings = [
        (binds.forward, *forward),
        (binds.backward, -*forward),
        (binds.right, *right),
        (binds.left, -*right),
        (binds.up, Vec3::Y),
        (binds.down, -Vec3::Y),
    ];

    let mut direction = Vec3::ZERO;
    for (key, vec) in bindings {
        if keyboard.pressed(key) {
            direction += vec;
        }
    }

    if direction != Vec3::ZERO {
        transform.translation += direction.normalize() * MOVE_SPEED * time.delta_secs();
    }
}

/// Shift+1 toggles the terrain `Wireframe` debug overlay on every loaded
/// chunk. Bare `Digit1` is reserved for `input::switch_state`'s jump to
/// `GameState::Loading`, so this only fires when Shift is also held.
pub fn toggle_terrain_wireframe(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut wireframe: ResMut<ChunkWireframe>,
    chunks: Query<Entity, With<ChunkTile>>,
) {
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if !shift_held || !keyboard.just_pressed(KeyCode::Digit1) {
        return;
    }

    wireframe.0 = !wireframe.0;
    for entity in &chunks {
        if wireframe.0 {
            commands.entity(entity).insert(Wireframe);
        } else {
            commands.entity(entity).remove::<Wireframe>();
        }
    }
}

pub fn lock_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

pub fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}
