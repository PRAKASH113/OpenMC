# Sky and Sun

## Current Implementation

**Sky dome** (`src/playing/sky.rs` + `assets/shaders/sky.wgsl`): a custom `Material` (`SkyMaterial`/`SkyUniform`) on a large sphere (`SKY_DOME_RADIUS`, currently 500 units) that re-centers on the camera every frame, so it never matters how far the player flies. The fragment shader computes a **symmetric** zenith/horizon gradient from the view direction — `mix(horizon_color, zenith_color, smoothstep(0.0, 0.4, abs(view_dir.y)))`. It's symmetric (same gradient looking up or down) on purpose: an earlier version had a distinct `ground_color` for the lower hemisphere, but that produced a visible, wrong-looking tint bleeding into view as the camera gained altitude above the real ground platform — there's no reason for the sky dome to simulate "ground" when actual ground geometry exists. The dome mesh itself is a level-0 icosphere (`Sphere::new(radius).mesh().ico(0)`, ~20 triangles): since the shader colors per-fragment (not per-vertex), a low-poly dome looks pixel-identical to a high-poly one at a fraction of the vertex/triangle cost.

**Sun** (`src/playing/sun.rs`): a real, tilted `Cuboid` mesh (not a shader-baked disc — see the "why not a cubemap/disc" reasoning below), `SUN_SIZE` units per side, positioned `SUN_DISTANCE` units from the camera along a fixed `sun_direction`, also re-centered on the camera every frame (same reasoning as the dome — otherwise flying around would change its apparent size/position). It's given an Euler tilt so it visibly reads as a cube (a corner/edge facing the camera) rather than a flat axis-aligned square. A `DirectionalLight` sun-a-like is spawned alongside it pointing the same direction, with real-time shadows on (Bevy's default — an earlier "no geometry benefits from shadows yet" override was removed once real terrain came back).

**Glow**: the sun cube uses an over-bright `emissive` color (values above 1.0, e.g. `LinearRgba::rgb(4.0, 3.6, 2.4)`) with `unlit: false`. Currently there's no `Hdr`/`Bloom` on the `Camera3d` (removed during the performance investigation, see `docs/performance-investigation.md`), so the glow doesn't visibly bloom outward right now — the emissive color still reads as a brighter cube, just without the soft bleed. Revisit once bloom is reintroduced.

