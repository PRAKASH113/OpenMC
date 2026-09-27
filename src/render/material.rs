//! The block palette: for now, one shared colour for every solid block.
//!
//! A texture atlas (a sprite per block type) arrives once there is more than
//! one visible block type worth telling apart — with only `Air` (invisible)
//! and `Solid`, a single flat colour says everything there is to say.

use bevy::prelude::*;

/// The only visible block colour right now.
const SOLID_COLOR: Color = Color::srgb(0.25, 0.65, 0.25);

/// The material every chunk mesh is given.
///
/// Built once at startup and reused by every chunk — one material handle
/// shared across every mesh, the same reasoning `scene.rs`'s placeholder
/// cubes used for their one shared mesh handle, just on the material side.
#[derive(Resource)]
pub(super) struct ChunkMaterial(pub Handle<StandardMaterial>);

/// Creates [`ChunkMaterial`].
pub(super) fn build_chunk_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let handle = materials.add(StandardMaterial::from_color(SOLID_COLOR));
    commands.insert_resource(ChunkMaterial(handle));
}
