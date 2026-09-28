//! Turning a chunk position into its blocks.
//!
//! A single-octave-summed (fBm) noise heightmap: every `(x, z)` column gets
//! its own surface height, sampled in *world* space so neighbouring chunks'
//! terrain lines up at the seam instead of each chunk looking like an
//! island. The surface is centred on sea level — absolute world height
//! `0`, the boundary between chunk `y = 0` and chunk `y = -1` — and never
//! strays more than [`config::TERRAIN_AMPLITUDE`] blocks above or below it.
//!
//! That bound means most loaded chunks need no noise sampled at all: a
//! chunk entirely below the band is solid rock through and through, and one
//! entirely above it is solid air, both trivial. Only a chunk whose range
//! actually overlaps the band costs a noise sample per column — see
//! [`generate`].

use bevy::prelude::UVec3;
use noise::{Fbm, NoiseFn, Perlin};

use super::block::Block;
use super::chunk::{Chunk, ChunkPos};
use crate::config::world as config;

/// Builds the chunk at `pos`.
///
/// A pure function — no ECS, no I/O — so it is unit-testable directly and
/// safe to run off the main thread (see `world::PendingChunks`) without
/// touching anything else.
pub(super) fn generate(pos: ChunkPos) -> Chunk {
    let size = config::CHUNK_SIZE;
    let amplitude = config::TERRAIN_AMPLITUDE;
    let chunk_bottom = pos.y as f64 * size as f64;
    let chunk_top = chunk_bottom + size as f64;

    // Trivial cases first: a chunk entirely below the band the surface can
    // reach is solid rock through and through; entirely above it, solid
    // air. Neither needs a noise sample — most of the world will be one of
    // these two once `CHUNKS_ABOVE_SEA_LEVEL`/`_BELOW_SEA_LEVEL` are large,
    // since the band is a fixed height regardless of how tall the world is.
    if chunk_top <= -amplitude {
        let mut chunk = Chunk::empty();
        chunk.fill_below_height(size, Block::Solid);
        return chunk;
    }
    if chunk_bottom >= amplitude {
        return Chunk::empty();
    }

    // This chunk's range overlaps the band. Whatever part of it falls
    // below the band (there may be none, if this chunk starts above sea
    // level) is still unconditionally solid regardless of the noise, so
    // it's bulk-filled the same way as the fully-solid case above — only
    // the overlapping band itself needs a per-column sample.
    let guaranteed_solid_height = (-amplitude - chunk_bottom).clamp(0.0, size as f64) as u32;
    let mut chunk = Chunk::empty();
    chunk.fill_below_height(guaranteed_solid_height, Block::Solid);

    let noise = terrain_noise();
    let origin = pos.origin();

    for x in 0..size {
        for z in 0..size {
            let world_x = origin.x as f64 + x as f64;
            let world_z = origin.z as f64 + z as f64;
            let surface = terrain_surface(&noise, amplitude, world_x, world_z);
            let local_height = (surface - chunk_bottom).clamp(0.0, size as f64) as u32;

            for y in guaranteed_solid_height..local_height {
                chunk.set_block(UVec3::new(x, y, z), Block::Solid);
            }
        }
    }

    chunk
}

/// The fBm (fractal Brownian motion — several octaves of noise summed
/// together) noise used for the heightmap. Built fresh on every call rather
/// than cached anywhere: this is the only thing that makes generation
/// deterministic from `pos` alone, with no hidden state to keep in sync.
fn terrain_noise() -> Fbm<Perlin> {
    let mut noise = Fbm::<Perlin>::new(config::TERRAIN_SEED);
    noise.octaves = config::TERRAIN_OCTAVES;
    noise.frequency = config::TERRAIN_FREQUENCY;
    noise.persistence = config::TERRAIN_PERSISTENCE;
    noise
}

/// The terrain surface's absolute world height at `(world_x, world_z)` —
/// not a chunk-local value, since a column's surface doesn't belong to any
/// one chunk in particular.
///
/// Sampled in world space, not chunk-local space, so the same world
/// position always produces the same height no matter which chunk it's
/// generated as part of — the reason neighbouring chunks' terrain lines up
/// at the seam instead of each chunk looking like its own disconnected
/// island.
fn terrain_surface(noise: &Fbm<Perlin>, amplitude: f64, world_x: f64, world_z: f64) -> f64 {
    noise.get([world_x, world_z]) * amplitude
}

