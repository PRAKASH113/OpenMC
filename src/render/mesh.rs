//! Building a renderable [`Mesh`] from a chunk's blocks.
//!
//! Uses [`binary_greedy_meshing`], a Rust port of the reference algorithm at
//! <https://github.com/cgerikj/binary-greedy-meshing>, rather than a
//! hand-rolled mesher: it doesn't just cull hidden faces, it *merges*
//! adjacent same-type faces into the largest quad it can, using bitwise
//! operations to do it fast. A flat 32×32 floor that a naive mesher (even a
//! face-culled one) would draw as 1,024 separate quads becomes exactly one.
//!
//! The crate needs its input *padded*: a chunk's own blocks in the middle,
//! surrounded by one block of its neighbours' data on every side, so it can
//! tell whether a boundary face is hidden by whatever's next door. That
//! padding is what gives us real cross-chunk face culling for free — see
//! [`write_neighbor_padding`] — something the previous hand-rolled mesher
//! never had (see `docs/AUDIT.md`, 1.6).

use std::collections::BTreeSet;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use binary_greedy_meshing as bgm;

use crate::config::world as config;
use crate::world::{Block, Chunk, ChunkPos, LoadedChunks};

/// The chunk size `binary_greedy_meshing` is instantiated for. The crate is
/// generic over this (a `const` parameter on [`bgm::Mesher`]), so it works
/// with our own [`config::CHUNK_SIZE`] rather than requiring its demo's 62.
const CS: usize = config::CHUNK_SIZE as usize;

/// Builds a mesh for the chunk at `pos`, in the chunk's own local space —
/// each block is a unit cube from its integer coordinate to
/// `coordinate + 1`. The entity's `Transform` (its
/// [`crate::world::ChunkPos::origin`]) is what places that in the world;
/// this function knows nothing about where the chunk actually is.
///
/// `world` is consulted for `pos`'s six face-adjacent neighbours, to pad the
/// input with their boundary blocks (see [`write_neighbor_padding`]) — not
/// for `chunk` itself, which the caller already has.
pub(super) fn chunk_mesh(pos: ChunkPos, chunk: &Chunk, world: &LoadedChunks) -> Mesh {
    let voxels = padded_voxels(pos, chunk, world);

    // `fast_mesh` needs an opacity mask maintained alongside the voxel
    // buffer, but is ~4x faster than the alternative (`mesh`, which derives
    // it internally on every call) — worth the extra step for something
    // rebuilt every time a chunk's neighbourhood changes. The transparency
    // mask stays all-zero: `Block` has no transparent variant yet, so
    // nothing is ever transparent, the same way the crate's own examples
    // handle a purely-opaque voxel set.
    let opaque_mask = bgm::compute_opaque_mask::<CS>(&voxels, &BTreeSet::new());
    let trans_mask = vec![0u64; bgm::Mesher::<CS>::CS_P2].into_boxed_slice();

    let mut mesher = bgm::Mesher::<CS>::new();
    mesher.fast_mesh(&voxels, &opaque_mask, &trans_mask);

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();

    for (face_index, quads) in mesher.quads.iter().enumerate() {
        let face = bgm::Face::from(face_index as u8);
        let normal = face.n().map(|component| component as f32);

        for &quad in quads {
            for vertex in face.vertices_packed(quad) {
                let [x, y, z] = vertex.xyz();
                positions.push([x as f32, y as f32, z as f32]);
                normals.push(normal);
                uvs.push([vertex.u() as f32, vertex.v() as f32]);
            }
        }
    }

    let indices = bgm::indices(positions.len() / 4);

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

/// Builds the padded voxel buffer `binary_greedy_meshing` needs: `chunk`'s
/// own blocks filling the interior, surrounded by one block of padding on
/// every side.
fn padded_voxels(pos: ChunkPos, chunk: &Chunk, world: &LoadedChunks) -> Box<[u16]> {
    let size = config::CHUNK_SIZE;
    let mut voxels = vec![0u16; bgm::Mesher::<CS>::CS_P3].into_boxed_slice();

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let value = voxel_id(chunk.block(UVec3::new(x, y, z)));
                voxels[bgm::pad_linearize::<CS>(x as usize, y as usize, z as usize)] = value;
            }
        }
    }

    write_neighbor_padding(&mut voxels, pos, world);
    voxels
}

