//! The voxel world: chunk coordinates, storage, and generation.
//!
//! Owns the *data* — what blocks exist, where, and how a chunk's blocks are
//! produced — and nothing about turning that data into something on screen.
//! `render/` depends on this module for that, never the reverse: it reads
//! [`LoadedChunks`] and the [`ChunkLoaded`]/[`ChunkUnloaded`] messages below,
//! and this module has no idea `render/` exists.
//!
//! Chunks load and unload as the player moves, within
//! [`config::RENDER_DISTANCE`] chunk *columns* of their current column — a
//! *circle* (by squared distance), not the bounding square, so the far
//! corners of that square aren't loaded just because they happen to fall
//! inside it. Every column within range loads its *entire* height —
//! [`config::CHUNKS_ABOVE_SEA_LEVEL`] chunks above sea level down to
//! [`config::CHUNKS_BELOW_SEA_LEVEL`] below it — regardless of which layer
//! the player is actually standing on. See [`in_render_distance`] and
//! [`in_vertical_range`].
//!
//! Generation runs on a background thread, not the frame that requests it:
//! [`load_chunks_around_player`] only decides what's needed and spawns a
//! task for each newly-wanted chunk; [`apply_generated_chunks`] picks up
//! whichever tasks have finished, on any frame, and is what actually inserts
//! them into [`LoadedChunks`] and fires [`ChunkLoaded`]. See [`PendingChunks`].
//!
//! `debug` (debug builds only) can freeze loading entirely for testing — see
//! [`ChunkLock`].

mod block;
mod chunk;
#[cfg(debug_assertions)]
mod debug;
mod generation;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};

pub(crate) use block::Block;
pub(crate) use chunk::{Chunk, ChunkPos};

