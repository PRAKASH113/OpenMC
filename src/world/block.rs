//! The block type: what a single voxel in the world can be.
//!
//! Minimal on purpose — just enough for storage and generation to exist and
//! be tested. Block variety (dirt, stone, ores, ...) is meaningless before
//! there is a renderer to tell them apart with, so it waits for `render/`.

/// A single voxel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Block {
    /// Empty space. The default, so an uninitialised chunk reads as air
    /// rather than solid ground.
    #[default]
    Air,
    /// Placeholder solid block, standing in for real terrain until
    /// generation grows past "is there ground here".
    Solid,
}
