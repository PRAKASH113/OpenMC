# Performance Investigation: Baseline FPS Crash

## Why this exists

Even after fixing chunk-switch hysteresis, adding GPU occlusion culling, and confirming (via `DEBUG_FIXED_CHUNK_GRID`, see `docs/world-generation.md`) that frustum culling itself works correctly, FPS still behaved wrong: starting around 60 FPS on launch, then crashing to 20-30 FPS within about 3 seconds — even with only 4 chunks loaded and a single ~800-triangle mesh actually visible. That's nowhere near enough geometry to explain a crash like that on reasonable hardware, which means the terrain/meshing system was never the real cause of *this specific* symptom — something else in the engine setup is.

Rather than keep guessing at a moving target (chunked terrain, sky dome, wireframe toggles, and the diagnostics HUD were all changing at once across the previous rounds of work), the scene was deliberately stripped back to the smallest possible baseline so the FPS-crash bug can be isolated with nothing else changing at the same time.

## Current state of the Playing scene (temporary, deliberate)

`src/playing/mod.rs` now spawns exactly three things, tagged `PlayingScreen`:

- The free-fly camera (`PlayerCamera`, unchanged input/look/movement).
- One plain test cube (`Cuboid::new(2.0, 2.0, 2.0)`, flat gray `StandardMaterial`) at the origin.
- The sun (`sun::spawn_sun` — a `DirectionalLight` plus an emissive cube that's re-centered on the camera every frame), unchanged except shadows are back on (see below).

**Disconnected, not deleted**: `src/playing/world/` (chunk manager, generator, binary greedy mesher — the whole chunked terrain pipeline) and `src/playing/sky.rs` (the procedural sky dome shader) still exist on disk exactly as they were, but `playing/mod.rs` no longer has `mod world;` / `mod sky;` at all, so neither compiles into the binary right now. This is intentional: that code represents real, previously-verified-working effort, and the plan is to bring it back deliberately, one piece at a time, once the baseline crash is understood — not to redo that work from scratch. `docs/world-generation.md` and `docs/optimisations.md` still describe that system accurately; they just describe code that isn't currently wired into the running game.

**Optimizations removed, not just the terrain**: the user asked for a true unoptimized baseline, so these are also gone from `playing/mod.rs`'s camera for now: `Hdr`, `Tonemapping`, `Bloom`, `DepthPrepass`, `OcclusionCulling`. The sun's `DirectionalLight` also had its `shadow_maps_enabled: false` override removed (`src/playing/sun.rs`), so shadows now run at Bevy's default cost too. The `WireframePlugin`/Shift+1 terrain-wireframe toggle (`input::playing::toggle_terrain_wireframe`) was removed along with the terrain it toggled.

**Diagnostics HUD reduced to one window** (`src/dev_tools/mod.rs`): FPS, frame time, triangle count, a game-entity count, process CPU/memory. The old two-window setup (a "Chunks" window with per-chunk visibility breakdown, `GridPos` coordinate readout, `DEBUG_FIXED_CHUNK_GRID`-aware labeling) doesn't apply to a scene with no chunks — all of that lived in `dev_tools` reading types from the now-disconnected `world` module, so it had to go too. It'll come back if/when chunked terrain does.

### The entity-count fix

Bevy's built-in `EntityCountDiagnosticsPlugin` counts *every* entity, and `docs/performance.md` already documented why that number was never trustworthy: ~320 `IsResource` entities and ~82 `Observer` entities are pure Bevy-internal bookkeeping with zero render-path involvement, dwarfing the handful of entities actually in the scene. Rather than keep explaining that gap away, `dev_tools` now computes its own `game_entity_count` diagnostic from `Query<Entity, With<PlayingScreen>>` — literally "how many things did *we* spawn for this scene." With the current scene that should read a small, fixed number (camera + sun light + sun cube + test cube) that doesn't move regardless of Bevy's internal bookkeeping.

### GPU usage — not included

Bevy 0.19 has no built-in cross-platform "GPU utilization %" diagnostic (nothing like Task Manager's GPU graph). The closest built-in mechanism, `bevy_render::diagnostic::RenderDiagnosticsPlugin`, reports GPU *frame time* per render pass via timestamp queries — a different metric (time, not %), not guaranteed to be supported on every GPU/backend, and enough extra plumbing (async readback, per-pass spans) that bolting it on right now would work against the goal of a minimal, easy-to-trust baseline. Left out rather than half-implemented; worth reconsidering once the crash itself is understood.

## What to test

With this baseline, `cargo run` (dev profile, already has the `[profile.dev]` opt-level override from the last round — see `docs/performance.md`) and watch the single "Performance" HUD:

- Does `game_entity_count` stay flat (a handful) the whole time? If not, something is spawning entities it shouldn't be — that alone could explain a progressive slowdown.
- Does `triangle_count` stay flat (well under a thousand — just the cube and sun cube)? If it climbs over time, something is generating geometry nothing asked for.
- Does the FPS crash (60 → 20-30 within ~3 seconds) still happen with *none* of the terrain/optimization code in the build at all? If yes, the cause is somewhere in the always-on parts of this minimal scene (camera setup, sun, input, or Bevy/engine-level configuration) — not anything removed above. If the crash disappears, the next step is reintroducing the removed pieces one at a time (sky dome, then chunked terrain, then GPU occlusion culling, etc.) to find which one brings the crash back.

## Reintroduction plan (once the crash is understood)

Add things back one at a time, retesting FPS after each, in roughly this order (cheapest/most-isolated first): sky dome → chunked terrain (one entity per chunk, face culling, greedy meshing) → GPU occlusion culling → Bloom/HDR/tonemapping → the Shift+1 wireframe toggle → the two-window diagnostics HUD. `docs/optimisations.md`'s roadmap (packed vertex format, instancing, etc.) resumes after that, once there's a trustworthy baseline to measure improvements against.

## Round 2: 45 FPS average (60-80 peaks) on a cube + sun — ruled out and still open

With the scene reduced to a camera, one test cube, and the sun, FPS still only averaged ~45 with peaks of 60-80 — far too low for that little geometry on capable hardware. Two things were checked and clarified as **not bugs**:

- `triangle_count` reading 24 already includes the sun: a Bevy `Cuboid` is 12 triangles, and the sun is *rendered* as a second cuboid (`sun::spawn_sun`'s emissive `SunCube`) on top of the actual `DirectionalLight` (which has zero triangles — it's a light, not a mesh). Test cube (12) + sun cube (12) = 24.
- `game_entity_count` reading 4 is correct: camera + test cube + the sun's `DirectionalLight` + the sun's separate `SunCube` mesh. The sun has always been two entities (light + visible cube), not one — see `docs/sky-and-sun.md`. The HUD's own `DiagnosticsOverlay` entities aren't tagged `PlayingScreen` and are correctly excluded.

**Ruled out by reading the source directly**: `SystemInformationDiagnosticsPlugin` (`bevy_diagnostic::system_information_diagnostics_plugin`) runs its `sysinfo` refresh in a background `AsyncComputeTaskPool` task, throttled by `sysinfo::MINIMUM_CPU_UPDATE_INTERVAL` — it does not block the main thread every frame, so it's very unlikely to be the source of frame-time variance.

**Resolved**: both remaining hypotheses were confirmed and closed.

- **GPU selection was correct all along.** The startup log's `AdapterInfo { name: "NVIDIA GeForce RTX 3050 Laptop GPU", device_type: DiscreteGpu, backend: Vulkan }` plus Task Manager showing load on GPU 1 (the discrete NVIDIA GPU, not the integrated Radeon) confirms Bevy's `PowerPreference::HighPerformance` default picked the right adapter. Not the cause.
- **VSync/present-mode pacing *was* the cause.** With `PresentMode::AutoNoVsync`, FPS now goes above 60 — the 45-average/60-80-peak pattern under `Fifo` was vsync/compositor frame-pacing behavior, not a real CPU or GPU bottleneck. The baseline (camera + cube + sun, no optimizations) is healthy.

**Decision**: keep `PresentMode::AutoNoVsync` for the remainder of this optimization round — `Fifo` would cap/mask the exact numbers needed to measure each reintroduced feature's real impact. Switch back to `Fifo` (or a nicer vsync mode) once optimization work is done and this becomes a normal-play build again, not a benchmarking one.

**Also confirmed while answering follow-up questions**: Bevy's default mesh pipeline already sets `cull_mode: Some(Face::Back)` unconditionally (`bevy_pbr`'s `MeshPipeline`, verified directly in source) — standard GPU backface culling has been active this whole time for every mesh except `SkyMaterial`, which deliberately disables it since the camera sits inside that sphere. There is no backface-culling work to do for a lone convex cube; it's a stock, always-on Bevy/wgpu feature, not something this project is missing. The real per-voxel technique (skip faces where one solid voxel touches another, since that geometry can never be seen from any angle) is different from backface culling and already lives in the disconnected `src/playing/world/mesher.rs` binary greedy mesher — next up for reintroduction.

## Round 3: sky dome reconnected

First step of the reintroduction plan: `playing/mod.rs` has `mod sky;` back, `MaterialPlugin::<SkyMaterial>::default()` registered, the dome spawned (tagged `PlayingScreen`, re-centered on the camera every frame alongside the sun), and `SkyMaterial`/`sky.wgsl` untouched from before. Expected HUD deltas going into this test, since both diagnostics are computed generically (every `Mesh3d`/`PlayingScreen` entity, not a chunk-specific special case):

- `triangle_count`: +~20 (the dome is a level-0 icosphere — see `docs/sky-and-sun.md` for why so low-poly is enough).
- `game_entity_count`: +1 (the `SkyDome` entity).
- FPS: watch for any drop from the Round 2 baseline. `SkyMaterial` already disables its own shadow pass and prepass (`enable_shadows() -> false`, `enable_prepass() -> false`), so a meaningful FPS hit here would be a real, specific finding — not expected given how cheap this mesh/shader is.

## Round 4: single chunk reconnected

Next step: the full chunk manager → generator → mesher → GPU pipeline in `src/playing/world/` (untouched since it was disconnected) is back, deliberately scoped to **exactly one chunk** rather than the normal 9x9:

- `config::world::RENDER_DISTANCE` and `SIMULATION_DISTANCE` are temporarily `0` instead of `4` — the desired-chunk loop (`-RENDER_DISTANCE..=RENDER_DISTANCE`) then only ever produces one `ChunkPos` (whichever chunk the player is currently standing in). This reuses the real streaming manager as-is (hysteresis, budgeting, despawn-on-move all still active and exercised) rather than a special-cased demo — as the player flies, the single loaded chunk follows them, which doubles as an ongoing check that chunk-switching still behaves.
- `manager::DEBUG_FIXED_CHUNK_GRID` is back to `false`: it already did its job confirming culling works, and the real manager takes over that job now.
- The placeholder test cube in `playing/mod.rs` was removed (superseded — the chunk is the thing to look at now); the camera spawns centered above the chunk (`x=16, z=16`, matching the original terrain-based spawn logic) and comfortably above `MAX_SURFACE_HEIGHT`, looking down.

Expected HUD deltas: `game_entity_count` +1 (one chunk entity), `triangle_count` up by whatever the greedy mesher produces for one chunk of generated terrain (expect well under a thousand — nowhere near the old 12k+/40k+ readings from before optimization, since face culling + greedy merging + height quantization are all still exactly as documented in `docs/world-generation.md`, untouched by any of this). Watch FPS for any drop from Round 3's baseline — if one small chunk tanks it, that's a specific, isolated finding pointing at the generation/meshing/spawn path itself rather than anything about scale. Once this looks healthy, raise `RENDER_DISTANCE` back up (gradually, not straight to 4) to find out at what scale, if any, problems reappear.

**Result**: `game_entity_count: 5` (camera + sky dome + sun light + sun cube + chunk — exactly as expected), `triangle_count: 748` for one chunk — face culling and greedy meshing both clearly working (a raw 32x32 column grid with no culling/merging would be far higher). FPS read ~30 with a single trivial chunk, lower than hoped but not flagged as the priority this round — visual issues were: terrain shape looked wrong (see Round 5) and lighting was harsh (see Round 5). Worth re-measuring FPS once those are fixed, since a bad-looking scene is a distraction from judging performance by eye anyway.

## Round 5: terrain quality, lighting, and back to full render distance

Three issues reported from the Round 4 screenshot, all fixed:

- **Terrain looked like "two cubes stacked," not natural blocky terrain.** Root cause: `QUANTIZE_STEP = 2` (from the very first terrain-quality pass) guarantees the *smallest possible* height difference between neighboring columns is 2 blocks — single-column features (edges, ridges) always came out looking like two blocks stacked instead of a normal one-block voxel step. Fixed by setting `QUANTIZE_STEP = 1` (plain rounding, no artificial widening) in `generator.rs` — a voxel game is *supposed* to have plain single-block steps. Also reduced `NOISE_OCTAVES` from 5 to 3: at `TERRAIN_SCALE = 0.02` and `NOISE_LACUNARITY = 2.0`, 5 octaves pushed the finest noise wavelength down to ~3 blocks, close enough to single-column noise to produce isolated one-column "spike" pillars regardless of quantization. 3 octaves keeps the finest wavelength around 12 blocks — height changes stay gradual between neighbors. See `docs/world-generation.md`.
- **Lighting looked harsh — faces not facing the sun read as near-black.** The sun (`DirectionalLight`) was the *only* light source; Bevy's default ambient (`GlobalAmbientLight`, `brightness: 80.0`) is tiny next to the sun's `illuminance: 6000.0`. Bumped to `AMBIENT_BRIGHTNESS = 300.0` in `playing/mod.rs` (`GlobalAmbientLight`, inserted in `PlayingPlugin::build`) — a flat, cheap stand-in for sky-scattered light, not real bounced lighting, tuned by eye. See `docs/sky-and-sun.md`.
- **Render distance raised back to normal**: `config::world::RENDER_DISTANCE`/`SIMULATION_DISTANCE` back to `4` (9x9 chunk grid) now that a single chunk's pipeline and stats looked correct.

Also restored **Shift+1** (`input::playing::toggle_terrain_wireframe`) — removed during the minimal-scene rebuild, now that there's real terrain worth inspecting again. Same mechanism as before: adds/removes `Wireframe` on every loaded `ChunkTile` entity and flips `ChunkWireframe` so newly-streamed chunks match.

**Mesh optimizations confirmed in place, for the record** (asked directly this round): **face culling** (never emit a face touching another solid block) and **binary greedy meshing** (merge coplanar faces into maximal rectangles) are both implemented and active — see "The binary greedy mesher" in `docs/world-generation.md`. The 748-triangle reading for one chunk in Round 4 is itself evidence both are working: a raw, uncalled/unmerged 32x32-column chunk would be dramatically higher. Not yet done: the vertex-packing/instancing/indirect-draw items in `docs/optimisations.md` — those were deliberately paused for this whole reintroduction effort and pick back up once render-distance scaling is confirmed healthy.

Next: rebuild at `RENDER_DISTANCE = 4`, confirm the terrain/lighting fixes look right, check the wireframe toggle still works, and report the new stats (especially FPS at full render distance — this is the real test of whether the earlier crash-to-20s-FPS symptom is actually gone at scale, not just at 1 chunk).

## Round 6: performance confirmed healthy at full render distance; harsh shadows and render-distance pop-in fixed

Performance at `RENDER_DISTANCE = 4` reported as good — the crash-to-20s-FPS symptom from the very start of this investigation is confirmed gone at real scale, not just at 1 chunk. Two small polish items tackled next, both deliberately scoped small ("fix these, then add other things"):

- **Shadows still too harsh.** `AMBIENT_BRIGHTNESS = 300.0` from Round 5 wasn't enough — raised to `1200.0` (roughly a fifth of the sun's `illuminance: 6000.0`). See `docs/sky-and-sun.md`.
- **Chunks popping into/out of existence at the render-distance edge.** Added `DistanceFog` (`bevy::pbr`) to the camera: `FogFalloff::Linear` from `FOG_START` to `FOG_END`, where `FOG_END` is derived directly from `config::world::RENDER_DISTANCE * CHUNK_SIZE` — the fog boundary always tracks the actual chunk-loading boundary, not a separately-tuned magic number that could drift out of sync if render distance changes later. Fog color reads `SkyUniform::default().horizon_color` directly (not a duplicated literal) so distant chunks fade into the sky rather than an arbitrary gray. See `docs/sky-and-sun.md`.