// Aliased: inside `crate::world`, a bare `world::` would read as this module
// rather than `crate::config::world`.
use crate::config::world as config;
use crate::player::Player;
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

    /// Whether the block at integer world coordinate `block` stops the player,
    /// for collision.
    ///
    /// Three cases, not two:
    /// - **Outside the world's vertical extent** (above
    ///   `CHUNKS_ABOVE_SEA_LEVEL` or below `CHUNKS_BELOW_SEA_LEVEL`): not
    ///   solid. Those chunks are never generated, and the space above the
    ///   world's top has to be flyable.
    /// - **Inside it but not loaded yet**: solid. Generation is async, so a
    ///   player can reach a chunk before its data exists. Treating it as a
    ///   wall means they wait for it instead of falling through ground that
    ///   hasn't arrived.
    /// - **Loaded**: whatever the block actually is.
    pub(crate) fn is_solid(&self, block: IVec3) -> bool {
        let (chunk_pos, local) = ChunkPos::of_block(block);
        if !in_vertical_range(
            chunk_pos.y,
            config::CHUNKS_ABOVE_SEA_LEVEL,
            config::CHUNKS_BELOW_SEA_LEVEL,
        ) {
            return false;
        }
        match self.chunks.get(&chunk_pos) {
            Some(chunk) => chunk.block(local) == Block::Solid,
            None => true,
        }
    }

    /// Every currently-loaded chunk's position, in no particular order.
    ///
    /// `render::debug`'s sea-level grid uses this to draw a marker per loaded
    /// chunk *column* — it needs to know what's loaded without caring about
    /// any chunk's actual block data, which `chunk` alone can't answer
    /// without already knowing a position to ask about.
    pub(crate) fn positions(&self) -> impl Iterator<Item = &ChunkPos> {
        self.chunks.keys()
    }

    /// Records `chunk` as loaded at `pos`, replacing whatever was there.
    ///
    /// The write counterpart to [`Self::chunk`] — kept `pub(crate)`, not
    /// just used internally by [`load_chunks_around_player`], so `render/`'s
    /// mesher tests can build a small `LoadedChunks` by hand (a couple of
    /// chunks with known neighbours) without going through generation or the
    /// ECS at all.
    pub(crate) fn insert(&mut self, pos: ChunkPos, chunk: Chunk) {
        self.chunks.insert(pos, chunk);
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

/// Chunks currently being generated on a background thread, keyed by
/// position so the same chunk is never queued twice while one attempt is
/// still in flight.
///
/// `generate` is a pure function with no ECS access (see `generation.rs`),
/// so running it inside [`AsyncComputeTaskPool`] needs nothing from it but
/// the `ChunkPos` — no world access to hand across the thread boundary, no
/// synchronisation to get right.
#[derive(Resource, Default)]
struct PendingChunks(HashMap<ChunkPos, Task<Chunk>>);

/// Owns chunk storage and generation.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedChunks>()
            .init_resource::<PendingChunks>()
            .init_resource::<ChunkLock>()
            .add_message::<ChunkLoaded>()
            .add_message::<ChunkUnloaded>()
            .add_systems(
                Update,
                (
                    load_chunks_around_player
                        .run_if(in_state(GameState::InGame).and_then(chunk_loading_unlocked)),
                    // Not gated on `chunk_loading_unlocked`: the lock stops
                    // *new* generation from being requested, but work already
                    // in flight should still land when it finishes rather
                    // than being stuck in limbo. Not gated on
                    // `Changed<Transform>` either (unlike the system above) —
                    // a background task can finish on a frame the player
                    // didn't move.
                    apply_generated_chunks.run_if(in_state(GameState::InGame)),
                ),
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

/// Whether `pos` is within `radius` chunks of `center`, horizontally, and
/// inside the world's fixed vertical extent (`above`/`below`, see
/// [`in_vertical_range`]) — *not* relative to `center`'s own height. The
/// player's column loads its whole height regardless of which layer they're
/// standing on; only the horizontal distance is measured from them.
///
/// A circle (squared distance), not the bounding square: the square's
/// corners are up to `radius * sqrt(2)` chunks away, about 27% more chunks
/// loaded for the same nominal render distance, and they would pop in and
/// out at an inconsistent distance depending on which direction the player
/// is moving. Squared rather than an actual `sqrt` so this stays cheap
/// across however many candidate chunks it's checked against.
fn in_render_distance(
    pos: ChunkPos,
    center: ChunkPos,
    radius: u32,
    above: u32,
    below: u32,
) -> bool {
    if !in_vertical_range(pos.y, above, below) {
        return false;
    }

    let radius = radius as i32;
    let dx = pos.x - center.x;
    let dz = pos.z - center.z;
    dx * dx + dz * dz <= radius * radius
}

/// Whether chunk-vertical-coordinate `y` falls within the world's fixed
/// vertical extent: chunk `0` (which starts exactly at sea level) up to
/// `above - 1`, and chunk `-1` down to `-(below as i32)`.
///
/// A fixed extent, not a radius around the player — unlike the horizontal
/// circle above, this never changes as the player moves up or down. That's
/// the whole point of `CHUNKS_ABOVE_SEA_LEVEL`/`_BELOW_SEA_LEVEL` describing
/// a world height rather than a render distance.
fn in_vertical_range(y: i32, above: u32, below: u32) -> bool {
    (-(below as i32)..(above as i32)).contains(&y)
}

/// Queues generation for every not-yet-loaded, not-already-queued chunk
/// within render distance of the player's current chunk, and drops every
/// loaded or in-flight chunk that has fallen outside it.
///
/// Eviction runs before queuing, so at the render-distance boundary neither
/// map is ever briefly holding both an old and a new chunk in the same slot.
/// Dropping a still-running [`Task`] cancels it — there's no point letting a
/// chunk finish generating for a position the player has already left.
///
/// `Changed<Transform>` on the query, not just `With<Player>`: if the player
/// hasn't moved, their chunk can't have changed, so there's nothing to load
/// or evict. This is what makes the system free while `Paused` — the only
/// systems that ever write to the player's `Transform` are gated on
/// `Playing`, so it genuinely never changes while paused — and free while
/// standing still. It doesn't distinguish moving from turning, since
/// turning rotates the same `Transform`; that's a coarser saving than a
/// hand-rolled "did the chunk actually change" check would give, but it's
/// the query filter `CLAUDE.md` asks for reached for first.
///
/// The player, not the camera: in third person the camera sits several
/// blocks behind the player and can be in a different chunk. What should
/// load is what's around the body walking through the world.
fn load_chunks_around_player(
    mut world: ResMut<LoadedChunks>,
    mut pending: ResMut<PendingChunks>,
    mut unloaded: MessageWriter<ChunkUnloaded>,
    player: Query<&Transform, (With<Player>, Changed<Transform>)>,
) {
    let Ok(transform) = player.single() else {
        return;
    };

    let center = ChunkPos::containing(transform.translation);
    let radius = config::RENDER_DISTANCE;
    let above = config::CHUNKS_ABOVE_SEA_LEVEL;
    let below = config::CHUNKS_BELOW_SEA_LEVEL;

    world.chunks.retain(|pos, _| {
        let keep = in_render_distance(*pos, center, radius, above, below);
        if !keep {
            info!("world: unloading chunk {pos:?}");
            unloaded.write(ChunkUnloaded { pos: *pos });
        }
        keep
    });
    pending
        .0
        .retain(|pos, _| in_render_distance(*pos, center, radius, above, below));

    let radius = radius as i32;
    let pool = AsyncComputeTaskPool::get();
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            // Every column within horizontal range loads its whole height —
            // not just `center.y`, the layer the player happens to be on.
            for y in -(below as i32)..(above as i32) {
                let pos = ChunkPos {
                    x: center.x + dx,
                    y,
                    z: center.z + dz,
                };
                if !in_render_distance(pos, center, radius as u32, above, below)
                    || world.chunks.contains_key(&pos)
                    || pending.0.contains_key(&pos)
                {
                    continue;
                }

                info!("world: queuing chunk {pos:?} for generation");
                let task = pool.spawn(async move { generation::generate(pos) });
                pending.0.insert(pos, task);
            }
        }
    }
}

