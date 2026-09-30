//! The world camera: the 3D view the player sees through.
//!
//! Unlike the UI camera this one is tied to a loaded world: it is spawned on
//! entering [`GameState::InGame`] and despawned on leaving, so no 3D view is
//! rendered while the player is in a menu.
//!
//! This module owns the camera *entity* — what it is and how it starts. Where
//! it sits each frame is [`super::follow`]'s job: a third-person view behind
//! the player. What the player's input does (looking, moving, capturing the
//! mouse) lives in [`crate::input`], which acts on the player, not on this
//! camera.

use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::transform::TransformSystems;

use super::follow::follow_player;
// Aliased: inside `crate::camera`, a bare `camera::` would read as this
// module rather than `crate::config::camera`.
use crate::config::camera as config;
use crate::states::GameState;

/// Marks the camera that renders the world.
#[derive(Component)]
pub struct WorldCamera;

/// Draw order for the world camera.
///
/// Lower than the UI camera's, so the interface renders on top of the world.
const ORDER: isize = 0;

/// Spawns the world camera with each world, keeps it behind the player, and
/// removes it afterwards.
pub struct WorldCameraPlugin;

impl Plugin for WorldCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_world_camera)
            .add_systems(OnExit(GameState::InGame), despawn_world_camera)
            // `PostUpdate`, before transforms propagate: the player has
            // already moved and turned this frame (in `Update`), so the
            // camera lands on where the player *is*, not where it was a
            // frame ago. A frame of lag here reads as jitter.
            .add_systems(
                PostUpdate,
                follow_player
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

fn spawn_world_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: ORDER,
            ..default()
        },
        // `Camera3d` requires a `Projection` and defaults to one if none is
        // given, but that default's FOV would then only exist in Bevy's
        // source — spawning it explicitly from `config::camera` is what
        // makes FOV a value this project actually owns and can tune.
        Projection::Perspective(PerspectiveProjection {
            fov: config::FOV_DEGREES.to_radians(),
            ..default()
        }),
        // Geometry edges genuinely alias here, unlike on the UI camera.
        Msaa::Sample4,
        // No starting transform worth choosing: `follow_player` places the
        // camera behind the player before the first frame renders.
        Transform::default(),
        WorldCamera,
    ));
}

fn despawn_world_camera(mut commands: Commands, cameras: Query<Entity, With<WorldCamera>>) {
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
}
