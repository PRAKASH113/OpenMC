# World Generation

> Fully reconnected and back at `RENDER_DISTANCE = 4` (a 9x9 grid) after the single-chunk test in `docs/performance-investigation.md` confirmed healthy.

## Why this replaced the placeholder ground

The original `src/playing/ground.rs` spawned a 32x32 grid of individually-colored unit-cube blocks — 1024 separate entities. A direct investigation (temporary debug system, since removed) showed this was the dominant cause of a reported 7 FPS / 1455-entity reading: `mesh3d=1026` out of `total=1455`, with 1024 of those being the loose ground blocks (confirmed exactly by triangle count: 1024 x 12 tris = 12288, matching the HUD's 12320 almost exactly once the sky dome and sun cube are added). 1024 separate entities each going through per-frame transform propagation, visibility computation, and individual render extraction is expensive for what's visually "just a flat platform."

## Non-negotiable design rule: one entity per chunk

Every one of a chunk's up to 32,768 blocks is merged into a **single** `Mesh`, attached to **exactly one** entity. There is no per-block spawning anywhere in this pipeline:
- `generator.rs` produces a plain data buffer (`ChunkData`) — no entities.
- `mesher.rs` produces a `Mesh` value — no entities.
- `manager.rs` is the only place an `Entity` is spawned, and it spawns exactly one per chunk (`Mesh3d` + `MeshMaterial3d`, tagged `PlayingScreen` so it's cleaned up like everything else in the Playing scene on `OnExit`).

At `RENDER_DISTANCE = 4` (a 9x9 grid of chunk columns), that's ~81 chunk entities instead of the 1024+ per-block entities the old ground had for a much smaller area.

## `GridPos`: a general 3D world grid coordinate (not the same thing as `ChunkPos`)

`chunk::GridPos` divides world space into `CHUNK_SIZE`-sized cells on *every* axis, including Y — unlike `ChunkPos`, which is horizontal-only and specifically tied to what gets rendered. Today `GridPos` is purely an informational readout: the performance HUD can show which cell the player currently occupies (Grid X/Y/Z) and where within that cell (Local X/Y/Z, `GridPos::local_offset`, always `0..CHUNK_SIZE`) — currently hidden from the HUD (confirmed working, kept for later) but still computed every frame, see `docs/performance.md`. A wireframe box around the current cell is also drawn via `Gizmos` (immediate-mode, redrawn every frame — no entity to spawn/despawn as the player crosses cell boundaries). It's the seed of a more general spatial grid — e.g. a future simulation distance that covers a wider radius than what's rendered — not something wired into chunk loading itself yet.

Related but distinct: `chunk::ChunkTile(ChunkPos)`, a marker component carrying a chunk *entity's own position*. It exists so other modules (the per-chunk triangle HUD in `dev_tools`, see `docs/performance.md`) can correlate a chunk entity back to its position and query its `ViewVisibility` without needing access to `ChunkManager`'s private `loaded` map — an entity should carry its own identity rather than requiring callers to reach into another module's bookkeeping.

The debug wireframe box that used to render around the player's current grid cell (via `Gizmos`) has been removed — it was confirming the same thing the Grid X/Y/Z HUD readout already confirms, and the constant on-screen box was more clutter than signal once that was established.

## Scope (confirmed with the user before building)

- **Horizontal only, for now.** `RENDER_DISTANCE` (in `config::world`) governs a 9x9 grid of chunk *columns* around the player (X/Z) — a single vertical layer, not a full 3D cube of chunks. Vertical chunk stacking is explicit future work.
- **`SIMULATION_DISTANCE`** is reserved for later (tick/simulate a wider radius than what's rendered) and is not used yet — but a compile-time assertion (`const _: () = assert!(SIMULATION_DISTANCE >= RENDER_DISTANCE)`) enforces the constraint structurally the moment it's touched, rather than relying on convention.
- **Single block type** (air/solid) — enough to prove the pipeline end-to-end. Grass/dirt/stone layering is a natural, small follow-up.
- **No async/threaded generation** — a per-frame spawn budget (`CHUNKS_PER_FRAME` in `manager.rs`) keeps the initial load-in from hitching in a single frame, but generation + meshing still run on the main thread.

## Cross-chunk face culling (padded chunk data)

The first version of the mesher treated "outside my own chunk's data" as air, so a solid block sitting right at a chunk's edge always got a face rendered there — even when the neighboring chunk (whether loaded yet or not) is solid at that exact spot too. Fixed by padding `ChunkData` by one block on every horizontal side (`chunk::PADDING`/`PADDED_SIZE`): `generator.rs` fills that border by sampling the *same* heightmap function at the neighboring world coordinates (it's a pure function of world position, so this needs no coordination with whether the neighboring chunk entity actually exists yet), and the X/Z mesher passes read `u64` column bitmasks over the full padded range so the boundary layers see real neighbor data instead of an assumed "air" — same bitwise trick as the interior, just shifted by the 1-block padding offset. The Y axis is untouched: there's no vertical chunking yet, so "off the top/bottom of the world" genuinely is air, not just "unknown."

## Pipeline

`src/playing/world/`:

1. **`manager.rs`** (`ChunkManager` resource + `update_chunks` system): reads the `PlayerCamera` entity's `Transform`, computes its current `ChunkPos`, and — only when that chunk coordinate actually changes (not every frame) — recomputes the desired 9x9 chunk set, despawns anything no longer desired, and queues anything newly desired. A small budget (`CHUNKS_PER_FRAME`) of queued chunks gets generated + meshed + spawned each frame, spreading the work out instead of stalling on a big jump.
2. **`generator.rs`** (`generate_chunk`): a layered (`noise::Fbm<Perlin>`, fractal Brownian motion) heightmap per (x, z) column — solid below the sampled surface height, air above. `MAX_SURFACE_HEIGHT` (the highest a column can ever reach) is exported and used by `playing/mod.rs` to place the initial camera spawn comfortably above any possible terrain, so the camera never spawns buried in the ground.

   A single Perlin octave (the original approach) is one smooth wave at one frequency — scaling it up or down just produces a bigger or smaller copy of the *same* shape, which is why the terrain looked repetitive ("every hill is the same") once quantized. `Fbm` layers several octaves of increasingly fine, lower-amplitude noise on top of each other, which is what actually varies the local *shape* of the terrain instead of just its scale. `NOISE_OCTAVES` was tuned down from 5 to **3**: at `TERRAIN_SCALE = 0.02` with `NOISE_LACUNARITY = 2.0`, 5 octaves pushed the finest wavelength down to ~3 blocks — close enough to single-column noise that neighboring columns could differ wildly, producing isolated one-column "spike" pillars. 3 octaves keeps the finest wavelength around 12 blocks, so height changes stay gradual between neighbors.
3. **`mesher.rs`** (`build_chunk_mesh`, the binary greedy mesher): see below.
4. **`chunk.rs`**: `ChunkPos` (chunk-grid coordinate) and `ChunkData` (the flat `CHUNK_SIZE`^3 block buffer) — pure data types, no Bevy entities involved.

## The binary greedy mesher

Two optimizations, both requested explicitly, layered on top of each other:

1. **Face culling**: never emit a face directly touching another solid block's face — only faces bordering air (or the chunk boundary, treated as air for now).
2. **Binary greedy meshing**: merge coplanar same-type faces into the fewest possible rectangles instead of one quad per block face.

`CHUNK_SIZE = 32` is not a coincidence — a 32-tall column of solid/air bits packs exactly into one `u32` (the X/Z passes use `u64` instead, to also fit the 1-block padding on each side — see below). For each of the 3 axes, blocks are packed into per-column bitmasks (e.g. for the Y axis, one `u32` per (x, z) column, bit `y` set if that block is solid). Face visibility for an *entire column* is then a couple of bitwise ops instead of per-block comparisons: `column & !(column >> 1)` finds every "solid here, air directly above" bit at once (up faces). This is the "binary" in binary greedy meshing.

The down-facing pass is skipped entirely (not just culled — never run): the generator only produces "solid below the surface height" columns with no caves, so a down face (solid with air below) can only ever occur at y=0, the world floor, which is never visible in normal play. Free triangle-count win, see `docs/performance.md`.

The per-layer 2D result (a grid of "does this cell have a visible face") is then greedily merged into maximal rectangles (`greedy_merge` in `mesher.rs`) — same algorithm shared across all face directions via a small `Grid` helper, since the merge logic itself doesn't care which world axis it's operating on; only the per-axis column-building and final quad vertex layout differ (documented per-face vertex winding in `mesher.rs` directly, since getting winding backwards silently makes faces invisible from the "should be visible" side rather than erroring).

**Fundamental limit of this technique**: two faces can only merge into one quad if they sit at the *exact same* height — a quad is one flat plane, so no amount of algorithmic cleverness merges faces at different Y coordinates. An earlier version tried to force more merges by *quantizing* the surface height to steps of 2+ blocks (so more neighboring columns coincidentally landed on the same height) — this backfired visually: quantizing to steps of N guarantees the *smallest possible* height difference between two neighboring columns is N blocks, so single-column features (a ridge, an edge) always looked like N blocks stacked instead of a normal one-block voxel step. A voxel game's terrain is *supposed* to have plain single-block steps — that's not a flaw to engineer away. `QUANTIZE_STEP` is back to `1` (plain rounding, no artificial widening); the real lever for merge-friendliness is keeping the noise itself smooth (`NOISE_OCTAVES` above) so neighboring columns coincidentally share a height often, rather than forcing it. See `docs/performance.md` and `docs/performance-investigation.md`.

## Chunk-switch hysteresis (a real bug, not just an optimization)

`manager.rs`'s "has the player's chunk changed" check originally compared `ChunkPos::from_world_pos(pos)` directly against the last known chunk every frame. Near an exact chunk boundary, ordinary floating-point movement noise is enough to flip the computed `ChunkPos` back and forth every single frame — and *each flip re-evaluates the entire 9x9 render window*, despawning and regenerating most of the chunks along the shifted edge. This is what caused a severe, sustained FPS drop specifically when moving near/looking closely at terrain (exactly where a player's position is likely to sit near a chunk boundary), and why it seemed to get worse "the longer you stayed" — the thrashing just kept going for as long as the position kept flickering across the line. Fixed with hysteresis (`resolve_player_chunk` in `manager.rs`): once a chunk is confirmed, the player must move `SWITCH_MARGIN` blocks *past* its edge — not just barely across it — before a switch is recognized.

## Debug visualization (requested explicitly, temporary)

- Every solid block is a single uniform debug color (green, via per-vertex `Mesh::ATTRIBUTE_COLOR`) — confirms the *generator* independent of any real per-block-type coloring work.
- Chunk entities can render with a `Wireframe` component (`bevy::pbr::wireframe`) so actual triangle edges show on screen. **Fewer, larger flat quads with few internal edges = merging is working. A fine grid of tiny per-block squares = it isn't.** This was the user's own suggested verification method, precisely because it makes greedy-merge correctness or incorrectness immediately visible rather than needing to trust the algorithm blind. Originally always-on; now off by default and toggled with **Shift+1** (`input::playing::toggle_terrain_wireframe`), which adds/removes `Wireframe` on every currently-loaded `ChunkTile` entity and flips the `ChunkWireframe` resource so newly-streamed-in chunks (`world::manager::load_pending_chunks`) match whatever state was last toggled. Bare `Digit1` (the Loading-state jump in `input::switch_state`) explicitly excludes the case where Shift is held, so the two don't collide.
- `manager.rs::DEBUG_FIXED_CHUNK_GRID` (`pub(crate)`, same "flip the const, test, flip back" pattern as the dev-tools debug consts): when true, `ChunkManager` loads exactly a fixed 2x2 grid at the origin once and never streams based on player position again — no matter how far the player flies, nothing new loads or unloads. Used earlier to verify GPU frustum/occlusion culling in isolation (confirmed working). Back to **`false`** now that the real streaming manager is doing that job again; kept around, still fully wired up, in case culling ever needs isolating the same way later. Chunk *loading* itself is deliberately **not** view-direction-based — it's a radius around the player regardless of which way they're facing, so turning around doesn't cause visible pop-in — only *rendering* the already-loaded chunks is where frustum/occlusion culling applies.

Both are meant to be replaced once the pipeline is confirmed correct: uniform green → real per-block-type colors, and the wireframe is a verification aid, not a permanent visual style.
