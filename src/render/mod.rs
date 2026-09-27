//! Turning the world's chunk data into what the player sees.
//!
//! [`crate::world`] owns *what blocks exist*; this owns *how they reach the
//! screen* — meshing a chunk's blocks into a [`Mesh`], giving it a material,
//! and spawning or despawning the entity that renders it as chunks load and
//! unload. Depends on `world/`, never the reverse — the same direction
//! `input/` depends on `camera/`.

mod material;
mod mesh;

use std::collections::HashMap;

use bevy::prelude::*;

use material::{ChunkMaterial, build_chunk_material};
use mesh::chunk_mesh;

use crate::states::GameState;
use crate::world::{ChunkLoaded, ChunkPos, ChunkUnloaded, LoadedChunks};

/// Marks the light that lights the world. Lives here, not `world/`: it
/// exists purely so chunk meshes are visible, the same category of thing as
/// the material they're given.
#[derive(Component)]
struct WorldLight;

/// Maps a loaded chunk's position to the entity currently rendering it.
///
/// Lets [`despawn_chunk_meshes`] find the right entity for a `ChunkUnloaded`
/// in one lookup rather than scanning every spawned chunk mesh.
#[derive(Resource, Default)]
struct ChunkEntities(HashMap<ChunkPos, Entity>);

/// Owns turning loaded chunks into what's on screen.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkEntities>()
            .add_systems(Startup, build_chunk_material)
            .add_systems(OnEnter(GameState::InGame), spawn_world_light)
            .add_systems(
                Update,
                (spawn_chunk_meshes, despawn_chunk_meshes).run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                OnExit(GameState::InGame),
                (despawn_world_light, despawn_all_chunk_meshes),
            );
    }
}

/// Lights the world. Angled so lit and shaded faces both show, which is what
/// makes the chunk read as solid geometry rather than a flat green cutout.
fn spawn_world_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 12.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        WorldLight,
    ));
}

fn despawn_world_light(mut commands: Commands, lights: Query<Entity, With<WorldLight>>) {
    for entity in &lights {
        commands.entity(entity).despawn();
    }
}

/// Builds and spawns a mesh entity for every chunk that just loaded.
fn spawn_chunk_meshes(
    mut commands: Commands,
    mut loaded: MessageReader<ChunkLoaded>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<ChunkMaterial>,
    world: Res<LoadedChunks>,
    mut entities: ResMut<ChunkEntities>,
) {
    for ChunkLoaded { pos } in loaded.read() {
        let Some(chunk) = world.chunk(*pos) else {
            // The chunk was unloaded again before this system got to it —
            // possible if a player crosses back out of render distance
            // within the same frame it loaded. Nothing to mesh.
            continue;
        };

        let mesh = meshes.add(chunk_mesh(chunk));
        let entity = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material.0.clone()),
                Transform::from_translation(pos.origin()),
                *pos,
            ))
            .id();
        entities.0.insert(*pos, entity);
    }
}

/// Despawns the mesh entity for every chunk that just unloaded.
fn despawn_chunk_meshes(
    mut commands: Commands,
    mut unloaded: MessageReader<ChunkUnloaded>,
    mut entities: ResMut<ChunkEntities>,
) {
    for ChunkUnloaded { pos } in unloaded.read() {
        if let Some(entity) = entities.0.remove(pos) {
            commands.entity(entity).despawn();
        }
    }
}

/// Clears every chunk mesh when the world is left, so re-entering starts
/// from nothing rather than showing whatever was last on screen.
///
/// Independent of [`ChunkUnloaded`] on purpose: `world/` drops its whole map
/// at once on the same transition rather than firing one message per chunk,
/// so this clears its own entities the same way instead of waiting for
/// messages that never come.
fn despawn_all_chunk_meshes(mut commands: Commands, mut entities: ResMut<ChunkEntities>) {
    for (_, entity) in entities.0.drain() {
        commands.entity(entity).despawn();
    }
}
