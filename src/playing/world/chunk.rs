use bevy::prelude::*;

use crate::config::world::CHUNK_SIZE as CHUNK_SIZE_I32;

pub const CHUNK_SIZE: usize = CHUNK_SIZE_I32 as usize;

/// Horizontal (X/Z) blocks of padding sampled from the neighboring chunk's
/// world position, so face culling at a chunk's edge sees the same terrain
/// the neighboring chunk will (see `docs/world-generation.md`). There's no
/// vertical padding: the world doesn't extend past y=0 or y=CHUNK_SIZE yet,
/// so treating "off the top/bottom" as air there is simply correct, not a
/// limitation.
pub const PADDING: usize = 1;
pub const PADDED_SIZE: usize = CHUNK_SIZE + PADDING * 2;

pub type BlockId = u8;
pub const AIR: BlockId = 0;
pub const SOLID: BlockId = 1;

/// Horizontal chunk coordinate (in chunks, not blocks). There's no vertical
/// chunking yet — the world is a single horizontal slab of chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// The chunk containing the given world-space position.
    pub fn from_world_pos(pos: Vec3) -> Self {
        Self {
            x: (pos.x / CHUNK_SIZE as f32).floor() as i32,
            z: (pos.z / CHUNK_SIZE as f32).floor() as i32,
        }
    }

    /// World-space origin (min corner) of this chunk.
    pub fn world_origin(self) -> Vec3 {
        Vec3::new(
            (self.x * CHUNK_SIZE as i32) as f32,
            0.0,
            (self.z * CHUNK_SIZE as i32) as f32,
        )
    }
}

/// Marker carrying a chunk entity's own position, so other systems (e.g. the
/// per-chunk triangle-count HUD in `dev_tools`) can correlate an entity back
/// to its chunk without needing access to `ChunkManager`'s internal map.
#[derive(Component, Debug, Clone, Copy)]
pub struct ChunkTile(pub ChunkPos);

/// A general 3D world-space grid coordinate: `CHUNK_SIZE`-sized cells on
/// *every* axis, including Y — unlike `ChunkPos`, which is horizontal-only
/// and tied to what actually gets rendered. For now this is purely an
/// informational readout (see the performance HUD) of "which cell is the
/// player in"; it's the seed of a general spatial grid (e.g. for a future
/// simulation distance covering more than what's rendered) rather than
/// something tied to chunk loading today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl GridPos {
    pub fn from_world_pos(pos: Vec3) -> Self {
        let cell = CHUNK_SIZE as f32;
        Self {
            x: (pos.x / cell).floor() as i32,
            y: (pos.y / cell).floor() as i32,
            z: (pos.z / cell).floor() as i32,
        }
    }

    /// World-space origin (min corner) of this cell.
    pub fn world_origin(self) -> Vec3 {
        let cell = CHUNK_SIZE as f32;
        Vec3::new(self.x as f32, self.y as f32, self.z as f32) * cell
    }

    /// Where `world_pos` falls *within* this cell — always `0..CHUNK_SIZE` on
    /// every axis, even for negative world coordinates, since `GridPos` itself
    /// already floor-divided to find the cell.
    pub fn local_offset(self, world_pos: Vec3) -> Vec3 {
        world_pos - self.world_origin()
    }
}

/// Raw block data for one chunk, padded by one block on every horizontal
/// (X/Z) side so the mesher can correctly cull faces against the actual
/// neighboring terrain instead of guessing "air" at chunk boundaries.
///
/// Coordinates: `y` is always local/unpadded (`0..CHUNK_SIZE`, the real world
/// height range). `x`/`z` come in two flavors — padded (`0..PADDED_SIZE`,
/// where padded coordinate `p` is world offset `p as i32 - PADDING as i32`
/// from the chunk origin) for the generator filling in border data, and
/// local (`0..CHUNK_SIZE`, the chunk's own real blocks) for everything else.
pub struct ChunkData {
    blocks: Vec<BlockId>,
}

impl ChunkData {
    pub fn empty() -> Self {
        Self {
            blocks: vec![AIR; PADDED_SIZE * CHUNK_SIZE * PADDED_SIZE],
        }
    }

    #[inline]
    fn index(padded_x: usize, y: usize, padded_z: usize) -> usize {
        padded_x + y * PADDED_SIZE + padded_z * PADDED_SIZE * CHUNK_SIZE
    }

    #[inline]
    pub fn set_padded(&mut self, padded_x: usize, y: usize, padded_z: usize, block: BlockId) {
        self.blocks[Self::index(padded_x, y, padded_z)] = block;
    }

    #[inline]
    pub fn get_padded(&self, padded_x: usize, y: usize, padded_z: usize) -> BlockId {
        self.blocks[Self::index(padded_x, y, padded_z)]
    }

    /// Get a real (non-padding) block using local chunk coordinates
    /// (`0..CHUNK_SIZE` on every axis).
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        self.get_padded(x + PADDING, y, z + PADDING)
    }
}
