//! The voxel world: chunk coordinates, storage, and generation.
//!
//! Owns the *data* — what blocks exist, where, and how a chunk's blocks are
//! produced — and nothing about turning that data into something on screen.
//! `render/` depends on this module for that, never the reverse: it reads
//! [`LoadedChunks`] and the [`ChunkLoaded`]/[`ChunkUnloaded`] messages below,
//! and this module has no idea `render/` exists.
//!
//! Chunks load and unload as the player moves, within
//! [`config::RENDER_DISTANCE`] chunks of their current chunk — a *circle*
//! (by squared distance), not the bounding square, so the far corners of
//! that square aren't loaded just because they happen to fall inside it. See
//! [`in_render_distance`].
//!
//! `debug` (debug builds only) can freeze that entirely for testing — see
//! [`ChunkLock`].

mod block;
mod chunk;
#[cfg(debug_assertions)]
mod debug;
mod generation;

use std::collections::HashMap;

use bevy::prelude::*;

pub(crate) use block::Block;
pub(crate) use chunk::{Chunk, ChunkPos};

use crate::camera::WorldCamera;
// Aliased: inside `crate::world`, a bare `world::` would read as this module
// rather than `crate::config::world`.
use crate::config::world as config;
use crate::states::GameState;

/// Fired the frame a chunk is generated and inserted into [`LoadedChunks`].
#[derive(Message)]
pub(crate) struct ChunkLoaded {
    pub pos: ChunkPos,
}

/// Fired the frame a chunk falls out of render distance and is dropped from
/// [`LoadedChunks`].
///
/// Not fired when the whole world is torn down on leaving
/// [`GameState::InGame`] — [`clear_loaded_chunks`] drops everything at once,
/// and `render/` clears its own entities independently on the same exit
/// rather than reacting to one message per chunk.
#[derive(Message)]
pub(crate) struct ChunkUnloaded {
    pub pos: ChunkPos,
}

/// Every chunk currently generated and held in memory.
#[derive(Resource, Default)]
pub(crate) struct LoadedChunks {
    chunks: HashMap<ChunkPos, Chunk>,
}

impl LoadedChunks {
    /// The chunk at `pos`, if it is currently loaded.
    pub(crate) fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }
}

/// Whether chunk loading and unloading is frozen.
///
/// A testing aid, not a gameplay feature — see `world::debug` (debug builds
/// only) for how it's toggled. Defined unconditionally, unlike the toggle
/// itself, so [`load_chunks_around_player`]'s run condition below reads the
/// same in every build: always unlocked in release, since nothing there can
/// ever set it otherwise.
#[derive(Resource)]
struct ChunkLock(bool);

impl Default for ChunkLock {
    fn default() -> Self {
        Self(INITIALLY_LOCKED)
    }
}

/// The starting value of [`ChunkLock`].
///
/// In debug builds this reads `config::debug::CHUNK_LOCK_INITIALLY_ENGAGED`,
/// but only when `TESTING_TOOLS_ENABLED` is also `true` — otherwise the
/// world would start locked with no way to unlock it, since the `F9` hotkey
/// itself only exists when testing tools are enabled (`world::debug`). In
/// release this is always `false`: nothing there can read a debug-only
/// constant, and nothing should start a real build in a frozen state.
#[cfg(debug_assertions)]
const INITIALLY_LOCKED: bool = crate::config::debug::TESTING_TOOLS_ENABLED
    && crate::config::debug::CHUNK_LOCK_INITIALLY_ENGAGED;

#[cfg(not(debug_assertions))]
const INITIALLY_LOCKED: bool = false;

/// Owns chunk storage and generation.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedChunks>()
            .init_resource::<ChunkLock>()
            .add_message::<ChunkLoaded>()
            .add_message::<ChunkUnloaded>()
            .add_systems(
                Update,
                load_chunks_around_player
                    .run_if(in_state(GameState::InGame).and_then(chunk_loading_unlocked)),
            )
            .add_systems(OnExit(GameState::InGame), clear_loaded_chunks);

        #[cfg(debug_assertions)]
        debug::register(app);
    }
}

/// Run condition: whether [`ChunkLock`] currently allows loading/unloading.
fn chunk_loading_unlocked(lock: Res<ChunkLock>) -> bool {
    !lock.0
}

