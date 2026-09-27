//! Building a renderable [`Mesh`] from a chunk's blocks.
//!
//! Naive per-block cubes would emit all six faces of every solid block,
//! including every face buried against a solid neighbour that the camera
//! can never see — for a half-solid chunk that's roughly 390,000 vertices
//! for what is really a flat slab. This instead only emits a face when the
//! block on that side is not solid, which is the difference between that
//! and a few thousand. Merging coplanar faces into larger quads (full
//! greedy meshing) is a further optimisation left for later — this only
//! removes faces nothing can ever see in the first place.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

use crate::config::world as config;
use crate::world::{Block, Chunk};

/// One cube face: the direction it points, the neighbouring block offset
/// that hides it, and its four corners.
///
/// Corners are listed counter-clockwise as seen from the direction `normal`
/// points — Bevy's default winding treats that as the front, so getting
/// this order right is what makes the face visible instead of invisible
/// (back-face culled) from outside the block.
struct Face {
    normal: Vec3,
    neighbor: IVec3,
    corners: [Vec3; 4],
}

const FACES: [Face; 6] = [
    // +X
    Face {
        normal: Vec3::new(1.0, 0.0, 0.0),
        neighbor: IVec3::new(1, 0, 0),
        corners: [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
        ],
    },
    // -X
    Face {
        normal: Vec3::new(-1.0, 0.0, 0.0),
        neighbor: IVec3::new(-1, 0, 0),
        corners: [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ],
    },
    // +Y
    Face {
        normal: Vec3::new(0.0, 1.0, 0.0),
        neighbor: IVec3::new(0, 1, 0),
        corners: [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
        ],
    },
    // -Y
    Face {
        normal: Vec3::new(0.0, -1.0, 0.0),
        neighbor: IVec3::new(0, -1, 0),
        corners: [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
        ],
    },
    // +Z
    Face {
        normal: Vec3::new(0.0, 0.0, 1.0),
        neighbor: IVec3::new(0, 0, 1),
        corners: [
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
    },
    // -Z
    Face {
        normal: Vec3::new(0.0, 0.0, -1.0),
        neighbor: IVec3::new(0, 0, -1),
        corners: [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ],
    },
];

/// Builds a mesh for `chunk`, in the chunk's own local space — each block is
/// a unit cube from its integer coordinate to `coordinate + 1`. The entity's
/// `Transform` (its [`crate::world::ChunkPos::origin`]) is what places that
/// in the world; this function knows nothing about where the chunk actually
/// is.
pub(super) fn chunk_mesh(chunk: &Chunk) -> Mesh {
    let size = config::CHUNK_SIZE;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let local = UVec3::new(x, y, z);
                if !is_solid(chunk.block(local)) {
                    continue;
                }

                for face in &FACES {
                    if is_solid(neighbor_block(chunk, local, face.neighbor)) {
                        continue;
                    }

                    let base = positions.len() as u32;
                    let origin = local.as_vec3();
                    for corner in face.corners {
                        positions.push((origin + corner).to_array());
                        normals.push(face.normal.to_array());
                    }
                    uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
                    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
    }

    // `MAIN_WORLD` (the CPU-side copy Bevy keeps by default) is dropped:
    // nothing reads a chunk mesh back after it's uploaded, since collision
    // and picking don't exist yet, so keeping it around would just be a
    // second copy of every vertex sitting in RAM for no reader.
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// The block on the far side of `local + offset`, or [`Block::Air`] if that
/// falls outside the chunk.
///
/// Neighbouring chunks aren't consulted — there is no lookup from here back
/// into [`crate::world::LoadedChunks`] — so a chunk edge always counts as
/// exposed rather than looking beyond this chunk's own data. That means a
/// face is drawn (and immediately hidden behind the next chunk over, once
/// there is one) at every chunk boundary; skipping those extra faces needs
/// this function to see past its own chunk, which is future work once
/// `RENDER_DISTANCE` is raised above `0` and boundaries are actually hidden.
fn neighbor_block(chunk: &Chunk, local: UVec3, offset: IVec3) -> Block {
    let neighbor = local.as_ivec3() + offset;
    let size = config::CHUNK_SIZE as i32;

    let in_bounds = neighbor.x >= 0
        && neighbor.y >= 0
        && neighbor.z >= 0
        && neighbor.x < size
        && neighbor.y < size
        && neighbor.z < size;

    if !in_bounds {
        return Block::Air;
    }

    chunk.block(neighbor.as_uvec3())
}

/// Whether a block should cull the face it shares with its neighbour.
///
/// The only place that needs to know which [`Block`] variants occupy
/// space — a real per-block table replaces this one `match` once more
/// variants exist that aren't simply solid or empty (glass, for instance).
fn is_solid(block: Block) -> bool {
    match block {
        Block::Air => false,
        Block::Solid => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Chunk;

    /// How many vertices `chunk_mesh` produced — 4 per emitted face.
    fn vertex_count(mesh: &Mesh) -> usize {
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            .map(|attribute| attribute.len())
            .unwrap_or(0)
    }

    #[test]
    fn an_empty_chunk_has_no_geometry() {
        let mesh = chunk_mesh(&Chunk::empty());
        assert_eq!(vertex_count(&mesh), 0);
    }

    #[test]
    fn an_isolated_solid_block_gets_all_six_faces() {
        let mut chunk = Chunk::empty();
        chunk.set_block(UVec3::new(0, 0, 0), Block::Solid);

        // Nothing neighbours it, so every face is exposed: 6 faces * 4
        // corners each.
        assert_eq!(vertex_count(&chunk_mesh(&chunk)), 6 * 4);
    }

    #[test]
    fn two_adjacent_solid_blocks_cull_the_face_between_them() {
        let mut chunk = Chunk::empty();
        chunk.set_block(UVec3::new(0, 0, 0), Block::Solid);
        chunk.set_block(UVec3::new(1, 0, 0), Block::Solid);

        // 12 faces between the two blocks, minus the two that face each
        // other across the shared boundary.
        assert_eq!(vertex_count(&chunk_mesh(&chunk)), 10 * 4);
    }

    #[test]
    fn a_block_at_the_chunk_edge_still_gets_its_boundary_face() {
        // Neighbouring chunks aren't consulted, so a block at local (0, y, z)
        // must still get its -X face — the edge counts as exposed, not
        // solid, until cross-chunk lookups exist.
        let mut chunk = Chunk::empty();
        chunk.set_block(UVec3::new(0, 0, 0), Block::Solid);

        assert_eq!(
            neighbor_block(&chunk, UVec3::new(0, 0, 0), IVec3::NEG_X),
            Block::Air
        );
    }
}
