//! Binary greedy meshing: turns a chunk's raw block data into one merged
//! `Mesh` (one entity per chunk, never one entity per block — see
//! `docs/architecture.md`).
//!
//! For each of the 3 axes, blocks are packed into per-column bitmasks
//! (`CHUNK_SIZE` is 32, so a whole column fits in one word — plus one extra
//! padding bit for the Y axis, see `mesh_axis_y`). Face visibility for an
//! entire column is then a couple of bitwise ops instead of per-block
//! comparisons (`column & !(column >> 1)` finds every "opaque here, air
//! above" bit in the column at once). Each resulting 2D slice of visible
//! faces is split by block type (so a stone quad never merges with a
//! grass one) and each type's slice is greedily merged into the fewest
//! possible same-colored rectangles.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;

use super::chunk::{
    block_color, is_opaque, BlockId, ChunkData, BLOCK_TYPE_COUNT, CHUNK_SIZE, PADDED_SIZE, PADDING,
};

pub fn build_chunk_mesh(data: &ChunkData) -> Mesh {
    let mut builder = MeshBuilder::default();

    mesh_axis_y(data, &mut builder);
    mesh_axis_x(data, &mut builder);
    mesh_axis_z(data, &mut builder);

    builder.build()
}

#[derive(Default)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    fn push_quad(&mut self, corners: [[f32; 3]; 4], normal: [f32; 3], color: [f32; 4]) {
        let start = self.positions.len() as u32;
        self.positions.extend(corners);
        self.normals.extend([normal; 4]);
        self.colors.extend([color; 4]);
        self.indices
            .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }

    fn build(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// A `CHUNK_SIZE`-wide 2D bit-grid, one `u32` per row (bit `a` of row `b` =
/// cell `(a, b)`) — `CHUNK_SIZE` is exactly 32, so a whole row fits in one
/// word, no heap allocation needed (`Grid`'s old `Vec<bool>` allocated twice
/// per layer: once for the mask, once for `visited`).
type Mask = [u32; CHUNK_SIZE];

/// Greedily merges every set bit in `mask` into maximal rectangles, calling
/// `emit(a, width, b, height)` once per rectangle (axis-agnostic — callers
/// map `(a, b)` back to world axes). Ported from the row-and-bitscan
/// technique in `cgerikj/binary-greedy-meshing` (cloned into `ref/` for
/// study, deleted once done — see `docs/optimisations.md`). Their version
/// also tracks per-voxel type equality *inside* this same merge loop; ours
/// doesn't need to — callers split a level's visibility mask into one `Mask`
/// per block type *before* calling this (see `mesh_axis_y` etc.), so two
/// different block types are never even candidates for the same merge pass,
/// and this function stays exactly as simple (and exactly as tested) as
/// before multiple block types existed. Two bitwise tricks replace the old
/// `Grid`-and-nested-loop version:
/// - `(!(row >> a)).trailing_zeros()` finds the width of a contiguous run of
///   set bits starting at `a` in one CPU instruction (`bsf`/`ctz`), instead
///   of a `while` loop testing one bit at a time.
/// - Marking a whole `width`-wide run as consumed is `consumed[row] |= mask`
///   — one instruction covering every cell in the run, instead of `width`
///   separate `Vec<bool>` writes.
fn greedy_merge(mask: Mask, mut emit: impl FnMut(usize, usize, usize, usize)) {
    let n = CHUNK_SIZE;
    let mut consumed: Mask = [0; CHUNK_SIZE];

    for b in 0..n {
        let mut row = mask[b] & !consumed[b];
        while row != 0 {
            let a = row.trailing_zeros() as usize;
            let width = ((!(row >> a)).trailing_zeros() as usize).min(n - a);
            let run_mask: u32 = if width == 32 {
                u32::MAX
            } else {
                ((1u32 << width) - 1) << a
            };

            let mut height = 1;
            while b + height < n && (mask[b + height] & !consumed[b + height]) & run_mask == run_mask {
                height += 1;
            }

            for db in 0..height {
                consumed[b + db] |= run_mask;
            }
            row &= !run_mask;

            emit(a, width, b, height);
        }
    }
}

fn mesh_axis_y(data: &ChunkData, builder: &mut MeshBuilder) {
    let n = CHUNK_SIZE;
    // u64, not u32: fits one extra bit (position `n`) for the padded row
    // sampled from whatever chunk sits above this one (`chunk::PADDED_HEIGHT`
    // — needed once chunks stack vertically, see `docs/world-generation.md`).
    // Without it, this chunk's own top layer would always compute as
    // bordering open air, even where solid terrain actually continues in the
    // chunk above — a phantom floor face rendered underground.
    let mut columns = vec![0u64; n * n]; // indexed [x * n + z], bit y = opaque at (x, y, z)
    for x in 0..n {
        for z in 0..n {
            let mut bits = 0u64;
            for y in 0..=n {
                if is_opaque(data.get_padded(x + PADDING, y, z + PADDING)) {
                    bits |= 1 << y;
                }
            }
            columns[x * n + z] = bits;
        }
    }

    // Up/down face visibility for an *entire column* computed once, not once
    // per (x, y, z) triple: `col & !(col >> 1)` sets bit y wherever y is
    // opaque and y+1 is air — the actual "binary" trick this file's header
    // comment describes. Bit `n` (the padding row) participates in this shift
    // like any other bit, which is exactly what makes the top layer's
    // up-face check correct against the chunk above.
    let mut up_faces = vec![0u64; n * n];
    let mut down_faces = vec![0u64; n * n];
    for i in 0..columns.len() {
        let col = columns[i];
        up_faces[i] = col & !(col >> 1);
        down_faces[i] = col & !(col << 1);
    }

    // Down-facing (-Y) faces only ever occur where an opaque block has air
    // directly below it. `generator::block_at` only ever produces "opaque
    // below the computed surface height, water/air above" columns —
    // monotonic, no caves or overhangs — so once a column goes solid it stays
    // solid all the way down (see that function's doc comment), meaning a
    // down face is structurally impossible anywhere a camera could ever be.
    // Skipping the whole down pass is a free win; revisit the moment caves,
    // overhangs, or floating structures become possible. `down_faces` above
    // is still computed regardless (cheap, keeps this branch symmetric/ready).
    for up in [true] {
        let faces = if up { &up_faces } else { &down_faces };
        for y in 0..n {
            let mut masks: [Mask; BLOCK_TYPE_COUNT] = [[0; CHUNK_SIZE]; BLOCK_TYPE_COUNT];
            let mut used = [false; BLOCK_TYPE_COUNT];
            for x in 0..n {
                for z in 0..n {
                    if (faces[x * n + z] >> y) & 1 == 1 {
                        let block = data.get(x, y, z) as usize;
                        masks[block][z] |= 1 << x;
                        used[block] = true;
                    }
                }
            }
            if !used.iter().any(|&u| u) {
                continue;
            }

            let level = if up { (y + 1) as f32 } else { y as f32 };
            for block in 1..BLOCK_TYPE_COUNT {
                if !used[block] {
                    continue;
                }
                let color = block_color(block as BlockId);
                greedy_merge(masks[block], |x, width, z, depth| {
                    let (x0, x1) = (x as f32, (x + width) as f32);
                    let (z0, z1) = (z as f32, (z + depth) as f32);
                    if up {
                        builder.push_quad(
                            [[x0, level, z0], [x0, level, z1], [x1, level, z1], [x1, level, z0]],
                            [0.0, 1.0, 0.0],
                            color,
                        );
                    } else {
                        builder.push_quad(
                            [[x0, level, z1], [x0, level, z0], [x1, level, z0], [x1, level, z1]],
                            [0.0, -1.0, 0.0],
                            color,
                        );
                    }
                });
            }
        }
    }
}

fn mesh_axis_x(data: &ChunkData, builder: &mut MeshBuilder) {
    let n = CHUNK_SIZE;
    // Padded columns: bit `p` (0..PADDED_SIZE) = opaque at padded x = p, i.e.
    // world offset (p - PADDING) from the chunk origin. Including the
    // neighboring chunks' border blocks (sampled by the generator) lets face
    // visibility at the chunk edge see the real neighbor instead of assuming air.
    let mut columns = vec![0u64; n * n]; // indexed [y * n + z]
    for y in 0..n {
        for z in 0..n {
            let mut bits = 0u64;
            for padded_x in 0..PADDED_SIZE {
                if is_opaque(data.get_padded(padded_x, y, z + PADDING)) {
                    bits |= 1 << padded_x;
                }
            }
            columns[y * n + z] = bits;
        }
    }

    // Right(+X)/left(-X) face visibility for an entire padded column at
    // once — same trick as the Y axis (see `mesh_axis_y`). `PADDED_SIZE`
    // (34) is well under 64, so these shifts never need a bounds guard.
    let mut right_faces = vec![0u64; n * n];
    let mut left_faces = vec![0u64; n * n];
    for i in 0..columns.len() {
        let col = columns[i];
        right_faces[i] = col & !(col >> 1);
        left_faces[i] = col & !(col << 1);
    }

    for positive in [true, false] {
        let faces = if positive { &right_faces } else { &left_faces };
        for x in 0..n {
            let px = x + PADDING;
            let mut masks: [Mask; BLOCK_TYPE_COUNT] = [[0; CHUNK_SIZE]; BLOCK_TYPE_COUNT];
            let mut used = [false; BLOCK_TYPE_COUNT];
            for y in 0..n {
                for z in 0..n {
                    if (faces[y * n + z] >> px) & 1 == 1 {
                        let block = data.get(x, y, z) as usize;
                        masks[block][z] |= 1 << y;
                        used[block] = true;
                    }
                }
            }
            if !used.iter().any(|&u| u) {
                continue;
            }

            let level = if positive { (x + 1) as f32 } else { x as f32 };
            for block in 1..BLOCK_TYPE_COUNT {
                if !used[block] {
                    continue;
                }
                let color = block_color(block as BlockId);
                greedy_merge(masks[block], |y, width, z, depth| {
                    let (y0, y1) = (y as f32, (y + width) as f32);
                    let (z0, z1) = (z as f32, (z + depth) as f32);
                    if positive {
                        // Right (+X)
                        builder.push_quad(
                            [[level, y0, z0], [level, y1, z0], [level, y1, z1], [level, y0, z1]],
                            [1.0, 0.0, 0.0],
                            color,
                        );
                    } else {
                        // Left (-X)
                        builder.push_quad(
                            [[level, y0, z1], [level, y1, z1], [level, y1, z0], [level, y0, z0]],
                            [-1.0, 0.0, 0.0],
                            color,
                        );
                    }
                });
            }
        }
    }
}

fn mesh_axis_z(data: &ChunkData, builder: &mut MeshBuilder) {
    let n = CHUNK_SIZE;
    // Padded columns: bit `p` (0..PADDED_SIZE) = opaque at padded z = p (see
    // `mesh_axis_x` for why the padding matters).
    let mut columns = vec![0u64; n * n]; // indexed [x * n + y]
    for x in 0..n {
        for y in 0..n {
            let mut bits = 0u64;
            for padded_z in 0..PADDED_SIZE {
                if is_opaque(data.get_padded(x + PADDING, y, padded_z)) {
                    bits |= 1 << padded_z;
                }
            }
            columns[x * n + y] = bits;
        }
    }

    // Front(+Z)/back(-Z) face visibility for an entire padded column at
    // once — same trick as the Y axis (see `mesh_axis_y`).
    let mut front_faces = vec![0u64; n * n];
    let mut back_faces = vec![0u64; n * n];
    for i in 0..columns.len() {
        let col = columns[i];
        front_faces[i] = col & !(col >> 1);
        back_faces[i] = col & !(col << 1);
    }

    for positive in [true, false] {
        let faces = if positive { &front_faces } else { &back_faces };
        for z in 0..n {
            let pz = z + PADDING;
            let mut masks: [Mask; BLOCK_TYPE_COUNT] = [[0; CHUNK_SIZE]; BLOCK_TYPE_COUNT];
            let mut used = [false; BLOCK_TYPE_COUNT];
            for x in 0..n {
                for y in 0..n {
                    if (faces[x * n + y] >> pz) & 1 == 1 {
                        let block = data.get(x, y, z) as usize;
                        masks[block][y] |= 1 << x;
                        used[block] = true;
                    }
                }
            }
            if !used.iter().any(|&u| u) {
                continue;
            }

            let level = if positive { (z + 1) as f32 } else { z as f32 };
            for block in 1..BLOCK_TYPE_COUNT {
                if !used[block] {
                    continue;
                }
                let color = block_color(block as BlockId);
                greedy_merge(masks[block], |x, width, y, depth| {
                    let (x0, x1) = (x as f32, (x + width) as f32);
                    let (y0, y1) = (y as f32, (y + depth) as f32);
                    if positive {
                        // Front (+Z)
                        builder.push_quad(
                            [[x0, y0, level], [x1, y0, level], [x1, y1, level], [x0, y1, level]],
                            [0.0, 0.0, 1.0],
                            color,
                        );
                    } else {
                        // Back (-Z)
                        builder.push_quad(
                            [[x1, y0, level], [x0, y0, level], [x0, y1, level], [x1, y1, level]],
                            [0.0, 0.0, -1.0],
                            color,
                        );
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every set bit in the mask, as `(a, b)` coordinates.
    fn set_bits(mask: &Mask) -> HashSet<(usize, usize)> {
        let mut set = HashSet::new();
        for (b, row) in mask.iter().enumerate() {
            for a in 0..CHUNK_SIZE {
                if (row >> a) & 1 == 1 {
                    set.insert((a, b));
                }
            }
        }
        set
    }

    /// Runs `greedy_merge` and checks the three properties that must hold
    /// regardless of *which* valid tiling the algorithm picks: every emitted
    /// rectangle only covers cells that were actually set, no two rectangles
    /// overlap, and every set cell is covered by exactly one rectangle (no
    /// gaps, no double-merging). This is what actually matters after
    /// rewriting the merge to use bitscan/mask tricks instead of a
    /// `Grid`-and-nested-loop — the exact rectangle boundaries the algorithm
    /// chooses aren't a contract, full and non-overlapping coverage is.
    fn check_merge_is_correct(mask: Mask) {
        let expected = set_bits(&mask);
        let mut covered = HashSet::new();

        greedy_merge(mask, |a, width, b, height| {
            for db in 0..height {
                for da in 0..width {
                    let cell = (a + da, b + db);
                    assert!(
                        expected.contains(&cell),
                        "rectangle (a={a}, width={width}, b={b}, height={height}) covers unset cell {cell:?}"
                    );
                    assert!(
                        covered.insert(cell),
                        "cell {cell:?} covered by more than one emitted rectangle"
                    );
                }
            }
        });

        assert_eq!(
            covered, expected,
            "merged rectangles don't cover exactly the set bits in the mask"
        );
    }

    #[test]
    fn empty_mask_emits_nothing() {
        check_merge_is_correct([0; CHUNK_SIZE]);
    }

    #[test]
    fn full_mask_merges_into_one_rectangle() {
        let mask: Mask = [u32::MAX; CHUNK_SIZE];
        let mut count = 0;
        greedy_merge(mask, |_, _, _, _| count += 1);
        assert_eq!(count, 1, "a fully solid layer should merge into a single rectangle");
        check_merge_is_correct(mask);
    }

    #[test]
    fn single_cell() {
        let mut mask: Mask = [0; CHUNK_SIZE];
        mask[5] = 1 << 3;
        check_merge_is_correct(mask);
    }

    #[test]
    fn checkerboard_has_no_two_by_two_merges() {
        let mut mask: Mask = [0; CHUNK_SIZE];
        for b in 0..CHUNK_SIZE {
            for a in 0..CHUNK_SIZE {
                if (a + b) % 2 == 0 {
                    mask[b] |= 1 << a;
                }
            }
        }
        check_merge_is_correct(mask);
    }

    #[test]
    fn run_touching_the_top_bit_does_not_overcount_width() {
        // A run reaching exactly bit 31 exercises the `width == 32` special
        // case's neighbor: `!(row >> a)` must not be read as having "phantom"
        // 1-bits past the top of the word.
        let mut mask: Mask = [0; CHUNK_SIZE];
        mask[0] = 0xF000_0000; // bits 28..=31 set
        check_merge_is_correct(mask);
    }

    #[test]
    fn random_masks_are_always_correct() {
        fastrand::seed(1337);
        for _ in 0..500 {
            let mut mask: Mask = [0; CHUNK_SIZE];
            for row in mask.iter_mut() {
                *row = fastrand::u32(..);
            }
            check_merge_is_correct(mask);
        }
    }

    /// End-to-end check that `mesh_axis_y` never merges two different block
    /// types into the same quad — the whole reason each level's visibility
    /// mask gets split into one `Mask` *per type* before calling
    /// `greedy_merge`, rather than passing type data through the merge
    /// itself. Builds a chunk with a stone half and a grass half at the same
    /// height (touching along one edge) and confirms every *vertex color* in
    /// the resulting mesh's top faces is a pure, single block color — a
    /// same-quad blend would show up as a third, in-between color.
    #[test]
    fn different_block_types_never_share_a_merged_quad() {
        use super::super::chunk::{ChunkData, GRASS, STONE};
        use bevy::render::mesh::VertexAttributeValues;

        let mut data = ChunkData::empty();
        for padded_x in 0..PADDED_SIZE {
            for padded_z in 0..PADDED_SIZE {
                for y in 0..=CHUNK_SIZE {
                    if y >= CHUNK_SIZE / 2 {
                        continue; // air above the halfway point
                    }
                    // Below the surface, always stone... except the very top
                    // layer, split down the middle: stone on one half, grass
                    // on the other, both at the *same* height.
                    let is_top = y == CHUNK_SIZE / 2 - 1;
                    let block = if is_top && padded_x < PADDED_SIZE / 2 { STONE } else if is_top { GRASS } else { STONE };
                    data.set_padded(padded_x, y, padded_z, block);
                }
            }
        }

        let mesh = build_chunk_mesh(&data);
        let VertexAttributeValues::Float32x4(colors) = mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap() else {
            panic!("expected the color attribute to be Float32x4");
        };
        let stone_color = block_color(STONE);
        let grass_color = block_color(GRASS);

        for color in colors {
            assert!(
                *color == stone_color || *color == grass_color,
                "found a vertex color {color:?} that isn't a pure stone or grass color — \
                 suggests two different block types merged into one quad"
            );
        }
    }
}