/// Picks up whichever queued chunks have finished generating, inserts them
/// into [`LoadedChunks`], and fires [`ChunkLoaded`] for each.
///
/// Polls every pending task once, collecting finished ones into a plain
/// `Vec` before touching [`PendingChunks`] again — removing a finished
/// entry while still iterating the map it came from isn't possible in safe
/// Rust, and polling an already-finished task a second time isn't something
/// a future's contract promises to handle.
fn apply_generated_chunks(
    mut world: ResMut<LoadedChunks>,
    mut pending: ResMut<PendingChunks>,
    mut loaded: MessageWriter<ChunkLoaded>,
) {
    let finished: Vec<(ChunkPos, Chunk)> = pending
        .0
        .iter_mut()
        .filter_map(|(&pos, task)| block_on(poll_once(task)).map(|chunk| (pos, chunk)))
        .collect();

    for (pos, chunk) in finished {
        pending.0.remove(&pos);
        info!("world: generated chunk {pos:?}");
        world.insert(pos, chunk);
        loaded.write(ChunkLoaded { pos });
    }
}

/// Drops every loaded chunk when the world is left, so re-entering
/// generates fresh rather than reusing whatever was left over. Also cancels
/// anything still generating — otherwise a task from a previous visit to
/// `InGame` could finish after this one starts and insert into the fresh
/// [`LoadedChunks`] as if it belonged there.
fn clear_loaded_chunks(mut world: ResMut<LoadedChunks>, mut pending: ResMut<PendingChunks>) {
    *world = LoadedChunks::default();
    pending.0.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: i32, y: i32, z: i32) -> ChunkPos {
        ChunkPos { x, y, z }
    }

    #[test]
    fn the_center_chunk_is_always_in_range() {
        assert!(in_render_distance(pos(0, 0, 0), pos(0, 0, 0), 0, 1, 1));
    }

    #[test]
    fn a_square_corner_is_excluded_by_the_circular_radius() {
        // At radius 1 the bounding square would include (1, 1), but it's
        // sqrt(2) chunks away — outside a radius-1 circle.
        assert!(!in_render_distance(pos(1, 0, 1), pos(0, 0, 0), 1, 1, 1));
    }

    #[test]
    fn a_straight_edge_at_exactly_the_radius_is_included() {
        assert!(in_render_distance(pos(1, 0, 0), pos(0, 0, 0), 1, 1, 1));
    }

    #[test]
    fn one_chunk_past_the_radius_is_excluded() {
        assert!(!in_render_distance(pos(2, 0, 0), pos(0, 0, 0), 1, 1, 1));
    }

    #[test]
    fn render_distance_is_independent_of_the_players_own_height() {
        // The player standing on y = 5 doesn't shift which vertical layers
        // are in range — it's a fixed world extent, not a radius around them.
        assert!(in_render_distance(pos(0, 0, 0), pos(0, 5, 0), 0, 1, 1));
    }

    #[test]
    fn in_vertical_range_includes_sea_level_and_the_layer_below_it() {
        assert!(in_vertical_range(0, 1, 1));
        assert!(in_vertical_range(-1, 1, 1));
    }

    #[test]
    fn in_vertical_range_excludes_anything_past_the_configured_extent() {
        assert!(!in_vertical_range(1, 1, 1));
        assert!(!in_vertical_range(-2, 1, 1));
    }

    #[test]
    fn in_vertical_range_scales_with_above_and_below() {
        assert!(in_vertical_range(19, 20, 12));
        assert!(!in_vertical_range(20, 20, 12));
        assert!(in_vertical_range(-12, 20, 12));
        assert!(!in_vertical_range(-13, 20, 12));
    }

    #[test]
    fn an_unloaded_chunk_inside_the_world_counts_as_solid() {
        assert!(LoadedChunks::default().is_solid(IVec3::ZERO));
    }

    #[test]
    fn above_the_worlds_top_is_never_solid() {
        let top = (config::CHUNKS_ABOVE_SEA_LEVEL * config::CHUNK_SIZE) as i32;
        assert!(!LoadedChunks::default().is_solid(IVec3::new(0, top, 0)));
    }

    #[test]
    fn a_loaded_chunk_answers_from_its_own_blocks() {
        let mut world = LoadedChunks::default();
        let mut chunk = Chunk::empty();
        chunk.set_block(UVec3::new(1, 2, 3), Block::Solid);
        world.insert(pos(0, 0, 0), chunk);

        assert!(world.is_solid(IVec3::new(1, 2, 3)));
        assert!(!world.is_solid(IVec3::new(1, 3, 3)));
    }
}
