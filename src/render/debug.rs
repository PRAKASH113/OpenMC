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

use std::collections::HashSet;

use bevy::camera::visibility::RenderLayers;
use bevy::input::common_conditions::{input_just_pressed, input_pressed};
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;

use crate::camera::WorldCamera;
use crate::config::debug as debug_config;
use crate::config::debug::ChunkGridMode;
use crate::config::input;
use crate::config::world as world_config;
use crate::player::{Player, in_creative};
use crate::states::GameState;
use crate::world::{ChunkPos, LoadedChunks};

/// How much of the chunk the player is standing in gets outlined right now.
/// The player rather than the camera: in third person the camera sits several
/// blocks behind, often in the neighbouring chunk.
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
/// of following the player — [`input::DEBUG_MODIFIER`] + `TOGGLE_CHUNK_GRID`
/// engages or releases it, mirroring how `world::ChunkLock` freezes chunk
/// loading.
///
/// Always starts `None` regardless of
/// `config::debug::CHUNK_GRID_INITIALLY_LOCKED`: locking needs an actual
/// chunk position, and none exists until the player has spawned — see
/// [`apply_initial_chunk_grid_lock`], which applies that setting once a
/// player actually exists to read a position from.
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
        .add_systems(
            Startup,
            (
                apply_initial_wireframe_state,
                move_gizmos_to_their_own_layer,
            ),
        )
        .add_systems(
            Update,
            (
                let_world_camera_see_gizmos,
                // Every key here only works in Creative. What they've
                // already switched on stays on in Survival; switch back to
                // Creative to turn it off.
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
                )
                    .run_if(in_creative),
                (
                    apply_initial_chunk_grid_lock,
                    draw_chunk_grid,
                    draw_sea_level_marker,
                )
                    .run_if(in_state(GameState::InGame)),
            ),
        );
}

/// The render layer gizmos draw on. Anything but `0`, which every camera and
/// mesh is on by default.
///
/// Gizmos render to *every* camera whose layers intersect theirs, and the UI
/// camera (`camera::camera_ui`) is a `Camera2d` — orthographic, parked at
/// the origin, never moving. Left on layer `0` with everything else, it drew
/// its own flat copy of every gizmo on top of the world: the chunk cube seen
/// face-on as a small square, the sea-level lines as a red segment, both
/// pinned to the middle of the screen however the world camera moved.
const GIZMO_LAYER: usize = 1;

fn move_gizmos_to_their_own_layer(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.render_layers = RenderLayers::layer(GIZMO_LAYER);
}

