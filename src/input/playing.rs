use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::app::states::PauseState;
use crate::config::controls::{KeyBinds, MOUSE_SENSITIVITY, MOVE_SPEED};
use crate::config::player::{EYE_HEIGHT, GRAVITY, JUMP_VELOCITY, TERMINAL_VELOCITY, WALK_SPEED};
use crate::playing::{
    collision_size, resolve_movement, ChunkManager, ChunkTile, ChunkWireframe, ControlMode,
    Grounded, PlayerCamera, PlayerVelocity,
};

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

/// Tab swaps between `ControlMode::Dev` (free-fly, no gravity/collision —
/// unchanged from before this mode existed) and `ControlMode::Player`
/// (gravity + collision box, see `docs/player-physics.md`). Unbound before
/// this, chosen since it's a common "toggle noclip/fly" key in other games.
pub fn toggle_control_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    mode: Res<State<ControlMode>>,
    mut next_mode: ResMut<NextState<ControlMode>>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        next_mode.set(match mode.get() {
            ControlMode::Dev => ControlMode::Player,
            ControlMode::Player => ControlMode::Dev,
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

/// `ControlMode::Player`'s movement: same mouse look as `movement_and_look`,
/// but WASD is horizontal-only (no flying — projected onto the ground
/// plane), gravity constantly pulls down, Space jumps only when grounded,
/// and the result goes through `resolve_movement` against `ChunkManager`
/// instead of writing straight to `Transform`. See `docs/player-physics.md`.
pub fn player_movement_and_look(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    binds: Res<KeyBinds>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    manager: Res<ChunkManager>,
    query: Single<(&mut Transform, &mut PlayerCamera, &mut PlayerVelocity, &mut Grounded)>,
) {
    let (mut transform, mut camera, mut velocity, mut grounded) = query.into_inner();

    let mouse_delta = mouse_motion.delta;
    if mouse_delta != Vec2::ZERO {
        camera.yaw -= mouse_delta.x * MOUSE_SENSITIVITY;
        camera.pitch -= mouse_delta.y * MOUSE_SENSITIVITY;
        camera.pitch = camera.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        transform.rotation = Quat::from_axis_angle(Vec3::Y, camera.yaw)
            * Quat::from_axis_angle(Vec3::X, camera.pitch);
    }

    // Horizontal-only: flatten forward/right onto the ground plane so
    // looking up/down doesn't tilt walking speed or direction.
    let flatten = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
    let forward = flatten(*transform.forward());
    let right = flatten(*transform.right());

    let bindings = [
        (binds.forward, forward),
        (binds.backward, -forward),
        (binds.right, right),
        (binds.left, -right),
    ];

    let mut horizontal = Vec3::ZERO;
    for (key, vec) in bindings {
        if keyboard.pressed(key) {
            horizontal += vec;
        }
    }
    let horizontal = horizontal.normalize_or_zero() * WALK_SPEED;
    velocity.0.x = horizontal.x;
    velocity.0.z = horizontal.z;

    if grounded.0 && keyboard.just_pressed(binds.up) {
        velocity.0.y = JUMP_VELOCITY;
    }
    velocity.0.y = (velocity.0.y - GRAVITY * time.delta_secs()).max(-TERMINAL_VELOCITY);

    let size = collision_size();
    let feet = transform.translation - Vec3::Y * EYE_HEIGHT;
    let delta = velocity.0 * time.delta_secs();

    let is_solid = |block: IVec3| manager.is_solid(block);
    let (new_feet, blocked) = resolve_movement(&is_solid, feet, size, delta);

    if blocked[0] {
        velocity.0.x = 0.0;
    }
    if blocked[2] {
        velocity.0.z = 0.0;
    }
    if blocked[1] {
        // Only a downward block counts as "grounded" — bonking a ceiling
        // while jumping shouldn't let you jump again mid-air.
        grounded.0 = velocity.0.y < 0.0;
        velocity.0.y = 0.0;
    } else {
        grounded.0 = false;
    }

    transform.translation = new_feet + Vec3::Y * EYE_HEIGHT;
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
