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

/// How many chunks are kept loaded around the player's current chunk,
/// horizontally.
///
/// A radius, not a bounding square: `0` means only the chunk the player is
/// standing in; `1` means every chunk within 1 chunk of it in a *circle*
/// (by squared distance) rather than the 3×3 square that bound would
/// suggest — a square's corners are `sqrt(2)` chunks away, noticeably
/// farther than the radius says, so a circle is what most games actually
/// load. At `1` that circle is a 5-chunk plus shape: the centre plus its
/// four straight-line neighbours, with the diagonal corners a square would
/// have included left out. See `world::in_render_distance`. Raising this
/// further needs no code change, since the loading loop already scans the
/// real radius.
pub const RENDER_DISTANCE: u32 = 2;

// The two constants below describe the world's vertical layout in chunks
// relative to sea level. They are not used yet — generation does not have a
// sea level concept until terrain noise is wired in — but they document the
// intended shape and will be read once that work lands.

// /// Number of chunk layers above sea level.
// ///
// /// Together with `CHUNKS_BELOW_SEA_LEVEL` this sets the total world height:
// /// `(CHUNKS_ABOVE_SEA_LEVEL + CHUNKS_BELOW_SEA_LEVEL) * CHUNK_SIZE` blocks.
// pub const CHUNKS_ABOVE_SEA_LEVEL: u32 = 20;

// /// Number of chunk layers below sea level (caves, bedrock, etc.).
// pub const CHUNKS_BELOW_SEA_LEVEL: u32 = 12;