/// Adds [`GIZMO_LAYER`] to the world camera, on top of the default layer `0`
/// it renders chunk meshes from. The UI camera is left alone, on `0` only.
///
/// `Added` rather than an `OnEnter(InGame)` system: the camera is spawned in
/// `camera::camera_world`'s own `OnEnter` system, and systems in the same
/// schedule from different plugins have no guaranteed order.
fn let_world_camera_see_gizmos(mut commands: Commands, cameras: Query<Entity, Added<WorldCamera>>) {
    for camera in &cameras {
        commands
            .entity(camera)
            .insert(RenderLayers::layer(0).with(GIZMO_LAYER));
    }
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

/// Engages or releases [`ChunkGridLock`] on whichever chunk the player is
/// currently in.
fn toggle_chunk_grid_lock(
    mut lock: ResMut<ChunkGridLock>,
    player: Query<&Transform, With<Player>>,
) {
    lock.0 = match lock.0 {
        Some(_) => None,
        None => player.single().ok().map(|transform| {
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
/// player exists to read a position from — see [`ChunkGridLock`]'s doc
/// comment for why this can't just be a `Default` impl. Runs every frame
/// while `InGame` until it succeeds once, via the `Local<bool>` "have I
/// already done this" flag, then does nothing for the rest of the process's
/// lifetime.
fn apply_initial_chunk_grid_lock(
    mut lock: ResMut<ChunkGridLock>,
    mut applied: Local<bool>,
    player: Query<&Transform, With<Player>>,
) {
    if *applied || !debug_config::CHUNK_GRID_INITIALLY_LOCKED {
        return;
    }
    let Ok(transform) = player.single() else {
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

/// The axis colors every gizmo in this module uses consistently: a line
/// parallel to X is always this red, parallel to Y always this green,
/// parallel to Z always this blue — regardless of which cube or face it's
/// drawn on, so the color alone identifies the axis at a glance.
const AXIS_X_COLOR: Color = Color::srgb(1.0, 0.2, 0.2);
const AXIS_Y_COLOR: Color = Color::srgb(0.2, 1.0, 0.2);
const AXIS_Z_COLOR: Color = Color::srgb(0.2, 0.6, 1.0);

/// Draws the current [`ChunkGrid`] mode around whichever chunk is relevant —
/// the locked one from [`ChunkGridLock`] if set, otherwise whichever chunk
/// the player is currently in.
fn draw_chunk_grid(
    mut gizmos: Gizmos,
    grid: Res<ChunkGrid>,
    lock: Res<ChunkGridLock>,
    player: Query<&Transform, With<Player>>,
) {
    if grid.0 == ChunkGridMode::None {
        return;
    }

    let pos = match lock.0 {
        Some(locked) => locked,
        None => {
            let Ok(transform) = player.single() else {
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

        // Through the middle, to find the centre at a glance.
        gizmos.line(
            center - half * Vec3::X,
            center + half * Vec3::X,
            AXIS_X_COLOR,
        );
        gizmos.line(
            center - half * Vec3::Y,
            center + half * Vec3::Y,
            AXIS_Y_COLOR,
        );
        gizmos.line(
            center - half * Vec3::Z,
            center + half * Vec3::Z,
            AXIS_Z_COLOR,
        );

        // And a "+" on every face, so each axis carries on across the
        // cube's surface and the whole thing reads as eight sub-cubes. Each
        // face's cross uses the two axes lying *in* that face (a face
        // normal to X shows Y and Z), in their usual colors.
        let faces = [
            (Vec3::X, Vec3::Y, AXIS_Y_COLOR, Vec3::Z, AXIS_Z_COLOR),
            (Vec3::Y, Vec3::X, AXIS_X_COLOR, Vec3::Z, AXIS_Z_COLOR),
            (Vec3::Z, Vec3::X, AXIS_X_COLOR, Vec3::Y, AXIS_Y_COLOR),
        ];
        for (normal, a, color_a, b, color_b) in faces {
            for side in [-1.0, 1.0] {
                let face_center = center + side * half * normal;
                gizmos.line(face_center - half * a, face_center + half * a, color_a);
                gizmos.line(face_center - half * b, face_center + half * b, color_b);
            }
        }
    }
}

/// Draws the sea-level plane (absolute world height `0`, where
/// `world::generation`'s terrain noise is centred — below this line is
/// where `CHUNKS_BELOW_SEA_LEVEL` starts) as a grid: every currently-loaded
/// chunk *column* gets one line along each horizontal axis, each spanning
/// that column's own width and crossing at its centre — so as more columns
/// load, the individual crosses line up edge to edge into one continuous
/// grid over the whole loaded area, instead of a single pair of lines
/// following the camera around.
///
/// Deduplicated by `(x, z)`: every vertical layer of the same column would
/// otherwise draw the identical pair of lines on top of each other, once
/// per layer, for no visual difference.
fn draw_sea_level_marker(mut gizmos: Gizmos, visible: Res<SeaLevelLine>, world: Res<LoadedChunks>) {
    if !visible.0 {
        return;
    }

    let size = world_config::CHUNK_SIZE as f32;
    let half = size / 2.0;

    let mut columns = HashSet::new();
    for pos in world.positions() {
        if !columns.insert((pos.x, pos.z)) {
            continue;
        }

        let column_origin = ChunkPos {
            x: pos.x,
            y: 0,
            z: pos.z,
        }
        .origin();
        let center = Vec3::new(column_origin.x + half, 0.0, column_origin.z + half);

        gizmos.line(
            center - half * Vec3::X,
            center + half * Vec3::X,
            AXIS_X_COLOR,
        );
        gizmos.line(
            center - half * Vec3::Z,
            center + half * Vec3::Z,
            AXIS_Z_COLOR,
        );
    }
}