Both are tuned-by-eye values (ambient brightness, fog start/end ratio) — expect another pass if they don't look right in practice. Next up (explicitly deferred by the user until these two were confirmed): more small polish items, one at a time, same as this round.

## Round 7: fog broke the sun; terrain looked glossy

Two issues from testing Round 6's fog/lighting changes:

- **The sun turned gray** — correctly diagnosed by the user as a fog side effect. `DistanceFog` fully saturates beyond `FOG_END` (128 units), but the sun cube sits `SUN_DISTANCE` (400 units) away by design, so it was rendering as 100% fog color instead of its own emissive color. Fixed with `fog_enabled: false` on the sun cube's material (`src/playing/sun.rs`) — it's supposed to read as infinitely distant, not part of the local atmosphere. See `docs/sky-and-sun.md`.
- **Terrain looked "glossy"** — a specular highlight sliding across the ground as the camera moved, on top of the already-known harsh flat-shading look. `StandardMaterial`'s defaults (`perceptual_roughness: 0.5`, `reflectance: 0.5`) are a semi-glossy "plastic" preset, wrong for flat voxel terrain. Set to `perceptual_roughness: 1.0, reflectance: 0.0` (`world::manager`'s shared chunk material) — fully matte, no specular term.

**Not fixed yet, flagged explicitly**: the underlying "harsh, hard-edged lighting" look (sharp brightness jumps between adjacent faces, no soft gradient) is inherent to flat per-face normals under one hard directional light, not something the roughness/reflectance tweak addresses. A real fix is a bigger stylistic choice — e.g. Minecraft-style fixed per-face-direction brightness multipliers (independent of actual sun angle) instead of true directional lighting, and/or baked ambient occlusion at block edges/corners — deferred as a separate "later" item per the user's own pacing ("fix these things then later we will add other things").
