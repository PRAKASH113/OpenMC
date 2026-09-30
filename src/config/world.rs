//! World generation and layout configuration.
//!
//! Plain constants — no logic. Everything that controls how the world is
//! sized and laid out lives here so it can be found and tuned without reading
//! the generation code.

/// Side length of a chunk in blocks, applied to all three axes (X, Y, Z).
///
/// A chunk is always a cube: 32 × 32 × 32 blocks. Changing this value
/// changes the granularity of generation, meshing, and culling — larger
/// chunks mean fewer draw calls but more wasted work when only part of a
/// chunk is visible.
pub const CHUNK_SIZE: u32 = 32;

/// How many chunk *columns* are kept loaded around the player's current
/// column, horizontally.
///
/// A radius, not a bounding square: `0` means only the column the player is
/// standing in; `1` means every column within 1 chunk of it in a *circle*
/// (by squared distance) rather than the 3×3 square that bound would
/// suggest — a square's corners are `sqrt(2)` chunks away, noticeably
/// farther than the radius says, so a circle is what most games actually
/// load. At `1` that circle is a 5-column plus shape: the centre plus its
/// four straight-line neighbours, with the diagonal corners a square would
/// have included left out. See `world::in_render_distance`. Raising this
/// further needs no code change, since the loading loop already scans the
/// real radius.
///
/// Purely horizontal: every column within range loads its *entire* height
/// (`CHUNKS_ABOVE_SEA_LEVEL` + `CHUNKS_BELOW_SEA_LEVEL` chunks), not just
/// the layer the player happens to be on — see those two constants below.
pub const RENDER_DISTANCE: u32 = 3;

/// Number of chunk layers above sea level. Chunk `y = 0` is the first layer
/// above sea level — it starts exactly at absolute block height `0`, chunk
/// `y = 1` at `CHUNK_SIZE`, and so on up to chunk `y = CHUNKS_ABOVE_SEA_LEVEL - 1`.
///
/// Together with [`CHUNKS_BELOW_SEA_LEVEL`] this sets the total world
/// height: `(CHUNKS_ABOVE_SEA_LEVEL + CHUNKS_BELOW_SEA_LEVEL) * CHUNK_SIZE`
/// blocks. Every column within [`RENDER_DISTANCE`] loads all of it — see
/// `world::in_vertical_range`.
///
/// Set to `1` for now while vertical loading is new and worth watching
/// closely; the intended target is `20`. Raising either this or
/// `CHUNKS_BELOW_SEA_LEVEL` multiplies how many chunks a single loaded
/// column contains, so it's worth changing in small steps rather than
/// jumping straight to the target.
pub const CHUNKS_ABOVE_SEA_LEVEL: u32 = 1;

/// Number of chunk layers below sea level (caves, bedrock, etc). Chunk
/// `y = -1` is the first layer below sea level, `y = -2` the next, down to
/// `y = -(CHUNKS_BELOW_SEA_LEVEL as i32)`. See [`CHUNKS_ABOVE_SEA_LEVEL`]
/// for the rest.
///
/// Set to `1` for now, same reasoning; the intended target is `12`.
pub const CHUNKS_BELOW_SEA_LEVEL: u32 = 1;

// -------------------------------------------------------------------- terrain

/// Seed for the terrain noise. Same seed, same world, every time — change it
/// for a different one.
pub const TERRAIN_SEED: u32 = 0;

/// How many octaves of noise are summed to build the heightmap.
///
/// Each added octave layers finer detail on top of the last (fractal
/// Brownian motion — see [`noise::Fbm`]). More octaves means more visible
/// detail at the cost of more noise samples per column; 4-6 is a reasonable
/// range for a heightmap like this one.
pub const TERRAIN_OCTAVES: usize = 4;

/// Cycles of the base noise octave per block. Smaller means broader,
/// smoother hills; larger means tighter, more frequent ones.
pub const TERRAIN_FREQUENCY: f64 = 0.01;

/// How much each successive octave's contribution shrinks. Lower values
/// (closer to 0) make the terrain smoother; higher values (closer to 1)
/// make it rougher, since finer octaves then contribute nearly as much as
/// coarse ones.
pub const TERRAIN_PERSISTENCE: f64 = 0.5;

/// Height variation from the noise, in blocks, added to and subtracted from
/// sea level (absolute world height `0`) — the surface never goes above
/// `TERRAIN_AMPLITUDE` or below `-TERRAIN_AMPLITUDE`. A chunk entirely
/// outside that band is trivial to generate (solid rock or solid air, no
/// noise sampled at all — see `world::generation`); only chunks the band
/// actually passes through cost a noise sample per column.
pub const TERRAIN_AMPLITUDE: f64 = 10.0;
