mod chunk;
mod generator;
mod manager;
mod mesher;

use bevy::pbr::wireframe::WireframePlugin;
use bevy::prelude::*;

use crate::app::states::GameState;

pub(crate) use chunk::ChunkTile;
pub(crate) use generator::MAX_SURFACE_HEIGHT;
pub(crate) use manager::ChunkManager;

// `chunk::{ChunkPos, GridPos}` and `manager::DEBUG_FIXED_CHUNK_GRID` aren't
// re-exported right now — nothing outside `world` needs them yet (the
// per-chunk/coordinate HUD panels are still disconnected, see
// `docs/performance-investigation.md`). They're still `pub(crate)` at their
// definitions, so re-adding a `pub(crate) use` here is all reintroducing that
// HUD will need.

/// Whether chunk entities currently render with a `Wireframe` overlay —
/// toggled by Shift+1 (see `input::playing::toggle_terrain_wireframe`). Off by
/// default; a mesher-correctness verification aid, not a permanent visual
/// style (see `docs/world-generation.md`).
#[derive(Resource, Default)]
pub(crate) struct ChunkWireframe(pub bool);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WireframePlugin::default())
            .init_resource::<ChunkManager>()
            .init_resource::<ChunkWireframe>()
            .add_systems(OnEnter(GameState::Playing), manager::reset)
            .add_systems(
                Update,
                manager::update_chunks.run_if(in_state(GameState::Playing)),
            );
    }
}
