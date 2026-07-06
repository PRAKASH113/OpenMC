use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use super::chunk::{ChunkData, ChunkPos, CHUNK_SIZE, PADDED_SIZE, PADDING, SOLID};

/// Fractal Brownian motion over Perlin: several octaves of increasingly
/// fine, lower-amplitude detail layered on top of each other. A single
/// Perlin octave (the original approach) is one smooth wave at one
/// frequency — no matter how it's scaled, every hill ends up looking like a
/// rescaled copy of every other hill, which read as "every slope is the
/// same" once quantized (see below). Multiple octaves is what actually gives
/// a landscape *varied* local features (gentle rolling in some places,
/// sharper bumps in others) instead of one repeated shape everywhere.
pub type TerrainNoise = Fbm<Perlin>;

/// Kept low deliberately: with `TERRAIN_SCALE = 0.02` and `NOISE_LACUNARITY =
/// 2.0`, each additional octave doubles the finest wavelength present (5
/// octaves reached a ~3-block wavelength — close enough to single-column
/// noise that neighboring columns could differ wildly, producing isolated
/// one-column-wide "spike" columns many blocks taller than their footprint,
/// instead of a natural block-by-block staircase). 3 octaves keeps the finest
/// wavelength around 12 blocks, so height changes between neighbors stay
/// gradual.
const NOISE_OCTAVES: usize = 3;
const NOISE_LACUNARITY: f64 = 2.0;
const NOISE_PERSISTENCE: f64 = 0.5;

pub fn make_terrain_noise(seed: u32) -> TerrainNoise {
    Fbm::<Perlin>::new(seed)
        .set_octaves(NOISE_OCTAVES)
        .set_lacunarity(NOISE_LACUNARITY)
        .set_persistence(NOISE_PERSISTENCE)
}

// Restored a much larger, more dramatic height range — flattening the noise
// itself was the wrong lever for triangle count (it made the terrain boring
// without even being the real fix; see QUANTIZE_STEP below for the actual fix).
const TERRAIN_SCALE: f64 = 0.02;
const TERRAIN_HEIGHT: f64 = 16.0;
const BASE_HEIGHT: f64 = 10.0;

/// Snaps the raw sampled height to a step size before flooring to a block
/// count. A step of 1 is really just "round to the nearest block" — no
/// artificial widening. Anything higher than 1 guarantees the *smallest
/// possible* height difference between two neighboring columns is that many
/// blocks — with `QUANTIZE_STEP = 2`, no two adjacent columns could ever
/// differ by just 1 block, only 0 or 2+, so every single-column-wide feature
/// came out looking like two blocks stacked instead of a normal one-block
/// step (this is what made the terrain look "not blocky, just tall" — see
/// `docs/performance-investigation.md`). A voxel game's terrain is *supposed*
/// to have plain single-block steps; that's not something to engineer away.
/// The real lever for keeping the greedy mesher effective is smoothing the
/// noise itself (`NOISE_OCTAVES` above) so neighboring columns coincidentally
/// share a height often enough to merge, not forcing them to via quantization.
const QUANTIZE_STEP: i32 = 1;

/// Highest a column can possibly reach, used to keep the camera comfortably
/// above any generated terrain regardless of where it spawns.
pub const MAX_SURFACE_HEIGHT: f64 = BASE_HEIGHT + TERRAIN_HEIGHT;

/// Generates one chunk's block data from a layered (fbm) Perlin-noise
/// heightmap: solid below the sampled surface height for that (x, z) column,
/// air above it.
///
/// Fills the full *padded* range (see `chunk::PADDING`), sampling the same
/// noise function at the neighboring chunks' world positions for the border —
/// this is what lets the mesher cull faces at chunk boundaries correctly
/// without needing the neighboring chunk entities to exist yet.
pub fn generate_chunk(pos: ChunkPos, noise: &TerrainNoise) -> ChunkData {
    let mut data = ChunkData::empty();

    let origin_x = pos.x * CHUNK_SIZE as i32;
    let origin_z = pos.z * CHUNK_SIZE as i32;

    for padded_x in 0..PADDED_SIZE {
        for padded_z in 0..PADDED_SIZE {
            let world_x = (origin_x + padded_x as i32 - PADDING as i32) as f64;
            let world_z = (origin_z + padded_z as i32 - PADDING as i32) as f64;
            let noise_value = noise.get([world_x * TERRAIN_SCALE, world_z * TERRAIN_SCALE]);
            let raw_height = BASE_HEIGHT + noise_value * TERRAIN_HEIGHT;
            let surface_height =
                (raw_height / QUANTIZE_STEP as f64).round() as i32 * QUANTIZE_STEP;

            for y in 0..CHUNK_SIZE {
                if (y as i32) < surface_height {
                    data.set_padded(padded_x, y, padded_z, SOLID);
                }
            }
        }
    }

    data
}
