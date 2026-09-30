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

    /// The chunk holding the block at integer world coordinate `block`, and
    /// that block's position inside it.
    ///
    /// Euclidean division, for the same reason [`Self::containing`] floors:
    /// block `-1` is the last block of chunk `-1`, not the first of chunk `0`.
    pub fn of_block(block: IVec3) -> (Self, UVec3) {
        let size = config::CHUNK_SIZE as i32;
        let chunk = Self {
            x: block.x.div_euclid(size),
            y: block.y.div_euclid(size),
            z: block.z.div_euclid(size),
        };
        let local = UVec3::new(
            block.x.rem_euclid(size) as u32,
            block.y.rem_euclid(size) as u32,
            block.z.rem_euclid(size) as u32,
        );
        (chunk, local)
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

    /// The six chunks sharing a face with this one — the only neighbours
    /// `render/`'s mesher reads border data from, since face culling never
    /// looks past a direct neighbour (see `render::mesh`). Order is
    /// arbitrary; nothing depends on it.
    pub fn face_neighbors(self) -> [ChunkPos; 6] {
        [
            ChunkPos {
                x: self.x + 1,
                ..self
            },
            ChunkPos {
                x: self.x - 1,
                ..self
            },
            ChunkPos {
                y: self.y + 1,
                ..self
            },
            ChunkPos {
                y: self.y - 1,
                ..self
            },
            ChunkPos {
                z: self.z + 1,
                ..self
            },
            ChunkPos {
                z: self.z - 1,
                ..self
            },
        ]
    }
}

/// One cube of blocks, [`config::CHUNK_SIZE`] to a side.
///
/// Blocks are stored flat and indexed by [`Self::index`] rather than nested
/// per-axis `Vec`s, so the whole chunk is one contiguous allocation. Within
/// that flat layout, `x` varies fastest and `y` slowest ("YZX" — the layout
/// Minecraft uses), so every block sharing a height sits in one contiguous
/// run. That matters because terrain is naturally described by height (a
/// heightmap says "solid up to here"), so this layout turns "fill everything
/// below this height" into one contiguous slice fill (see
/// [`Self::fill_below_height`]) instead of visiting every block one at a
/// time in an order that scatters across memory.
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

    /// Sets every block with `y < height` (every `x` and `z`) to `block`, in
    /// one pass.
    ///
    /// Relies directly on [`Self::index`]'s layout: with `y` the slowest
    /// axis, every block below a given height occupies one contiguous run
    /// at the front of [`Self::blocks`], so this is a single slice fill
    /// rather than `height * CHUNK_SIZE * CHUNK_SIZE` individual
    /// bounds-checked writes through [`Self::set_block`]. If that layout
    /// ever changes, this method has to change with it — it is the one
    /// place outside [`Self::index`] itself that assumes it.
    pub(crate) fn fill_below_height(&mut self, height: u32, block: Block) {
        let size = config::CHUNK_SIZE;
        assert!(
            height <= size,
            "fill height {height} exceeds chunk size {size}"
        );
        let count = (height * size * size) as usize;
        self.blocks[..count].fill(block);
    }

    /// Flattens a local `(x, y, z)` into an index into [`Self::blocks`].
    ///
    /// `x` varies fastest, `z` next, `y` slowest — see the type-level doc
    /// comment for why. Panics if any axis is outside `0..CHUNK_SIZE`,
    /// deliberately: an out-of-range coordinate here is a bug in the caller
    /// (generation or, later, meshing), not a valid "empty" query to answer
    /// quietly.
    fn index(local: UVec3) -> usize {
        let size = config::CHUNK_SIZE;
        assert!(
            local.x < size && local.y < size && local.z < size,
            "chunk-local position {local:?} is outside a {size}^3 chunk"
        );
        (local.x + local.z * size + local.y * size * size) as usize
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
    fn of_block_splits_a_block_into_its_chunk_and_local_position() {
        let size = config::CHUNK_SIZE as i32;
        let (chunk, local) = ChunkPos::of_block(IVec3::new(-1, 0, size + 1));
        assert_eq!(chunk, ChunkPos { x: -1, y: 0, z: 1 });
        assert_eq!(local, UVec3::new(size as u32 - 1, 0, 1));
    }

    #[test]
    fn face_neighbors_are_the_six_positions_one_step_away_on_one_axis() {
        let center = ChunkPos { x: 5, y: -1, z: 2 };
        let neighbors = center.face_neighbors();
        assert_eq!(neighbors.len(), 6);
        for neighbor in neighbors {
            let dx = (neighbor.x - center.x).abs();
            let dy = (neighbor.y - center.y).abs();
            let dz = (neighbor.z - center.z).abs();
            // Exactly one axis differs, and only by 1 — never a diagonal.
            assert_eq!(
                dx + dy + dz,
                1,
                "{neighbor:?} is not a face neighbour of {center:?}"
            );
        }
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

    /// Pins the index formula's axis order down directly: `x` fastest, `z`
    /// next, `y` slowest. `fill_below_height` depends on this exact order —
    /// if it ever changes, this test should be the first thing to fail,
    /// not a silent slowdown discovered later.
    #[test]
    fn x_is_fastest_z_is_next_y_is_slowest() {
        let size = config::CHUNK_SIZE;
        assert_eq!(Chunk::index(UVec3::new(0, 0, 0)), 0);
        assert_eq!(Chunk::index(UVec3::new(1, 0, 0)), 1);
        assert_eq!(Chunk::index(UVec3::new(0, 0, 1)), size as usize);
        assert_eq!(Chunk::index(UVec3::new(0, 1, 0)), (size * size) as usize);
    }

    #[test]
    fn fill_below_height_fills_exactly_that_many_layers() {
        let size = config::CHUNK_SIZE;
        let mut chunk = Chunk::empty();
        chunk.fill_below_height(size / 2, Block::Solid);

        // Every block strictly below the height is solid...
        assert_eq!(chunk.block(UVec3::new(0, 0, 0)), Block::Solid);
        assert_eq!(
            chunk.block(UVec3::new(size - 1, size / 2 - 1, size - 1)),
            Block::Solid
        );
        // ...and everything at or above it is untouched.
        assert_eq!(chunk.block(UVec3::new(0, size / 2, 0)), Block::Air);
    }

    #[test]
    #[should_panic(expected = "exceeds chunk size")]
    fn fill_below_height_rejects_a_height_past_the_chunk() {
        Chunk::empty().fill_below_height(config::CHUNK_SIZE + 1, Block::Solid);
    }
}
