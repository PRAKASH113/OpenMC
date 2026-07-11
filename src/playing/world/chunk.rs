use bevy::prelude::*;

use crate::config::world::CHUNK_SIZE as CHUNK_SIZE_I32;

pub const CHUNK_SIZE: usize = CHUNK_SIZE_I32 as usize;

/// Horizontal (X/Z) blocks of padding sampled from the neighboring chunk's
/// world position, so face culling at a chunk's edge sees the same terrain
/// the neighboring chunk will (see `docs/world-generation.md`).
pub const PADDING: usize = 1;
pub const PADDED_SIZE: usize = CHUNK_SIZE + PADDING * 2;

/// One extra Y layer *above* the chunk's own `CHUNK_SIZE` range, sampled the
/// same way as horizontal padding — the first block of whatever chunk sits
/// above this one, computed as a pure function of world position rather than
/// requiring that chunk to actually be loaded. Needed once chunks stack
/// vertically (see `docs/world-generation.md`): without it, the mesher's
/// up-facing pass has no way to know whether the chunk above continues solid
/// right at this chunk's own top boundary, and would incorrectly draw a
/// visible "floor" face there even when it's buried underground. There's no
/// equivalent bottom padding — see `mesher.rs`'s down-facing-pass doc comment
/// for why that direction doesn't need it.
pub const PADDED_HEIGHT: usize = CHUNK_SIZE + 1;

pub type BlockId = u8;
pub const AIR: BlockId = 0;
pub const STONE: BlockId = 1;
pub const GRASS: BlockId = 2;
pub const SAND: BlockId = 3;
pub const WATER: BlockId = 4;
/// Total distinct `BlockId` values, including `AIR` — sizes the mesher's
/// fixed per-type mask arrays (`[Mask; BLOCK_TYPE_COUNT]`) instead of a
/// `HashMap`, since the palette is small and known at compile time. Bump
/// this alongside adding a new `BlockId`.
pub const BLOCK_TYPE_COUNT: usize = 5;

/// Whether a block is opaque for face-culling purposes: any non-air block
/// counts, including `WATER` — two adjacent opaque blocks (water-on-sand,
/// water-on-water) still hide the shared face between them. This is
/// deliberately *not* the same question as "can the player walk through
/// this" (see `ChunkManager::is_solid`, which treats `WATER` as
/// non-solid) — water is opaque-looking but not physically solid.
#[inline]
pub fn is_opaque(block: BlockId) -> bool {
    block != AIR
}

/// The color a block renders as (flat per-vertex color, no textures yet).
pub fn block_color(block: BlockId) -> [f32; 4] {
    match block {
        STONE => [0.55, 0.55, 0.55, 1.0],
        GRASS => [0.20, 0.65, 0.25, 1.0],
        SAND => [0.82, 0.72, 0.45, 1.0],
        WATER => [0.20, 0.45, 0.85, 1.0],
        // AIR should never reach the mesher (no face is ever generated for
        // it), and anything else is an unrecognized `BlockId` — magenta is
        // the classic "this is a bug, not a design choice" color, chosen so
        // it's impossible to mistake for an intentional terrain color.
        _ => [1.0, 0.0, 1.0, 1.0],
    }
}

/// 3D chunk coordinate (in chunks, not blocks) — `x`/`z` horizontal as
/// before, `y` a vertical chunk layer (see `docs/world-generation.md` for
/// why vertical stacking needed real Y padding, not just a new field here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// The chunk containing the given world-space position.
    pub fn from_world_pos(pos: Vec3) -> Self {
        let cell = CHUNK_SIZE as f32;
        Self {
            x: (pos.x / cell).floor() as i32,
            y: (pos.y / cell).floor() as i32,
            z: (pos.z / cell).floor() as i32,
        }
    }

    /// World-space origin (min corner) of this chunk.
    pub fn world_origin(self) -> Vec3 {
        Vec3::new(
            (self.x * CHUNK_SIZE as i32) as f32,
            (self.y * CHUNK_SIZE as i32) as f32,
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
/// every axis. For now this is purely an informational readout (see the
/// performance HUD) of "which cell is the player in" — currently hidden from
/// the HUD but still computed, see `docs/performance.md`.
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
/// (X/Z) side (cross-chunk face culling, see `docs/world-generation.md`) and
/// one block on top only (cross-vertical-chunk face culling for the
/// up-facing pass — see `PADDED_HEIGHT`).
///
/// Coordinates: `x`/`z` come in two flavors — padded (`0..PADDED_SIZE`,
/// where padded coordinate `p` is world offset `p as i32 - PADDING as i32`
/// from the chunk origin) for the generator filling in border data, and
/// local (`0..CHUNK_SIZE`, the chunk's own real blocks) for everything else.
/// `y` similarly has a local range (`0..CHUNK_SIZE`, real blocks) and a
/// padded range (`0..PADDED_HEIGHT`, real blocks plus the one extra sampled
/// row on top) — there's no bottom or horizontal-style "offset" for `y`,
/// `set_padded`/`get_padded`'s `y` parameter is already the real coordinate.
pub struct ChunkData {
    blocks: Vec<BlockId>,
}

impl ChunkData {
    pub fn empty() -> Self {
        Self {
            blocks: vec![AIR; PADDED_SIZE * PADDED_HEIGHT * PADDED_SIZE],
        }
    }

    #[inline]
    fn index(padded_x: usize, y: usize, padded_z: usize) -> usize {
        padded_x + y * PADDED_SIZE + padded_z * PADDED_SIZE * PADDED_HEIGHT
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