#[cfg(test)]
mod tests {
    use bevy::prelude::UVec3;

    use super::*;

    /// The height of the solid/air boundary in a column, recovered by
    /// scanning up from the bottom — valid because generation always fills a
    /// column solid from `0` to its height and air above, a heightmap by
    /// construction.
    fn scanned_height(chunk: &Chunk, x: u32, z: u32) -> u32 {
        let size = config::CHUNK_SIZE;
        (0..size)
            .find(|&y| chunk.block(UVec3::new(x, y, z)) == Block::Air)
            .unwrap_or(size)
    }

    #[test]
    fn generation_is_deterministic() {
        let pos = ChunkPos { x: 3, y: 0, z: -2 };
        let a = generate(pos);
        let b = generate(pos);
        for x in [0, 10, 31] {
            for z in [0, 10, 31] {
                assert_eq!(
                    scanned_height(&a, x, z),
                    scanned_height(&b, x, z),
                    "column ({x}, {z}) differed between two generations of the same chunk"
                );
            }
        }
    }

    #[test]
    fn terrain_height_varies_across_a_chunk() {
        let chunk = generate(ChunkPos { x: 0, y: 0, z: 0 });
        let heights: Vec<u32> = (0..config::CHUNK_SIZE)
            .map(|x| scanned_height(&chunk, x, 0))
            .collect();
        assert!(
            heights.iter().any(|&h| h != heights[0]),
            "every sampled column had the same height {} — terrain looks flat",
            heights[0]
        );
    }

    #[test]
    fn every_column_height_stays_within_the_chunk() {
        let chunk = generate(ChunkPos { x: 7, y: 0, z: -7 });
        for x in 0..config::CHUNK_SIZE {
            for z in 0..config::CHUNK_SIZE {
                let height = scanned_height(&chunk, x, z);
                assert!(
                    height <= config::CHUNK_SIZE,
                    "column ({x}, {z}) height {height} exceeds the chunk"
                );
            }
        }
    }

    #[test]
    fn different_chunk_positions_generate_different_terrain() {
        let here = generate(ChunkPos { x: 0, y: 0, z: 0 });
        let elsewhere = generate(ChunkPos { x: 4, y: 0, z: -3 });
        let differs = (0..config::CHUNK_SIZE)
            .any(|x| scanned_height(&here, x, 0) != scanned_height(&elsewhere, x, 0));
        assert!(
            differs,
            "two different chunk positions produced identical terrain"
        );
    }

    #[test]
    fn a_chunk_entirely_above_the_terrain_band_is_all_air() {
        // `TERRAIN_AMPLITUDE` is 10 blocks; chunk y = 5 spans absolute
        // 160..192, far above it.
        let chunk = generate(ChunkPos { x: 0, y: 5, z: 0 });
        assert_eq!(scanned_height(&chunk, 0, 0), 0);
        assert_eq!(scanned_height(&chunk, 16, 16), 0);
    }

    #[test]
    fn a_chunk_entirely_below_the_terrain_band_is_all_solid() {
        // Chunk y = -5 spans absolute -160..-128, far below the band.
        let chunk = generate(ChunkPos { x: 0, y: -5, z: 0 });
        assert_eq!(scanned_height(&chunk, 0, 0), config::CHUNK_SIZE);
        assert_eq!(scanned_height(&chunk, 16, 16), config::CHUNK_SIZE);
    }

    #[test]
    fn a_chunk_below_sea_level_still_has_both_ground_and_air() {
        // Chunk y = -1 spans absolute -32..0, overlapping the bottom of the
        // band — some columns should still be solid throughout (the
        // guaranteed-solid portion), others should show a surface. Scanned
        // across the whole face, not a single row: at this seed/frequency a
        // single 32-block line can stay on one side of the noise's zero
        // crossing even though the field as a whole varies (already proven
        // by `terrain_height_varies_across_a_chunk`).
        let chunk = generate(ChunkPos { x: 0, y: -1, z: 0 });
        let mut has_ground = false;
        let mut has_air = false;
        for x in 0..config::CHUNK_SIZE {
            for z in 0..config::CHUNK_SIZE {
                let height = scanned_height(&chunk, x, z);
                has_ground |= height > 0;
                has_air |= height < config::CHUNK_SIZE;
            }
        }
        assert!(has_ground, "chunk y = -1 has no ground at all");
        assert!(has_air, "chunk y = -1 is entirely solid, with no surface");
    }
}
