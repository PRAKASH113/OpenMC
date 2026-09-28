//! Testing-only visual debugging: chunk mesh wireframes, a chunk-bounds
//! grid, and a sea-level marker.
//!
//! Compiled out of release entirely, and inert even in a debug build unless
//! [`crate::config::debug::TESTING_TOOLS_ENABLED`] is `true` — see
//! `world::debug` for why both gates exist; this module follows the
//! identical shape.
//!
//! Wireframes need the GPU features requested in `app::plugin` (debug builds
//! only). If `F8` never shows anything, check the startup log for
//! `WireframePlugin not loaded` — that means the adapter doesn't support
//! them, not that anything here is broken.

use bevy::input::common_conditions::{input_just_pressed, input_pressed};
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;

use crate::camera::WorldCamera;
use crate::config::debug as debug_config;
use crate::config::debug::ChunkGridMode;
use crate::config::input;
use crate::config::world as world_config;
use crate::states::GameState;
use crate::world::ChunkPos;

/// How much of the chunk the camera is standing in gets outlined right now.
///
/// The possible values ([`ChunkGridMode`]) and the starting one
/// (`config::debug::CHUNK_GRID_INITIAL_MODE`) both live in `config`, the
/// same as [`crate::config::input::SprintMode`] — this resource just holds
/// which one is currently active and knows how to cycle between them.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
struct ChunkGrid(ChunkGridMode);

impl Default for ChunkGrid {
    fn default() -> Self {
        Self(debug_config::CHUNK_GRID_INITIAL_MODE)
    }
}

impl ChunkGrid {
    fn cycle(&mut self) {
        self.0 = match self.0 {
            ChunkGridMode::None => ChunkGridMode::Outline,
            ChunkGridMode::Outline => ChunkGridMode::OutlineAndAxes,
            ChunkGridMode::OutlineAndAxes => ChunkGridMode::None,
        };
    }
}

/// While `Some`, the chunk-bounds grid stays fixed on that position instead
/// of following the camera — [`input::DEBUG_MODIFIER`] + `TOGGLE_CHUNK_GRID`
/// engages or releases it, mirroring how `world::ChunkLock` freezes chunk
/// loading.
///
/// Always starts `None` regardless of
/// `config::debug::CHUNK_GRID_INITIALLY_LOCKED`: locking needs an actual
/// chunk position, and none exists until the world camera has spawned — see
/// [`apply_initial_chunk_grid_lock`], which applies that setting once a
/// camera actually exists to read a position from.
#[derive(Resource, Default)]
struct ChunkGridLock(Option<ChunkPos>);

/// Whether the sea-level marker is currently visible.
#[derive(Resource, Clone, Copy)]
struct SeaLevelLine(bool);

impl Default for SeaLevelLine {
    fn default() -> Self {
        Self(debug_config::SEA_LEVEL_LINE_INITIALLY_VISIBLE)
    }
}

/// Registers every visual debugging feature, if testing tools are enabled.
///
/// A plain `if` around registration, not a run condition on each system: with
/// testing tools off, none of this exists in the schedule at all.
pub(super) fn register(app: &mut App) {
    if !debug_config::TESTING_TOOLS_ENABLED {
        return;
    }

    app.init_resource::<ChunkGrid>()
        .init_resource::<ChunkGridLock>()
        .init_resource::<SeaLevelLine>()
        .add_systems(Startup, apply_initial_wireframe_state)
        .add_systems(
            Update,
            (
                toggle_wireframe.run_if(input_just_pressed(input::TOGGLE_WIREFRAME)),
                // The lock combination takes the same key as the plain
                // cycle, so each condition has to explicitly exclude the
                // other's modifier state — otherwise one press of `F7`
                // while holding the modifier would fire both.
                cycle_chunk_grid_mode.run_if(
                    input_just_pressed(input::TOGGLE_CHUNK_GRID)
                        .and_then(not(input_pressed(input::DEBUG_MODIFIER))),
                ),
                toggle_chunk_grid_lock.run_if(
                    input_just_pressed(input::TOGGLE_CHUNK_GRID)
                        .and_then(input_pressed(input::DEBUG_MODIFIER)),
                ),
                toggle_sea_level_line.run_if(input_just_pressed(input::TOGGLE_SEA_LEVEL_LINE)),
                (
                    apply_initial_chunk_grid_lock,
                    draw_chunk_grid,
                    draw_sea_level_marker,
                )
                    .run_if(in_state(GameState::InGame)),
            ),
        );
}

/// Sets the starting wireframe visibility from
/// [`debug_config::WIREFRAME_INITIALLY_VISIBLE`]. A `Startup` system, not a
/// `Default` impl, since `WireframeConfig` is Bevy's own resource — but it's
/// already inserted by the time this runs, since `WireframePlugin::build`
/// adds it directly rather than through a `Startup` system of its own.
fn apply_initial_wireframe_state(mut config: ResMut<WireframeConfig>) {
    config.global = debug_config::WIREFRAME_INITIALLY_VISIBLE;
}

