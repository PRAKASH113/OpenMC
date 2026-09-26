//! The world camera: the 3D view the player sees through.
//!
//! Unlike the UI camera this one is tied to a loaded world: it is spawned on
//! entering [`GameState::InGame`] and despawned on leaving, so no 3D view is
//! rendered while the player is in a menu.
//!
//! This module owns the camera *entity* — what it is and how it starts. What
//! the player's input does to it (looking, flying, capturing the mouse) lives
//! in [`crate::input`], which depends on this module and never the reverse.

use bevy::prelude::*;
use bevy::render::view::Msaa;

use crate::states::GameState;

/// Marks the camera that renders the world.
#[derive(Component)]
pub struct WorldCamera;

/// The camera's orientation as accumulated look angles, in radians.
///
/// Stored rather than read back from the transform: recovering Euler angles
/// from a quaternion every frame is lossy and drifts, and it makes clamping
/// pitch awkward. Spawned with the camera, so it always matches the starting
/// view; [`crate::input`] updates it from the mouse.
#[derive(Component, Default)]
pub struct LookAngles {
    /// Rotation around the vertical axis. Unbounded — it wraps naturally.
    pub yaw: f32,
    /// Rotation around the horizontal axis. Kept just short of straight up
    /// and down by the look control, so the view can never flip over.
    pub pitch: f32,
}

/// Draw order for the world camera.
///
/// Lower than the UI camera's, so the interface renders on top of the world.
const ORDER: isize = 0;

/// Where the camera starts, looking back at the origin.
const START_POSITION: Vec3 = Vec3::new(8.0, 6.0, 16.0);

/// Spawns the world camera with each world, and removes it afterwards.
pub struct WorldCameraPlugin;

impl Plugin for WorldCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_world_camera)
            .add_systems(OnExit(GameState::InGame), despawn_world_camera);
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
