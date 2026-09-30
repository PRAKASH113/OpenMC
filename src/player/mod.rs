//! The player: the body in the world that the controls move and the camera
//! follows.
//!
//! Owns the player entity — its model, where it starts, which way it's
//! looking ([`LookAngles`]) — and what happens to it: the game mode it plays
//! under (`game_mode`) and the gravity and terrain collision that act on it
//! (`physics`). What the keys *ask* for is read in [`crate::input`], which
//! writes it into [`MovementIntent`]; how the camera follows lives in
//! [`crate::camera`]. Both depend on this module, never the reverse.

#[cfg(debug_assertions)]
mod debug;
mod game_mode;
mod physics;

use bevy::prelude::*;

pub(crate) use game_mode::{ActiveGameMode, in_creative};
pub(crate) use physics::Motion;

use crate::states::{GameState, InGameState};

/// Marks the player entity.
#[derive(Component)]
pub(crate) struct Player;

/// Which way the player is looking, as accumulated angles in radians.
///
/// Stored rather than read back from a transform: recovering Euler angles
/// from a quaternion every frame is lossy and drifts, and it makes clamping
/// pitch awkward. The two angles drive different things. Yaw turns the
/// player's body, and pitch only tilts the camera, so the model always stays
/// upright. [`crate::input`] updates both from the mouse, and
/// [`crate::camera`] reads both to place the view.
#[derive(Component, Default)]
pub(crate) struct LookAngles {
    /// Rotation around the vertical axis. Unbounded — it wraps naturally.
    pub yaw: f32,
    /// Rotation around the horizontal axis. Kept just short of straight up
    /// and down by the look control, so the view can never flip over.
    pub pitch: f32,
}

/// What the controls are asking the player to do this frame.
///
/// Written by `input::movement` every frame the game is playing, and acted on
/// by `physics`, which decides what actually happens — a jump only counts
/// from the ground, and collision can stop any of it.
#[derive(Component, Default, Clone, Copy, PartialEq, Debug)]
pub(crate) struct MovementIntent {
    /// Horizontal velocity to move at, in world space, blocks per second.
    /// Always level (`y` is `0`).
    pub horizontal: Vec3,
    /// Vertical velocity while flying, blocks per second, positive up.
    /// Ignored on foot, where gravity decides vertical motion.
    pub vertical: f32,
    /// Whether a jump was asked for this frame. Ignored while flying.
    pub jump: bool,
}

/// Whether the player is flying: no gravity, and free vertical movement.
///
/// Only ever `true` in Creative. `input::movement` switches it with a
/// double-tap of Space or Left Shift, and `game_mode` resets it whenever the
/// mode changes.
#[derive(Component, Default)]
pub(crate) struct Flying(pub bool);

/// The systems that move the player. `input` runs its controls before this
/// set, so physics acts on this frame's keys rather than last frame's.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PlayerPhysics;

/// The player model, relative to the `assets/` folder.
///
/// Exported from Blockbench, 2 blocks tall with its origin between the feet,
/// and facing −Z — the same way Bevy's `Transform::forward` points, so it
/// needs no rotation to face where the player is going. Checked from the
/// file's own UVs, not assumed: the head's face texture is on its −Z side.
const MODEL_PATH: &str = "models/player.glb";

/// Where the player's feet start when a world loads.
///
/// Terrain never rises more than `config::world::TERRAIN_AMPLITUDE` (10)
/// blocks above sea level, so `15` starts the player clear of the ground,
/// and gravity takes them down to it. A constant rather than the real
/// surface height — see `docs/AUDIT.md` 4.8.
const SPAWN_POSITION: Vec3 = Vec3::new(8.0, 15.0, 16.0);

/// Spawns the player with each world, runs its game mode and physics, and
/// removes it afterwards.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        game_mode::register(app);
        #[cfg(debug_assertions)]
        debug::register(app);

        app.init_resource::<physics::CollisionEnabled>()
            .add_systems(OnEnter(GameState::InGame), spawn_player)
            .add_systems(OnExit(GameState::InGame), despawn_player)
            // `Playing` only: pausing freezes the player mid-air rather
            // than letting gravity carry on under the pause overlay.
            .add_systems(
                Update,
                physics::apply_physics
                    .in_set(PlayerPhysics)
                    .run_if(in_state(InGameState::Playing)),
            );
    }
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>, mode: Res<ActiveGameMode>) {
    // Start out looking toward the world origin, as the camera used to.
    let facing = Transform::from_translation(SPAWN_POSITION).looking_at(Vec3::ZERO, Vec3::Y);
    let (yaw, pitch, _) = facing.rotation.to_euler(EulerRot::YXZ);

    commands.spawn((
        Player,
        LookAngles { yaw, pitch },
        MovementIntent::default(),
        Flying(game_mode::starts_flying(mode.0)),
        physics::Motion::default(),
        // Yaw only: the body turns to face where the player looks, but never
        // tilts with it.
        Transform::from_translation(SPAWN_POSITION).with_rotation(Quat::from_rotation_y(yaw)),
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(MODEL_PATH))),
    ));
}

fn despawn_player(mut commands: Commands, players: Query<Entity, With<Player>>) {
    for entity in &players {
        commands.entity(entity).despawn();
    }
}
