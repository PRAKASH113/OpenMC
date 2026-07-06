use bevy::pbr::wireframe::Wireframe;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use crate::config::world::RENDER_DISTANCE;
use crate::playing::{PlayerCamera, PlayingScreen};

use super::chunk::{ChunkPos, ChunkTile, CHUNK_SIZE};
use super::generator::{generate_chunk, make_terrain_noise, TerrainNoise};
use super::mesher::build_chunk_mesh;
use super::ChunkWireframe;

/// Debug-only: when true, `ChunkManager` never streams chunks based on player
/// position — it loads exactly this fixed 2x2 grid once (see
/// `fixed_debug_chunks`) and nothing else is ever loaded or unloaded
/// afterward. Used earlier to verify GPU frustum/occlusion culling in
/// isolation (confirmed working: `visible_chunks`/`visible_tris` correctly
/// dropped to 0 when looking away). Back to `false` now that the real
/// streaming manager is taking over again (`config::world::RENDER_DISTANCE`
/// scopes it to a single chunk instead) — kept around, still wired up, in
/// case culling ever needs isolating again later.
pub(crate) const DEBUG_FIXED_CHUNK_GRID: bool = false;

fn fixed_debug_chunks() -> [ChunkPos; 4] {
    [
        ChunkPos::new(0, 0),
        ChunkPos::new(1, 0),
        ChunkPos::new(0, 1),
        ChunkPos::new(1, 1),
    ]
}

/// How many chunks get generated + meshed per frame. Keeps the initial
/// render-distance load-in (and any large jump from fast movement) spread
/// across several frames instead of causing a single big hitch. True
/// async/threaded generation is a documented future upgrade, not this.
const CHUNKS_PER_FRAME: usize = 4;

/// Once the player is confirmed to be in a chunk, they must move this far
/// *past* that chunk's edge before a switch to a different chunk is
/// recognized. Without this hysteresis, standing near an exact chunk
/// boundary (or even ordinary floating-point jitter while moving) can flip
/// the computed `ChunkPos` back and forth every single frame — and each flip
/// despawns/respawns most of the 9x9 render window, a severe and sustained
/// FPS hit for as long as it keeps happening. See `docs/performance.md`.
const SWITCH_MARGIN: f32 = 2.0;

#[derive(Resource)]
pub struct ChunkManager {
    loaded: HashMap<ChunkPos, Entity>,
    pending: Vec<ChunkPos>,
    last_player_chunk: Option<ChunkPos>,
    noise: TerrainNoise,
    /// Every chunk looks identical today (flat debug green), so they all
    /// share this one material handle instead of each chunk allocating its
    /// own `StandardMaterial` asset — see `docs/optimisations.md`. Fewer
    /// unique material assets means fewer of the per-asset bookkeeping
    /// entities/observers `docs/performance.md` investigated.
    material: Option<Handle<StandardMaterial>>,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self {
            loaded: HashMap::default(),
            pending: Vec::new(),
            last_player_chunk: None,
            noise: make_terrain_noise(1337),
            material: None,
        }
    }
}

pub fn reset(mut commands: Commands) {
    let mut manager = ChunkManager::default();
    if DEBUG_FIXED_CHUNK_GRID {
        manager.pending = fixed_debug_chunks().to_vec();
    }
    commands.insert_resource(manager);
}

pub fn update_chunks(
    mut commands: Commands,
    mut manager: ResMut<ChunkManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    wireframe: Res<ChunkWireframe>,
    player: Single<&Transform, With<PlayerCamera>>,
) {
    if !DEBUG_FIXED_CHUNK_GRID {
        let current = resolve_player_chunk(manager.last_player_chunk, player.translation);

        if manager.last_player_chunk != Some(current) {
            manager.last_player_chunk = Some(current);
            recompute_desired_chunks(&mut manager, current, &mut commands);
        }
    }

    load_pending_chunks(
        &mut manager,
        &mut commands,
        &mut meshes,
        &mut materials,
        wireframe.0,
    );
}

/// Resolves the player's current chunk with hysteresis: if they're still
/// within the last confirmed chunk's bounds (expanded by `SWITCH_MARGIN`),
/// stick with it rather than recomputing from scratch every frame.
fn resolve_player_chunk(last: Option<ChunkPos>, world_pos: Vec3) -> ChunkPos {
    if let Some(last_chunk) = last {
        let origin = last_chunk.world_origin();
        let size = CHUNK_SIZE as f32;
        let still_inside = world_pos.x >= origin.x - SWITCH_MARGIN
            && world_pos.x < origin.x + size + SWITCH_MARGIN
            && world_pos.z >= origin.z - SWITCH_MARGIN
            && world_pos.z < origin.z + size + SWITCH_MARGIN;
        if still_inside {
            return last_chunk;
        }
    }
    ChunkPos::from_world_pos(world_pos)
}

fn recompute_desired_chunks(manager: &mut ChunkManager, current: ChunkPos, commands: &mut Commands) {
    let mut desired: HashSet<ChunkPos> = HashSet::default();
    for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
            desired.insert(ChunkPos::new(current.x + dx, current.z + dz));
        }
    }

    let to_unload: Vec<ChunkPos> = manager
        .loaded
        .keys()
        .filter(|pos| !desired.contains(*pos))
        .copied()
        .collect();
    for pos in to_unload {
        if let Some(entity) = manager.loaded.remove(&pos) {
            commands.entity(entity).despawn();
        }
    }

    manager.pending.retain(|pos| desired.contains(pos));
    for &pos in &desired {
        if !manager.loaded.contains_key(&pos) && !manager.pending.contains(&pos) {
            manager.pending.push(pos);
        }
    }
}

fn load_pending_chunks(
    manager: &mut ChunkManager,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    wireframe_enabled: bool,
) {
    let material = manager
        .material
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                // Vertex colors (set by the mesher) carry the actual per-face color.
                base_color: Color::WHITE,
                // `StandardMaterial`'s defaults (`perceptual_roughness: 0.5`,
                // `reflectance: 0.5`) are tuned for a semi-glossy "plastic"
                // look — on flat-shaded voxel terrain that showed up as an
                // unnatural, moving specular hotspot. Fully rough + zero
                // reflectance kills the specular term entirely, leaving plain
                // diffuse shading (light-direction-dependent, but no shine).
                perceptual_roughness: 1.0,
                reflectance: 0.0,
                ..default()
            })
        })
        .clone();

    let budget = CHUNKS_PER_FRAME.min(manager.pending.len());
    for _ in 0..budget {
        let pos = manager.pending.remove(0);
        let data = generate_chunk(pos, &manager.noise);
        let mesh = build_chunk_mesh(&data);

        let mut entity_commands = commands.spawn((
            PlayingScreen,
            ChunkTile(pos),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(pos.world_origin()),
        ));
        // Newly-generated chunks (e.g. from flying into unexplored terrain
        // while the toggle is already on) should match the current setting,
        // not always start plain — see `input::playing::toggle_terrain_wireframe`.
        if wireframe_enabled {
            entity_commands.insert(Wireframe);
        }

        manager.loaded.insert(pos, entity_commands.id());
    }
}