**Ambient light** (`src/playing/mod.rs`'s `GlobalAmbientLight`): the sun is the *only* other light source, so any face without a direct line to it was lit by Bevy's tiny default ambient (`brightness: 80.0` against the sun's `illuminance: 6000.0`) — reading as almost pure black, a harsh contrast rather than a soft shadow. An initial bump to `300.0` wasn't enough; raised to `AMBIENT_BRIGHTNESS = 1200.0` (roughly a fifth of the sun's illuminance) — a flat, cheap stand-in for "light scattered by the sky" (not real bounced/indirect lighting), tuned by eye rather than to any physically-correct value — adjust further if it still looks off.

**Fog** (`src/playing/mod.rs`'s `DistanceFog` on the `Camera3d`): without it, chunks streaming in/out at the edge of `RENDER_DISTANCE` would visibly pop into and out of existence — a flat linear falloff (`FogFalloff::Linear`) fading to `fog_color()` hides that instead. `FOG_END` is derived directly from `config::world::RENDER_DISTANCE * CHUNK_SIZE` (not a separately-tuned magic number) so the fog boundary always tracks wherever the actual chunk-loading boundary is, even if render distance changes later; `FOG_START` is 40% of that so the transition is gradual rather than a visible line. `fog_color()` reads `SkyUniform::default().horizon_color` directly (rather than duplicating the literal) so fog always matches the sky's horizon and distant chunks fade into the sky, not into an arbitrary gray.

**A real bug this introduced**: `DistanceFog` fully saturates to the fog color for anything beyond `FOG_END` (128 units), but the sun cube deliberately sits `SUN_DISTANCE` (400 units) away — it was reading as a flat gray square (100% fog color) instead of its own glowing color. `StandardMaterial::fog_enabled` (defaults to `true`) controls this per-material; the sun cube's material now sets `fog_enabled: false` (`src/playing/sun.rs`), since it's meant to read as infinitely distant, not part of the local foggy atmosphere. The sky dome was never affected by this — `SkyMaterial`'s custom shader never had fog logic in it at all, so it was immune from the start; only `StandardMaterial`-based meshes (the sun cube, chunk terrain) automatically get Bevy's built-in fog applied.

**Terrain material** (`world::manager`'s shared chunk `StandardMaterial`): the defaults (`perceptual_roughness: 0.5`, `reflectance: 0.5`) are tuned for a semi-glossy "plastic" look, which showed up on flat-shaded voxel terrain as an unnatural, camera-relative specular hotspot sliding across the ground. Set to `perceptual_roughness: 1.0, reflectance: 0.0` — fully matte, no specular term at all, leaving plain diffuse (light-direction-dependent, not view-direction-dependent) shading. The remaining "harsh, hard-edged lighting" look (a sharp jump in brightness between adjacent faces at different angles, rather than a soft gradient) is inherent to flat per-face normals lit by one hard directional light — every face is uniformly lit or unlit, with nothing in between. That's a bigger stylistic decision (e.g. Minecraft-style fixed per-face-direction brightness instead of true directional lighting, or baked ambient occlusion at edges) than a quick tuning fix, and hasn't been addressed yet.

## Why a shader-driven sky dome (not a flat color or a cubemap texture)

Three approaches were considered before landing on the current one. This is kept as a record of the trade-offs, since the reasoning still applies to future sky work (day/night, stars, aurora, fog — see below).

### Option 1: Flat clear-color sky + sun
A `Camera3d` `ClearColor` for the sky, `DirectionalLight` for the sun, optionally a small emissive mesh for a visible sun disc.
- **Pros**: effectively free (no draw calls for the sky itself), no shader/exposure math, simplest possible.
- **Cons**: visually flat — no horizon gradient, nothing changes as you look around; can't represent a gradient, stars, or aurora at all.

### Option 2: Procedural gradient sky dome — **chosen**
A large sphere/cube mesh around the camera, unlit, colored by a gradient (vertex colors or, as built, a WGSL shader).
- **Pros**: real horizon gradient, still no texture assets, still cheap (one low-poly mesh), a visible sun is trivial to add, every future feature (day/night, stars, aurora, fog) is just more shader logic on the same material driven by uniforms — no new draw calls, no CPU-side regeneration.
- **Cons**: needs a small custom material/shader, needs the dome-recentering system.

### Option 3: Real cubemap skybox texture (`bevy_light::Skybox`)
Bevy's built-in `Skybox` component with a `Handle<Image>` cubemap, generated procedurally at runtime (no image assets available) via raw pixel construction + `reinterpret_stacked_2d_as_array` + a `Cube`-dimension `TextureViewDescriptor`.
- **Pros**: uses Bevy's real skybox path (foundation for image-based lighting via `GeneratedEnvironmentMapLight` later); swapping in a real HDR cubemap later is a drop-in change.
- **Cons**: by far the most complex to build correctly (manual `Extent3d`/`TextureDimension`/`TextureFormat`/reinterpretation, fiddly face ordering); **`Skybox.brightness` is in physical cd/m² units calibrated for HDR-authored source images** — a naive procedural texture with plain 0–1 sRGB colors needs `brightness` tuned empirically into the thousands to be visible at all after tonemapping (this cost real debugging time: `brightness: 1.0` rendered pure black); every future feature (time-of-day, stars, aurora) would mean regenerating the whole texture on the CPU, or blending multiple cubemaps, which the single-`Skybox` component doesn't support natively.

**Why Option 2 won**: every planned future feature — day/night cycles, twinkling stars, aurora borealis, fog — is naturally a per-pixel, per-frame *shader* computation driven by a handful of uniforms (time, sun angle, weather intensity). A flat color can't do per-pixel anything; a baked cubemap would need full CPU regeneration (or multi-texture blending Bevy doesn't support out of the box) for the exact same thing a shader gets by just reading a `time` uniform. The cubemap approach was also the most fragile of the three in practice.

## Future Sky Features (planned, not yet built)

All of these extend `SkyUniform`/`sky.wgsl` in place — no architecture change:
- **Day/night cycle**: drive `sun_direction` + zenith/horizon colors from a time-of-day resource, updated once per frame.
- **Twinkling stars**: a `time` uniform + procedural star-field noise in the fragment shader, faded in based on how far below the horizon the sun is.
- **Aurora borealis**: animated noise/sine bands blended additively, masked to the upper sky.

**Fog** is built (see "Current Implementation" above) via the simpler of the two options considered — Bevy's built-in `DistanceFog` camera component, not blended into `sky.wgsl` itself. Good enough for hiding render-distance pop-in; revisit blending it into the shader directly only if a reason to (e.g. fog that reacts to day/night or weather) comes up.