fn toggle_wireframe(mut config: ResMut<WireframeConfig>) {
    config.global = !config.global;
    info!(
        "render: wireframe {}",
        if config.global { "on" } else { "off" }
    );
}

fn cycle_chunk_grid_mode(mut grid: ResMut<ChunkGrid>) {
    grid.cycle();
    info!("render: chunk grid -> {:?}", grid.0);
}

/// Engages or releases [`ChunkGridLock`] on whichever chunk the camera is
/// currently in.
fn toggle_chunk_grid_lock(
    mut lock: ResMut<ChunkGridLock>,
    camera: Query<&Transform, With<WorldCamera>>,
) {
    lock.0 = match lock.0 {
        Some(_) => None,
        None => camera.single().ok().map(|transform| {
            let pos = ChunkPos::containing(transform.translation);
            info!("render: chunk grid locked to {pos:?}");
            pos
        }),
    };
    if lock.0.is_none() {
        info!("render: chunk grid lock released");
    }
}

/// Applies [`debug_config::CHUNK_GRID_INITIALLY_LOCKED`] the first frame a
/// world camera exists to read a position from — see [`ChunkGridLock`]'s
/// doc comment for why this can't just be a `Default` impl. Runs every
/// frame while `InGame` until it succeeds once, via the `Local<bool>` "have
/// I already done this" flag, then does nothing for the rest of the
/// process's lifetime.
fn apply_initial_chunk_grid_lock(
    mut lock: ResMut<ChunkGridLock>,
    mut applied: Local<bool>,
    camera: Query<&Transform, With<WorldCamera>>,
) {
    if *applied || !debug_config::CHUNK_GRID_INITIALLY_LOCKED {
        return;
    }
    let Ok(transform) = camera.single() else {
        return;
    };
    let pos = ChunkPos::containing(transform.translation);
    info!("render: chunk grid initially locked to {pos:?}");
    lock.0 = Some(pos);
    *applied = true;
}

fn toggle_sea_level_line(mut visible: ResMut<SeaLevelLine>) {
    visible.0 = !visible.0;
    info!(
        "render: sea-level line {}",
        if visible.0 { "on" } else { "off" }
    );
}

/// Draws the current [`ChunkGrid`] mode around whichever chunk is relevant —
/// the locked one from [`ChunkGridLock`] if set, otherwise whichever chunk
/// the camera is currently in.
fn draw_chunk_grid(
    mut gizmos: Gizmos,
    grid: Res<ChunkGrid>,
    lock: Res<ChunkGridLock>,
    camera: Query<&Transform, With<WorldCamera>>,
) {
    if grid.0 == ChunkGridMode::None {
        return;
    }

    let pos = match lock.0 {
        Some(locked) => locked,
        None => {
            let Ok(transform) = camera.single() else {
                return;
            };
            ChunkPos::containing(transform.translation)
        }
    };

    let size = world_config::CHUNK_SIZE as f32;
    let center = pos.origin() + Vec3::splat(size / 2.0);

    gizmos.cube(
        Transform::from_translation(center).with_scale(Vec3::splat(size)),
        Color::srgb(1.0, 1.0, 0.0),
    );

    if grid.0 == ChunkGridMode::OutlineAndAxes {
        let half = size / 2.0;
        gizmos.line(
            center - half * Vec3::X,
            center + half * Vec3::X,
            Color::srgb(1.0, 0.2, 0.2),
        );
        gizmos.line(
            center - half * Vec3::Y,
            center + half * Vec3::Y,
            Color::srgb(0.2, 1.0, 0.2),
        );
        gizmos.line(
            center - half * Vec3::Z,
            center + half * Vec3::Z,
            Color::srgb(0.2, 0.6, 1.0),
        );
    }
}

/// Draws a cross of two lines at sea level (absolute world height `0`,
/// where `world::generation`'s terrain noise is centred), through the
/// camera's current `(x, z)` position — below this line is where
/// `CHUNKS_BELOW_SEA_LEVEL` starts.
fn draw_sea_level_marker(
    mut gizmos: Gizmos,
    visible: Res<SeaLevelLine>,
    camera: Query<&Transform, With<WorldCamera>>,
) {
    if !visible.0 {
        return;
    }
    let Ok(transform) = camera.single() else {
        return;
    };

    // Long enough to reach past whatever's currently loaded, regardless of
    // the exact render distance.
    let span = ((world_config::RENDER_DISTANCE + 1) * world_config::CHUNK_SIZE) as f32;
    let x = transform.translation.x;
    let z = transform.translation.z;
    let color = Color::srgba(0.2, 0.6, 1.0, 0.9);

    gizmos.line(
        Vec3::new(x - span, 0.0, z),
        Vec3::new(x + span, 0.0, z),
        color,
    );
    gizmos.line(
        Vec3::new(x, 0.0, z - span),
        Vec3::new(x, 0.0, z + span),
        color,
    );
}
