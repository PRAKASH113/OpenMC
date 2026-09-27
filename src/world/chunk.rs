//! Chunk data: a cube of blocks, and the coordinate system that locates it.
//!
//! A chunk stores its blocks in one flat array indexed by a computed offset,
//! not a nested `Vec<Vec<Vec<Block>>>` — one contiguous allocation, which
//! matters once meshing has to walk every block in every loaded chunk.

use bevy::prelude::*;

use super::block::Block;
use crate::config::world as config;

/// A chunk's position, in *chunk* units rather than block units.
///
/// One unit here is [`config::CHUNK_SIZE`] blocks. Kept as its own type
/// rather than a bare `IVec3` so a chunk coordinate and a block coordinate
/// can never be passed to the wrong place by mistake.
///
/// Also a `Component`: `render/` tags each chunk's mesh entity with the
/// `ChunkPos` it renders, both so a bulk "every chunk mesh" query needs no
/// separate marker type, and because knowing which chunk an entity is is
/// generally useful (debugging, and later picking/collision).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    /// The chunk containing `world_pos`, a position in world (block) units.
    ///
    /// Floor division, not truncation: `-0.5` must land in chunk `-1`, not
    /// chunk `0` — truncating toward zero would put the blocks either side
    /// of the origin in the same chunk on the positive axis and different
    /// chunks on the negative one.
    pub fn containing(world_pos: Vec3) -> Self {
        let size = config::CHUNK_SIZE as f32;
        Self {
            x: (world_pos.x / size).floor() as i32,
            y: (world_pos.y / size).floor() as i32,
            z: (world_pos.z / size).floor() as i32,
        }
    }

    /// This chunk's origin corner, in world (block) units — where `render/`
    /// places the entity that renders it, since a chunk's mesh is built in
    /// its own local space (block `(0,0,0)` to `(SIZE,SIZE,SIZE)`).
    pub fn origin(self) -> Vec3 {
        let size = config::CHUNK_SIZE as f32;
        Vec3::new(
            self.x as f32 * size,
            self.y as f32 * size,
            self.z as f32 * size,
        )
    }
}

/// One cube of blocks, [`config::CHUNK_SIZE`] to a side.
///
/// Blocks are stored flat and indexed by [`Self::index`] rather than nested
/// per-axis `Vec`s, so the whole chunk is one contiguous allocation.
pub(crate) struct Chunk {
    blocks: Vec<Block>,
}

impl Chunk {
    /// An empty (all-air) chunk of the configured size.
    ///
    /// `pub(crate)`, not `pub(super)`, alongside [`Self::block`] and
    /// [`Self::set_block`]: `render/`'s mesher tests build chunks by hand to
    /// check its face-culling directly, without going through generation.
    pub(crate) fn empty() -> Self {
        let volume = (config::CHUNK_SIZE as usize).pow(3);
        Self {
            blocks: vec![Block::default(); volume],
        }
    }

    /// The block at a *local* position within this chunk (each axis in
    /// `0..CHUNK_SIZE`).
    pub(crate) fn block(&self, local: UVec3) -> Block {
        self.blocks[Self::index(local)]
    }

    /// Sets the block at a *local* position within this chunk (each axis in
    /// `0..CHUNK_SIZE`).
    pub(crate) fn set_block(&mut self, local: UVec3, block: Block) {
        let index = Self::index(local);
        self.blocks[index] = block;
    }

    /// Flattens a local `(x, y, z)` into an index into [`Self::blocks`].
    ///
    /// Panics if any axis is outside `0..CHUNK_SIZE`, deliberately: an
    /// out-of-range coordinate here is a bug in the caller (generation or,
    /// later, meshing), not a valid "empty" query to answer quietly.
    fn index(local: UVec3) -> usize {
        let size = config::CHUNK_SIZE;
        assert!(
            local.x < size && local.y < size && local.z < size,
            "chunk-local position {local:?} is outside a {size}^3 chunk"
        );
        (local.x + local.y * size + local.z * size * size) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_positive_position_floors_toward_zero() {
        let size = config::CHUNK_SIZE as f32;
        let pos = ChunkPos::containing(Vec3::new(size * 1.5, 0.0, 0.0));
        assert_eq!(pos.x, 1);
    }

    #[test]
    fn a_negative_position_floors_away_from_zero() {
        // -0.5 blocks in is still inside chunk -1, not chunk 0 — truncation
        // would wrongly give 0.
        let pos = ChunkPos::containing(Vec3::new(-0.5, -0.5, -0.5));
        assert_eq!(
            pos,
            ChunkPos {
                x: -1,
                y: -1,
                z: -1
            }
        );
    }

    #[test]
    fn the_origin_is_chunk_zero() {
        let pos = ChunkPos::containing(Vec3::ZERO);
        assert_eq!(pos, ChunkPos { x: 0, y: 0, z: 0 });
    }

    #[test]
    fn set_and_read_back_a_block() {
        let mut chunk = Chunk::empty();
        let local = UVec3::new(1, 2, 3);
        assert_eq!(chunk.block(local), Block::Air);

        chunk.set_block(local, Block::Solid);
        assert_eq!(chunk.block(local), Block::Solid);
    }

    #[test]
    #[should_panic(expected = "outside a")]
    fn an_out_of_range_index_panics() {
        Chunk::index(UVec3::new(config::CHUNK_SIZE, 0, 0));
    }
}