/// Whether `pos` is within [`config::RENDER_DISTANCE`] chunks of `center`,
/// horizontally, and on the same vertical layer — the vertical axis is a
/// separate, not-yet-wired concept (`config::world`'s sea-level constants),
/// so a chunk one layer above or below the player is never in range no
/// matter the radius.
///
/// A circle (squared distance), not the bounding square: the square's
/// corners are up to `radius * sqrt(2)` chunks away, about 27% more chunks
/// loaded for the same nominal render distance, and they would pop in and
/// out at an inconsistent distance depending on which direction the player
/// is moving. Squared rather than an actual `sqrt` so this stays cheap
/// across however many candidate chunks it's checked against.
fn in_render_distance(pos: ChunkPos, center: ChunkPos, radius: u32) -> bool {
    if pos.y != center.y {
        return false;
    }

    let radius = radius as i32;
    let dx = pos.x - center.x;
    let dz = pos.z - center.z;
    dx * dx + dz * dz <= radius * radius
}

/// Generates every not-yet-loaded chunk within render distance of the
/// player's current chunk, and drops every loaded chunk that has fallen
/// outside it.
///
/// Eviction runs before loading, so at the render-distance boundary the
/// chunk map is never briefly holding both an old and a new chunk in the
/// same slot.
///
/// `Changed<Transform>` on the query, not just `With<WorldCamera>`: if the
/// camera hasn't moved, its chunk can't have changed, so there's nothing to
/// load or evict. This is what makes the system free while `Paused` — the
/// only systems that ever write to the camera's `Transform` are gated on
/// `Playing`, so it genuinely never changes while paused — and free while
/// standing still. It doesn't distinguish moving from just looking around,
/// since both touch the same `Transform`; that's a coarser saving than a
/// hand-rolled "did the chunk actually change" check would give, but it's
/// the query filter `CLAUDE.md` asks for reached for first.
fn load_chunks_around_player(
    mut world: ResMut<LoadedChunks>,
    mut loaded: MessageWriter<ChunkLoaded>,
    mut unloaded: MessageWriter<ChunkUnloaded>,
    player: Query<&Transform, (With<WorldCamera>, Changed<Transform>)>,
) {
    let Ok(transform) = player.single() else {
        return;
    };

    let center = ChunkPos::containing(transform.translation);
    let radius = config::RENDER_DISTANCE;

    world.chunks.retain(|pos, _| {
        let keep = in_render_distance(*pos, center, radius);
        if !keep {
            info!("world: unloading chunk {pos:?}");
            unloaded.write(ChunkUnloaded { pos: *pos });
        }
        keep
    });

    let radius = radius as i32;
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let pos = ChunkPos {
                x: center.x + dx,
                y: center.y,
                z: center.z + dz,
            };
            if !in_render_distance(pos, center, config::RENDER_DISTANCE)
                || world.chunks.contains_key(&pos)
            {
                continue;
            }

            info!("world: generating chunk {pos:?}");
            world.chunks.insert(pos, generation::generate(pos));
            loaded.write(ChunkLoaded { pos });
        }
    }
}

/// Drops every loaded chunk when the world is left, so re-entering
/// generates fresh rather than reusing whatever was left over.
fn clear_loaded_chunks(mut world: ResMut<LoadedChunks>) {
    *world = LoadedChunks::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: i32, y: i32, z: i32) -> ChunkPos {
        ChunkPos { x, y, z }
    }

    #[test]
    fn the_center_chunk_is_always_in_range() {
        assert!(in_render_distance(pos(0, 0, 0), pos(0, 0, 0), 0));
    }

    #[test]
    fn a_different_vertical_layer_is_never_in_range() {
        assert!(!in_render_distance(pos(0, 1, 0), pos(0, 0, 0), 5));
    }

    #[test]
    fn a_square_corner_is_excluded_by_the_circular_radius() {
        // At radius 1 the bounding square would include (1, 1), but it's
        // sqrt(2) chunks away — outside a radius-1 circle.
        assert!(!in_render_distance(pos(1, 0, 1), pos(0, 0, 0), 1));
    }

    #[test]
    fn a_straight_edge_at_exactly_the_radius_is_included() {
        assert!(in_render_distance(pos(1, 0, 0), pos(0, 0, 0), 1));
    }

    #[test]
    fn one_chunk_past_the_radius_is_excluded() {
        assert!(!in_render_distance(pos(2, 0, 0), pos(0, 0, 0), 1));
    }
}
