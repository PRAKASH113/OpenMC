use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use super::chunk::{BlockId, ChunkData, ChunkPos, AIR, CHUNK_SIZE, GRASS, PADDED_SIZE, PADDING, SAND, STONE, WATER};

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

/// World Y below which a column that hasn't reached solid ground yet floods
/// with water instead of staying air — a fixed sea level, not tied to any
/// particular chunk. Columns whose `surface_height` is already above this
/// never see water at all; columns whose `surface_height` is below it get a
/// lake/ocean filling the gap up to this line.
const WATER_LEVEL: i32 = 8;

/// Dry land within this many blocks *above* `WATER_LEVEL` gets a sandy top
/// layer (beach) instead of grass; a column's *underwater* top layer (the
/// lake/ocean floor) is always sand too, regardless of depth — see
/// `block_at`. Both together are "sand near water," as requested.
const SAND_MARGIN: i32 = 2;

/// Surface heights at or above this get a stone top layer instead of grass —
/// "peaks of mountains," since only the upper portion of this world's height
/// range (`MAX_SURFACE_HEIGHT` above) ever reaches it.
const STONE_HEIGHT: f64 = BASE_HEIGHT + TERRAIN_HEIGHT * 0.7;

/// The sampled (quantized) surface height for a world (x, z) column — the
/// first air/water block sits at exactly this Y, everything below is solid.
fn surface_height(noise: &TerrainNoise, world_x: f64, world_z: f64) -> i32 {
    let noise_value = noise.get([world_x * TERRAIN_SCALE, world_z * TERRAIN_SCALE]);
    let raw_height = BASE_HEIGHT + noise_value * TERRAIN_HEIGHT;
    (raw_height / QUANTIZE_STEP as f64).round() as i32 * QUANTIZE_STEP
}

/// What block sits at `world_y` in a column whose surface sits at
/// `surface_height`. A pure function of those two numbers (no chunk
/// boundaries anywhere in the logic), so it gives identical answers however
/// far below the "surface" chunk it's asked about — a chunk many layers
/// underground just asks it about very negative-relative-to-surface `world_y`
/// values and naturally gets back solid stone every time, with no special
/// "this chunk is deep underground" case needed anywhere.
///
/// - Below the surface: solid. Only the exact top layer (`world_y ==
///   surface_height - 1`) gets a terrain-specific type (sand near water,
///   stone at high elevation, grass otherwise) — everything under that is
///   plain stone, matching "stone is just peaks of mountains" (only the
///   *exposed* surface reads as rocky; underground is uniformly stone,
///   there's no separate dirt/subsoil layer yet).
/// - At/above the surface but below `WATER_LEVEL`: water (a lake/ocean
///   filling the gap between low terrain and sea level).
/// - Otherwise: air.
fn block_at(surface_height: i32, world_y: i32) -> BlockId {
    if world_y < surface_height {
        if world_y != surface_height - 1 {
            return STONE;
        }
        if surface_height <= WATER_LEVEL + SAND_MARGIN {
            SAND
        } else if surface_height as f64 >= STONE_HEIGHT {
            STONE
        } else {
            GRASS
        }
    } else if world_y < WATER_LEVEL {
        WATER
    } else {
        AIR
    }
}

/// Generates one chunk's block data from a layered (fbm) Perlin-noise
/// heightmap — see `block_at` for the solid/water/air and block-type rules.
///
/// Fills the full padded range: one block of horizontal (X/Z) padding (see
/// `chunk::PADDING`) and one block of *vertical* padding on top only (see
/// `chunk::PADDED_HEIGHT`), sampling the same world-position-based rules at
/// the neighboring chunks' positions for both — this is what lets the mesher
/// cull faces at every chunk boundary (horizontal *and* vertical) correctly
/// without needing the neighboring chunk entities to exist yet.
pub fn generate_chunk(pos: ChunkPos, noise: &TerrainNoise) -> ChunkData {
    let mut data = ChunkData::empty();

    let origin_x = pos.x * CHUNK_SIZE as i32;
    let origin_y = pos.y * CHUNK_SIZE as i32;
    let origin_z = pos.z * CHUNK_SIZE as i32;

    for padded_x in 0..PADDED_SIZE {
        for padded_z in 0..PADDED_SIZE {
            let world_x = (origin_x + padded_x as i32 - PADDING as i32) as f64;
            let world_z = (origin_z + padded_z as i32 - PADDING as i32) as f64;
            let surface_height = surface_height(noise, world_x, world_z);

            // `y` runs one block past the chunk's own top (`CHUNK_SIZE`
            // inclusive) — see `chunk::PADDED_HEIGHT`.
            for y in 0..=CHUNK_SIZE {
                let world_y = origin_y + y as i32;
                let block = block_at(surface_height, world_y);
                data.set_padded(padded_x, y, padded_z, block);
            }
        }
    }

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_underground_is_always_stone() {
        assert_eq!(block_at(20, -1000), STONE);
        assert_eq!(block_at(-5, -1000), STONE);
    }

    #[test]
    fn far_above_the_surface_is_air() {
        assert_eq!(block_at(20, 1000), AIR);
    }

    #[test]
    fn low_dry_land_gets_a_sandy_shore() {
        // Surface height within SAND_MARGIN of WATER_LEVEL, and above it (dry).
        let height = WATER_LEVEL + SAND_MARGIN;
        assert_eq!(block_at(height, height - 1), SAND);
    }

    #[test]
    fn underwater_floor_is_always_sand() {
        // Surface height below WATER_LEVEL: the top solid block (lake/ocean
        // floor) is sand, and everything from there up to WATER_LEVEL is water.
        let height = WATER_LEVEL - 3;
        assert_eq!(block_at(height, height - 1), SAND, "lakebed should be sandy");
        assert_eq!(block_at(height, height), WATER);
        assert_eq!(block_at(height, WATER_LEVEL - 1), WATER);
        assert_eq!(block_at(height, WATER_LEVEL), AIR);
    }

    #[test]
    fn high_peaks_are_stone() {
        let height = STONE_HEIGHT.ceil() as i32;
        assert_eq!(block_at(height, height - 1), STONE);
    }

    #[test]
    fn mid_elevation_is_grass() {
        // Comfortably between the sand band and the stone threshold.
        let height = ((WATER_LEVEL + SAND_MARGIN) as f64 + STONE_HEIGHT) as i32 / 2;
        assert_eq!(block_at(height, height - 1), GRASS);
    }

    #[test]
    fn only_the_exact_top_layer_gets_a_special_type() {
        let height = 20;
        for world_y in 0..height - 1 {
            assert_eq!(block_at(height, world_y), STONE, "below the top layer should be plain stone");
        }
    }
}