/// Copies each of the six neighbouring chunks' boundary layer into the one
/// block of padding surrounding `voxels`, wherever that neighbour happens to
/// be loaded — this is what lets the mesher hide a face against a chunk on
/// the other side of a seam instead of always drawing it, the way the
/// previous hand-rolled mesher had to (it never looked past its own
/// chunk's data at all).
///
/// A neighbour that isn't loaded leaves its side of the padding as air —
/// `padded_voxels` already zero-initialised the buffer — so that side stays
/// exposed exactly as it would with no neighbour-awareness. Only the six
/// face-adjacent neighbours are read, never a diagonal one: the mesher's
/// face culling only ever looks at a cell's direct (non-diagonal)
/// neighbour, so a diagonal chunk's data is never actually consulted no
/// matter what it contains.
fn write_neighbor_padding(voxels: &mut [u16], pos: ChunkPos, world: &LoadedChunks) {
    let size = config::CHUNK_SIZE;
    let far = size - 1;
    let padded_far = (size + 1) as usize;

    // +X: our far padding, filled from the neighbour's near (x=0) face.
    if let Some(neighbor) = world.chunk(ChunkPos {
        x: pos.x + 1,
        ..pos
    }) {
        for y in 0..size {
            for z in 0..size {
                let value = voxel_id(neighbor.block(UVec3::new(0, y, z)));
                voxels[padded_index(padded_far, (y + 1) as usize, (z + 1) as usize)] = value;
            }
        }
    }
    // -X: our near padding, filled from the neighbour's far (x=size-1) face.
    if let Some(neighbor) = world.chunk(ChunkPos {
        x: pos.x - 1,
        ..pos
    }) {
        for y in 0..size {
            for z in 0..size {
                let value = voxel_id(neighbor.block(UVec3::new(far, y, z)));
                voxels[padded_index(0, (y + 1) as usize, (z + 1) as usize)] = value;
            }
        }
    }
    // +Y: our far padding, filled from the neighbour's near (y=0) face.
    if let Some(neighbor) = world.chunk(ChunkPos {
        y: pos.y + 1,
        ..pos
    }) {
        for x in 0..size {
            for z in 0..size {
                let value = voxel_id(neighbor.block(UVec3::new(x, 0, z)));
                voxels[padded_index((x + 1) as usize, padded_far, (z + 1) as usize)] = value;
            }
        }
    }
    // -Y: our near padding, filled from the neighbour's far (y=size-1) face.
    if let Some(neighbor) = world.chunk(ChunkPos {
        y: pos.y - 1,
        ..pos
    }) {
        for x in 0..size {
            for z in 0..size {
                let value = voxel_id(neighbor.block(UVec3::new(x, far, z)));
                voxels[padded_index((x + 1) as usize, 0, (z + 1) as usize)] = value;
            }
        }
    }
    // +Z: our far padding, filled from the neighbour's near (z=0) face.
    if let Some(neighbor) = world.chunk(ChunkPos {
        z: pos.z + 1,
        ..pos
    }) {
        for x in 0..size {
            for y in 0..size {
                let value = voxel_id(neighbor.block(UVec3::new(x, y, 0)));
                voxels[padded_index((x + 1) as usize, (y + 1) as usize, padded_far)] = value;
            }
        }
    }
    // -Z: our near padding, filled from the neighbour's far (z=size-1) face.
    if let Some(neighbor) = world.chunk(ChunkPos {
        z: pos.z - 1,
        ..pos
    }) {
        for x in 0..size {
            for y in 0..size {
                let value = voxel_id(neighbor.block(UVec3::new(x, y, far)));
                voxels[padded_index((x + 1) as usize, (y + 1) as usize, 0)] = value;
            }
        }
    }
}

/// A padded index built from already-padded coordinates (`0..=CS+1`),
/// unlike [`bgm::pad_linearize`], which only accepts unpadded (`0..CS`)
/// ones and adds the padding offset itself — exactly the reason it can't be
/// used to address the padding shell itself, which is what this is for.
/// Otherwise the identical formula: `x + 1` here is `x` there.
fn padded_index(px: usize, py: usize, pz: usize) -> usize {
    let cs_p = bgm::Mesher::<CS>::CS_P;
    pz + px * cs_p + py * cs_p * cs_p
}

/// Maps a [`Block`] to the voxel id `binary_greedy_meshing` expects. `0`
/// always means empty to the mesher, which is also [`Block`]'s own
/// `Default`, so [`Block::Air`] and "unwritten" agree.
fn voxel_id(block: Block) -> u16 {
    match block {
        Block::Air => 0,
        Block::Solid => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many vertices `chunk_mesh` produced — 4 per emitted quad.
    fn vertex_count(mesh: &Mesh) -> usize {
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            .map(|attribute| attribute.len())
            .unwrap_or(0)
    }

    fn solid_chunk() -> Chunk {
        let mut chunk = Chunk::empty();
        chunk.fill_below_height(config::CHUNK_SIZE, Block::Solid);
        chunk
    }

    #[test]
    fn an_empty_chunk_has_no_geometry() {
        let pos = ChunkPos { x: 0, y: 0, z: 0 };
        let world = LoadedChunks::default();
        let mesh = chunk_mesh(pos, &Chunk::empty(), &world);
        assert_eq!(vertex_count(&mesh), 0);
    }

    #[test]
    fn a_fully_solid_chunk_with_no_neighbors_merges_each_face_into_one_quad() {
        // This is the point of greedy meshing: a naive face-culled mesher
        // would emit 1,024 quads per side (one per exposed block face) for
        // a flat 32x32 wall. Merged, it's one quad per side — 6 total.
        let pos = ChunkPos { x: 0, y: 0, z: 0 };
        let world = LoadedChunks::default();
        let mesh = chunk_mesh(pos, &solid_chunk(), &world);
        assert_eq!(vertex_count(&mesh), 6 * 4);
    }

    #[test]
    fn a_face_shared_with_a_loaded_solid_neighbor_is_not_meshed() {
        // Two solid chunks side by side on the x axis: the face between
        // them is hidden on both sides, so meshing the first should produce
        // one fewer full face than the no-neighbours case above.
        let pos = ChunkPos { x: 0, y: 0, z: 0 };
        let mut world = LoadedChunks::default();
        world.insert(ChunkPos { x: 1, y: 0, z: 0 }, solid_chunk());

        let mesh = chunk_mesh(pos, &solid_chunk(), &world);
        assert_eq!(vertex_count(&mesh), 5 * 4);
    }
}
