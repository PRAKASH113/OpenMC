//! Turning a chunk position into its blocks.
//!
//! Deliberately minimal: a flat placeholder floor, not real terrain. Real
//! generation (noise-based heightmaps, biomes, caves) needs a sea-level
//! concept that `config::world` documents but does not wire in yet — see the
//! comment there. This exists so storage, coordinates, and the load/generate
//! cycle can be built and tested against *something* before terrain is real.

use super::block::Block;
use super::chunk::{Chunk, ChunkPos};
use crate::config::world as config;

/// Builds the chunk at `pos`.
///
/// A pure function — no ECS, no I/O — so it is unit-testable directly and,
/// later, safe to run off the main thread without touching anything else.
pub(super) fn generate(pos: ChunkPos) -> Chunk {
    let mut chunk = Chunk::empty();

    // Every chunk currently generates identically regardless of where it
    // is — `pos` is unused until terrain actually varies by position.
    let _ = pos;

    // Placeholder shape: solid in the bottom half, air above. Enough to
    // tell "generated" apart from "empty air" while there is no heightmap.
    // One fill rather than a per-block loop: `Chunk`'s storage puts every
    // block below a given height in one contiguous run (see
    // `Chunk::fill_below_height`), which real heightmap-based generation
    // will want just as much as this placeholder does.
    chunk.fill_below_height(config::CHUNK_SIZE / 2, Block::Solid);

    chunk
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    fn count_solid(chunk: &Chunk) -> usize {
        let size = config::CHUNK_SIZE;
        let mut count = 0;
        for x in 0..size {
            for y in 0..size {
                for z in 0..size {
                    if chunk.block(UVec3::new(x, y, z)) == Block::Solid {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    #[test]
    fn fills_exactly_the_bottom_half() {
        let size = config::CHUNK_SIZE as usize;
        let chunk = generate(ChunkPos { x: 0, y: 0, z: 0 });
        let expected = size * size * (size / 2);
        assert_eq!(count_solid(&chunk), expected);
    }

    #[test]
    fn generates_the_same_shape_regardless_of_position() {
        let here = generate(ChunkPos { x: 0, y: 0, z: 0 });
        let elsewhere = generate(ChunkPos { x: 5, y: -3, z: 12 });
        assert_eq!(count_solid(&here), count_solid(&elsewhere));
    }
}
