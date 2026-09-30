# Improvements Log

A running record of deliberate changes and the reasoning behind them.
[`ARCHITECTURE.md`](ARCHITECTURE.md) describes what the code *is*; this file
records *why it became that*, including decisions that were later reversed.

Read the **Standing decisions** table before proposing a change — if a new
idea contradicts a row there, that is a conversation to have, not something
to quietly undo.

---

## Standing decisions

Grouped by area. Each row is a rule currently in force. When one changes,
move the old version to **Reversals** below instead of editing it away.
Last reviewed in full: audit #2, 2026-09-26.

### Dependencies and build

| Decision | Why |
| --- | --- |
| `encase*` pinned to `0.12.1` in `Cargo.lock` | `0.12.2` moved to `syn 3` while `bevy_macro_utils` is on `syn 2`, and the mismatched `syn` types break the `bevy_encase_derive` proc macro. Do not `cargo update -p encase` past this until upstream fixes it. (`syn 3` itself is compiled anyway via `bytemuck_derive`; the pin is about the type mismatch, not build time.) |
| `opt-level = 0` for our crate, `3` for dependencies | Our code compiles fast; Bevy is compiled once and cached. Raise ours only after **measuring** a real hot loop — meshing is the named trigger. |
| Windows uses `rust-lld.exe` as its linker (`.cargo/config.toml`); Linux/macOS blocks are left commented as reference, not enabled | Requested — this project only targets Windows right now. Confirmed present on this machine's toolchain by checking the filesystem before enabling it, not assumed. |
| `bevy` uses `default-features = false, features = ["3d", "ui"]`, dropping `audio` and `2d` | No sound exists; the UI camera being `Camera2d` still works without the `2d` feature because `ui` already pulls in the Core2d render pass it needs — reasoned from the dependency graph (audit #2, "Step 2"), confirmed by a clean build and a boot run reaching `Menu`. Step 3 (dropping glTF) stays undone: it needs every `3d` sub-feature this project actually uses hand-listed correctly, which is worth doing with a build available to check the list is complete, not blind. |
| `noise` added as a dependency ahead of any code using it | Added in the same pass as the Bevy feature trim specifically so both changes share one full rebuild instead of two. Default features only; no code reads it yet. |

### Layout

| Decision | Why |
| --- | --- |
| `mod.rs` module style | Every module here has submodules, so the sibling-file style pairs each folder with a file and doubles the tree. Bevy is written this way too. |
| `main.rs` holds only `mod` declarations and crate attributes | Setup belongs in `app`. The only reason to reopen it is adding a module. |
| Every state lives under `states/`, one folder each; a sub-state's folder sits inside its parent's | The top level separates states from infrastructure, and the folders mirror the state hierarchy — `paused/` is in `ingame/` because `Paused` is a sub-state of `InGame`. |
| The state machine lives in `states/mod.rs`; each state folder is private to it | Someone looking for `GameState` finds it where the states are. `app/` keeps only `run()` and the composition root. |
| Domains are top-level siblings: `camera/`, `input/`, `window/`, `world/` | A module that owns a category of thing is a domain, not a utility. `window/` was promoted out of `utils/` once it split into two files by lifecycle. |
| `utils/` holds only small, complete modules with no natural domain (currently just `log.rs`) | The bar is completeness with no natural home elsewhere, not size — kept narrow on purpose so it does not collect an in-progress design by default. |
| One file per lifecycle when a module does both startup and runtime work | `window/` is `setup.rs` (once) plus `toggles.rs` (on keypress). |
| Debug-only tooling lives in its own module behind one `#[cfg(debug_assertions)]` | `states/debug.rs` is compiled out of release as a whole — provably absent, not a cfg on every item. |
| Engine plugin configuration lives inline in `AppPlugin::build`, not a separate adapter | Deciding which Bevy plugins run is a composition decision, the same kind as adding a domain plugin. Tried as `utils::engine` and reverted. |

### Config

| Decision | Why |
| --- | --- |
| Every configurable value lives in `config/`, one file per category: `window`, `input`, `camera`, `world` | One answer to "where do I change a setting?". A new category is a new file, not a decision. |
| `config/` is `pub const` values; the only types allowed are small enums a setting chooses between (`SprintMode`) | A settings surface meant to be read in seconds. No structs or `Default` impls until values must load from disk at startup. |
| Bevy enums used directly (`PresentMode`), never mirrored | A copy would lose the fallback semantics and need updating whenever Bevy's enum grows. "Let Bevy decide" is the *value* `AutoVsync`, not a separate mode. |
| Values that are not player preferences stay with their owner, not in `config/` | Camera draw order and MSAA are rendering details; the pitch clamp is a safety limit. They live in `camera_*` / `input::look`. |
| Config modules are imported under an alias when the bare name would read as the importing module | `use crate::config::input as controls` inside `input/`, `config::camera as config` inside `camera/`, `config::world as config` inside `world/`. |

### Performance

| Decision | Why |
| --- | --- |
| Rare triggers are run conditions, not `if`s inside systems | The system is skipped outright when its trigger is absent — no query fetch, no body. Chain with `.and_then(..)`; `.and()` is deprecated. |
| Per-frame systems exit on the common case before querying or doing maths | At `opt-level = 0`, inlined glam maths runs unoptimised in our crate. Skipping it beats speeding it up. |
| `AudioPlugin` and `GilrsPlugin` are disabled at runtime | No sound and no controllers exist. Both are leaf plugins. Re-enable Gilrs for controller support; delete the Audio line when the `audio` feature is dropped. |
| Systems that read a moving entity's `Transform` to decide whether to act (e.g. `load_chunks_around_player`) filter on `Changed<Transform>`, not just the entity's marker | If the entity hasn't moved, whatever depends on its position can't have changed either. Coarser than tracking the specific derived value (rotating in place also counts as "changed"), but it's the built-in query filter, reached for before a hand-rolled check. |
| `Chunk` stores blocks with `y` slowest (`x + z*S + y*S²`, "YZX", the layout Minecraft uses), never a different axis order without updating `Chunk::index`'s pinning test | Terrain is naturally described by height, so a layout where every block sharing a height is contiguous turns "fill everything below this height" into one slice fill (`Chunk::fill_below_height`) instead of a per-block loop. Whichever axis order is picked, generation and meshing must agree with it — that was the original bug this fixed. |

### States

| Decision | Why |
| --- | --- |
| Each state's screen is self-contained; duplication is intentional | The screens are meant to diverge completely; a shared abstraction would have to be torn out. |
| `Loading` is boot loading only | World generation is planned as `InGame`'s first sub-state and soft loading as a counted overlay, not a state. See [`LOADING.md`](LOADING.md). |
| `Paused` is a sub-state of `InGame`, not a sibling | Makes "paused with no world loaded" unrepresentable instead of guarded at runtime. |
| The pause screen is a translucent overlay (`OVERLAY` alpha `0.5`) | The world stays visible underneath. Earlier "too dark at any alpha" reports were the UI-texture accumulation bug, not the alpha. |

### Camera and rendering

| Decision | Why |
| --- | --- |
| `camera/` owns camera entities only; `input/` steers the *player*, and the world camera follows it (`camera::follow`), never the reverse | `LookAngles` moved to `player/` when third person arrived — the camera no longer has its own orientation, it derives one from the player's. Dependencies run one way: `input/` writes `player/`, `camera/` reads it. See 2026-09-29. |
| The UI camera is spawned at `Startup` and lives for the whole run; the world camera is spawned and despawned with `GameState::InGame` | State screens spawn only their own content and never contend over the view; nothing 3D renders in menus. |
| The world camera has no starting transform of its own; `camera::follow` places it before the first frame renders | Choosing a starting position for a camera that immediately gets overwritten would just be a second place to keep in sync with the player's. |
| Third person orbits a pivot at `config::camera::THIRD_PERSON_PIVOT_HEIGHT` above the player's feet, `THIRD_PERSON_DISTANCE` back along the player's own look angles, always facing the pivot | The standard third-person shape (Minecraft's own values: distance 4). Orbiting the *player's* angles, not a separate camera rotation, is what keeps the player centred as the view turns. See 2026-09-29. |
| The camera-follow system runs in `PostUpdate`, before `TransformSystems::Propagate`, gated on `Changed` player transform/look angles | Placing it after `input/`'s `Update` systems means it reads this frame's player position, not last frame's — a frame of lag here reads as jitter. The change-detection gate is what lets an idle player leave the camera untouched too. |
| Draw order: world camera `0`, UI camera `1` | The UI renders on top by construction. With no `IsDefaultUiCamera` marker, Bevy routes UI to the highest-order camera — mark one explicitly if a third camera is ever added. |
| UI camera `Msaa::Off`; world camera `Msaa::Sample4` | UI quads and font-atlas text gain nothing from multisampling; a 4x UI target would be pure cost. Do not "fix" the mismatch by matching MSAA — see the next row. |
| The UI camera clears its own texture to transparent (`ClearColorConfig::Custom(Color::NONE)`) and composites onto the window with `PREMULTIPLIED_ALPHA_BLENDING` | Because of the MSAA mismatch it renders into its own intermediate texture, so it must clear or every frame's UI piles onto the last. **Never** set it back to `ClearColorConfig::None`. |
| FOV is `config::camera::FOV_DEGREES`, in degrees, applied as an explicit `Projection` at spawn | Degrees are what a person tunes; radians are converted at the one place that needs them. Explicit so the value is owned here, not an unseen Bevy default. |

### Window

| Decision | Why |
| --- | --- |
| `config::window::WIDTH`/`HEIGHT` are **logical** pixels | Same visual size at any OS scaling. |
| On a live window, restore size with `window.resolution.set(..)`, never `window.resolution = WindowResolution::new(..)` | `set()` keeps the OS scale factor; a fresh `WindowResolution` resets it to `1.0` and shrinks the window on scaled displays. `WindowResolution::new` is only safe at window creation. |
| `BORDERLESS` is stored as the inverse of Bevy's `decorations`; the inversion happens once, in `window/setup.rs` | Bevy models "draw the title bar". |
| `present_mode` is applied in `window/setup.rs` | It is a property of Bevy's `Window`, not of a render plugin. |
| Fullscreen means `BorderlessFullscreen(Current)`, one shared constant in `window/mod.rs`; the F10/F11 toggles are not gated on any state | Alt-tabs instantly and keeps the display mode. Leaving fullscreen should never depend on where the player is. |

### Input

| Decision | Why |
| --- | --- |
| `input/` holds controls that interpret input every frame; one-shot key actions live with what they change | Look, movement and cursor capture are continuous. Escape and F10/F11 are single actions whose logic belongs to their target, with the key as a run condition. |
| All key bindings live in `config/input.rs`, never inline in a system | One visible set, and a unit test proves no key is bound twice. |
| Descend is Left Shift | Requested. Freed Left Ctrl for the hold-to-sprint mode. |
| Sprint is chosen by the compile-time `SPRINT_MODE`: `DoubleTap` (default) or `Hold` (`SPRINT_HOLD_KEY`, Left Ctrl) | Both styles exist; switching is a config edit and rebuild, not a runtime menu. The `#[allow(dead_code)]` on `SprintMode` is intentional — one variant is always unconstructed. |
| `DoubleTap`: the *same* W/A/S/D key twice within `DOUBLE_TAP_WINDOW` (0.3 s); sprint stays on until every movement key is released | Minecraft's feel — no re-triggering on every direction change. |
| No cursor re-grab on focus, no focus request after F10/F11, no Escape debounce | Each was added for a theory the logs disproved during the 2026-09-26 overlay bug, and removed. Re-add only with evidence (for example, logged `WindowFocused` loss). |
| `input/movement.rs` only writes `MovementIntent`; it never touches `Transform` itself | Separates "what the keys ask for" from "what actually happens" — `player::physics` decides that, since a wall or the ground can override intent. Also what lets both be unit-tested independently. |
| Space and Left Shift double-taps (take off, fast ascent, land) share `DOUBLE_TAP_WINDOW` and the same tap-tracking machinery as sprint, across all six movement/vertical keys at once | One completed double-tap clears the record outright, so a third tap starts a fresh pair instead of chaining — tapping Space three times takes off once, not "take off, then fast ascent". Reusing sprint's timing constant means one setting tunes the feel of every double-tap in the game, not several that could drift apart. |

### Player

| Decision | Why |
| --- | --- |
| The player is its own entity (`player::Player`), separate from either camera, carrying `LookAngles`, `MovementIntent`, `Flying`, and its physics state | Third person needs a body the camera can stand behind — the camera *is* the player was only ever true in first person. `world/` and the debug chunk grid load around the player now, not the camera, since in third person they can be in different chunks. |
| The player model is a glTF (`assets/models/player.glb`, a Blockbench export) loaded with Bevy 0.19's `WorldAssetRoot` (`GltfAssetLabel::Scene(0)`) | Requested: a real model instead of the bare camera. `WorldAssetRoot` is what `SceneRoot` became in this Bevy version — confirmed from `bevy_gltf`/`bevy_world_serialization` source, not assumed. |
| The model needs no rotation offset: it already faces −Z, matching `Transform::forward` | Checked by decoding the glTF's own UV data and confirming the head's face texture sits on the −Z-normal face, not assumed from the file looking roughly humanoid. |
| Only yaw reaches the player's `Transform`; pitch only moves the camera | The model must stay upright looking up or down. `input::look` writes both angles to `LookAngles` but rotates the body by yaw alone. |
| Movement, gravity, and collision act on the player's own `right()`/`forward()`, which are level because of the yaw-only rule above | Walking forward while looking down stays along the ground instead of driving into it — no separate "flatten this vector" step needed anywhere. |
| Two game modes (`config::player::GameMode`), Survival and Creative, chosen by an `ActiveGameMode` resource and switched with `F4` | Requested, with Minecraft's names kept only as placeholders until the game has its own. A resource, not a player component, since mode outlives any one world and systems that never touch the player (every testing-tool key) need to read it as a run condition (`in_creative`). |
| Every testing-tool hotkey (`world::debug`, `render::debug`) is additionally gated on `in_creative` | Requested: Survival should play the way the game ships; debug tools are a Creative-only convenience layered on top of the existing `TESTING_TOOLS_ENABLED` gate, not a replacement for it. |
| Flight exists only in Creative, toggled by double-tapping Space (take off) or Left Shift (land), tracked as a plain `Flying(bool)` component | Requested. A component, not folded into `MovementIntent`, because it's state that persists across frames (am I currently flying) rather than a per-frame ask. Switching game mode resets it — off in Survival unconditionally, `CREATIVE_STARTS_FLYING` in Creative — so a mode swap can never leave the player flying somewhere flight isn't supposed to exist. |
| Gravity, jump speed, terminal velocity, and the hitbox are plain constants in `config::player`, not derived from the model | Same reasoning as every other `config/` value — a tunable someone would actually reach for. The hitbox (0.6 wide, 1.9 tall) is deliberately narrower and shorter than the 2-block model, the same relationship Minecraft's 0.6-wide hitbox has to its own taller model, so the player fits through gaps the model's own silhouette wouldn't. |
| Collision moves the player one axis at a time (vertical first), each axis stopping flush against the first solid block rather than the whole move being cancelled | Per-axis is what makes sliding along a wall possible — a blocked horizontal axis doesn't also cancel a still-open vertical one (or the other horizontal one). Vertical first settles landing before any sliding, so walking along the ground never snags on the block being stood on. |
| A move is split into sub-steps under one block long | Stops a fast fall or a long low-framerate step from tunnelling clean through a one-block-thick floor. |
| `LoadedChunks::is_solid` treats an *ungenerated* chunk inside the world's vertical extent as solid, and anything outside that extent as air | Generation is async, so a player can physically reach a chunk before its data exists — treating that as solid ground (rather than open air to fall through) means they wait for it instead of falling through terrain that just hasn't arrived yet. Outside the configured world height has to stay flyable regardless, or a tall enough fall or flight ceiling would hit an invisible floor. |
| The collision maths (`move_and_collide`, `move_axis`, `overlaps_solid`) is plain functions over an `is_solid: impl Fn(IVec3) -> bool` closure | Unit-tested directly against hand-written "is this block solid" functions, no `World` or chunk data needed — the same reasoning `world::generation::generate` was built as a pure function for. |
| Standing still on the ground (or hovering while flying) returns from `apply_physics` before touching `Transform` at all | Beyond the saved work, this is what keeps chunk loading and the camera follow idle too — both are gated on `Changed<Transform>`, so an idle player keeps every system downstream of its position idle as well. |
| Player spawn is still a fixed constant (`player::SPAWN_POSITION`), not the real terrain surface | Unchanged from the world-camera version; genuinely needs solving once there's gravity to fall under, so it's `AUDIT.md` 4.8, not deferred indefinitely. |
| `Motion` tracks whether the *last* move was stopped on `x`/`z`, never `y`, as its own field (`horizontal_collision`) rather than deriving it ad hoc from `Moved` where it's consumed | `y` is blocked every frame while walking on flat ground (that's landing, not a wall) — folding vertical in would misfire constantly. Keeping it on `Motion`, read via a `pub(crate)` accessor, is what lets `input::movement` (a different top-level module) see it without `physics`'s internals becoming any more public than that one fact. |
| A `SprintMode::DoubleTap` sprint cancels itself the frame after a horizontal collision (`config::player::RESET_SPRINT_ON_COLLISION`, default `true`) | Requested: running into a block and jumping over it, still holding the movement key throughout, otherwise keeps the old sprint engaged the whole way — it never gets the *only* other way sprint turns off (every movement key released). A config flag because it's a judgment call about feel, not a fixed rule. The reset logic (`cancel_sprint_on_collision`) is a one-line pure function specifically so it's unit-tested directly rather than only reachable through the full ECS system. |
| Terrain collision can be switched off entirely (`player::physics::CollisionEnabled`, a resource) via a debug hotkey (`F5`/`TOGGLE_COLLISION`) | Requested: a noclip toggle for testing, in Creative only — same shape as every other testing feature (`TESTING_TOOLS_ENABLED` + `in_creative` + an `_INITIALLY_*` constant guarded by both). Off, physics applies a frame's raw delta straight to `Transform` with no block checks; `grounded` and the collision-tracking flag both go false rather than keep a stale value from before it was switched off. |
| Switching game mode (`game_mode::toggle_game_mode`) unconditionally re-enables collision, regardless of what the hotkey or `COLLISION_INITIALLY_DISABLED` last left it at | Leaving Creative with collision off must never strand the player noclipping through Survival, and re-entering Creative should start from a clean state rather than remembering a previous testing session. Same reasoning `Flying`'s reset on mode switch already used. |

### World

| Decision | Why |
| --- | --- |
| `world/` owns voxel data only — chunk coordinates, storage, generation | Meshing and chunk rendering belong in `render/`. Keeping "what blocks exist" apart from "how they reach the screen" from the start. |
| A chunk's blocks are one flat `Vec<Block>` indexed by a computed offset; never nested `Vec`s | One contiguous allocation; the layout meshing walks. (Which axis is slowest is still open — `AUDIT.md` 1.1.) |
| `ChunkPos` is its own type, distinct from block coordinates, and a `Component`; `ChunkPos::containing` floors, never truncates | A chunk and a block coordinate cannot be swapped by mistake; flooring keeps negative coordinates in the right chunk. It's a `Component` so `render/` can tag a chunk's mesh entity with the position it renders. |
| The chunk-data resource is `LoadedChunks`, never `World` | `world/mod.rs` glob-imports `bevy::prelude::*`, which exports Bevy's own ECS `World`. Two types of the same name in one file is a standing trap. |
| Loaded chunks live in one resource (`HashMap<ChunkPos, Chunk>`), not one entity per chunk | Nothing about chunk *data* needs the ECS. The mesh entities `render/` spawns are what renders it. |
| `RENDER_DISTANCE` is a **circular** (squared-distance) radius around the player's chunk *column*, horizontally only, computed by one function (`in_render_distance`) shared by loading and eviction | A bounding square's corners are `sqrt(2)` chunks away — more chunks loaded than the radius implies, and inconsistent pop-in by direction. Sharing one function keeps load and evict from ever disagreeing about a chunk at the boundary. See 2026-09-27. |
| Chunks outside render distance are evicted (dropped from `LoadedChunks`, firing `ChunkUnloaded`) as the player moves; the whole map is reset in one step on leaving `GameState::InGame`, without firing `ChunkUnloaded` per chunk | Streaming needs unloading, not just loading, or memory (and eventually stale data) only grows. The exit case doesn't message per chunk because `render/` clears its own entities independently on the same transition. |
| Every horizontally-in-range chunk *column* loads its **entire** vertical extent (`CHUNKS_ABOVE_SEA_LEVEL` down to `CHUNKS_BELOW_SEA_LEVEL`), not a vertical radius around the player's own height ("Option A") | The world's height is a fixed property of the world, not of wherever the player currently stands — a vertical radius (loading only nearby layers) was considered and rejected because it would make the loaded world's shape depend on the player's own Y, contradicting the "fixed height" the sea-level constants are meant to describe. See 2026-09-27 (2). |
| `CHUNKS_ABOVE_SEA_LEVEL` / `CHUNKS_BELOW_SEA_LEVEL` are active, each `1` for now | Requested — wired in alongside Option A. Deliberately small while vertical loading is new and worth watching closely, not the eventual target (`20`/`12`); raising either multiplies how many chunks a column contains, so it's worth changing in small steps. See 2026-09-27 (2). |
| `generation::generate` is a pure function (no ECS, no I/O) | Unit-testable directly, and movable onto a background task pool unchanged — and now is. |
| `world::debug` (chunk locking) can freeze loading/unloading for testing, behind a hotkey (`F9`) | Requested, as the first of what will be several testing features. Sets the pattern below for all of them. |
| Chunk generation runs on `AsyncComputeTaskPool`, tracked in `PendingChunks` (`HashMap<ChunkPos, Task<Chunk>>`) keyed the same way as `LoadedChunks`; `load_chunks_around_player` only spawns tasks and decides eviction, `apply_generated_chunks` only polls and applies them | Splits "what's wanted" from "what's finished" into two systems with two different, correct run conditions — the first only needs to run when the player moves; the second must run every frame regardless, since a background task can finish on a frame nothing else changed. |
| A pending chunk's `Task` is dropped (cancelling it) the moment its position falls out of range, the same frame `LoadedChunks` evicts an already-loaded one | No point letting a chunk finish generating for a position the player already left; `retain` on both maps, driven by the same `in_render_distance` check, keeps them from disagreeing about what's still wanted. |
| `PendingChunks` is cleared (cancelling everything in flight), not just `LoadedChunks`, on leaving `GameState::InGame` | Without this, a task started before leaving could finish after re-entering and insert into the *new* session's `LoadedChunks` as if it belonged there. |
| Terrain is a heightmap from `noise::Fbm<Perlin>`, sampled in *world* space, not chunk-local space, centred on sea level (absolute world height `0`) and bounded by `TERRAIN_AMPLITUDE` in either direction | World-space sampling is what makes neighbouring chunks' terrain line up at the seam — chunk-local sampling would make every chunk look like its own island regardless of neighbours. The amplitude bound is what makes a chunk fully outside the band trivial to generate (solid or air, no noise sampled), which matters increasingly as `CHUNKS_ABOVE_SEA_LEVEL`/`_BELOW_SEA_LEVEL` grow. See 2026-09-27 (2). |
| The guaranteed-solid band below `-TERRAIN_AMPLITUDE` (or a chunk's own bottom, whichever is higher) is bulk-filled with `Chunk::fill_below_height`; only the band a column's height might actually land in is set block-by-block | Keeps the audit #2 1.1 contiguous-fill optimisation meaningfully alive now that terrain isn't flat, instead of falling back to a per-block loop for the whole chunk. |
| `ChunkPos::face_neighbors` returns the six chunks sharing a face with this one, order unspecified | One place both `render/`'s padding lookup and its re-mesh-on-load trigger get "which chunks touch this one" from, rather than each recomputing offsets separately. |
| `ChunkPos::of_block` splits an integer world position into its chunk and local coordinate with Euclidean (not truncating) division | Same reasoning as `ChunkPos::containing`'s floor division: block `-1` has to land as the last block of chunk `-1`, not the first of chunk `0`. Added for `player::physics`' block-solidity lookups, which query arbitrary world positions rather than a chunk's own local range. |

### Testing tools (`config::debug`, `*::debug`)

| Decision | Why |
| --- | --- |
| Every testing feature is gated twice: `#[cfg(debug_assertions)]` (compiled out of release entirely) *and* `config::debug::TESTING_TOOLS_ENABLED` (a plain runtime `bool`, debug builds only) | The first guarantees a testing tool can never reach a shipped build, structurally, matching `states::debug`'s existing standing decision. The second is independent of that: being a debug build isn't the same as *actively testing* right now, so a stray hotkey press during ordinary debug-build play can't silently trigger a testing feature. |
| The state a testing feature needs (e.g. `world::ChunkLock`) is a normal, unconditional resource; only the hotkey that *changes* it lives in the `#[cfg(debug_assertions)]` module | So the systems that read it (e.g. `load_chunks_around_player`'s run condition) are identical in every build — always "off" in release, since nothing there can ever set it otherwise — rather than needing `Option<Res<_>>` or a second code path. |
| A testing feature's registration is skipped outright with a plain `if TESTING_TOOLS_ENABLED { app.add_systems(..) }`, not a `run_if` on the system | When disabled, the system is never in the schedule at all, not added-then-skipped every frame. |
| Testing-feature key bindings still live in `config/input.rs`, under the existing debug section, not in `config/debug.rs` | All key bindings live in one place regardless of owner, so the "no key bound twice" test covers them too. `config/debug.rs` holds the non-binding switch (`TESTING_TOOLS_ENABLED`) that gates whether those bindings do anything. |
| A settings-surface enum for a testing feature (`ChunkGridMode`) lives in `config::debug`, the same as `config::input::SprintMode` — the type and its starting value in `config`, the behaviour that interprets it in the module that owns the feature (`render::debug`) | Consistent with every other config value in the project: `config` holds what's tunable, the domain module holds what it does. Requested explicitly for `ChunkGridMode` and the three sea-level/chunk-grid `_INITIALLY_*` constants. See 2026-09-27 (2). |
| A starting state that needs live ECS data not yet available at `Default`-impl time (`CHUNK_GRID_INITIALLY_LOCKED`, which needs an actual camera position) is applied by a system with a `Local<bool>` "have I done this yet" guard, not a `Default` impl | The lock can't default to a position before a camera exists to read one from; ordering one system explicitly after camera spawn would still race the very first time the state is entered, where a `Local<bool>`-guarded system that simply runs every frame until it succeeds cannot. See 2026-09-27 (2). |
| Gizmos draw on their own render layer (`render::debug::GIZMO_LAYER`), which only the world camera has; the UI camera stays on layer `0` alone | Gizmos render to every camera whose layers intersect theirs. On the shared default layer the UI camera — a fixed orthographic `Camera2d` — drew a flat copy of every gizmo pinned to the middle of the screen. Moving gizmos off layer `0`, rather than the UI camera, keeps UI rendering out of it entirely. See 2026-09-28 (4). |
| Two testing actions can share one physical key if they're the same gesture at different commitment levels (`F7` cycles the chunk grid, `Alt+F7` locks it) — not a reason to spend a second key | The run conditions for both explicitly exclude each other's modifier state (`input_pressed(DEBUG_MODIFIER)` / `not(...)`), so exactly one fires per keypress. A dedicated modifier (`Left Alt`) rather than reusing `SPRINT_HOLD_KEY` (`Left Ctrl`): this is an unrelated debug gesture, not a second meaning for a gameplay key. See 2026-09-27 (2). |

### Render

| Decision | Why |
| --- | --- |
| `render/` owns turning loaded chunks into what's on screen — meshing, materials, and the chunk mesh entities' lifecycle; `world/` never imports it | One-way dependency, `render/` -> `world/`, the same direction as `input/` -> `camera/`. |
| `world/` and `render/` communicate through `ChunkLoaded`/`ChunkUnloaded` messages, not by `render/` polling `LoadedChunks` for changes | Event-driven: `render/`'s systems do nothing on a frame with no messages, instead of diffing a `HashMap` every frame whether or not it changed. |
| Meshing uses the `binary_greedy_meshing` crate (real greedy quad merging), not a hand-rolled face-culled mesher | A maintained, MIT-licensed Rust port of a proven reference algorithm, generic over chunk size, already used in a real Bevy voxel game — preferred over hand-porting the reference C++ or hand-rolling our own. See 2026-09-27 and Reversals. |
| The mesher's padded input buffer is filled from the six face-adjacent neighbours' boundary layer, wherever they're loaded; diagonal neighbours are never read | The crate's face culling never looks at a cell's diagonal neighbour, so reading diagonal chunk data would be pure waste — confirmed from the crate's own source, not assumed. |
| Loading a chunk also re-meshes any of its already-spawned face neighbours | A neighbour meshed *before* this chunk existed was built assuming this side was open air; without re-meshing it, that mesh would stay stale — visibly, a permanently double-sided seam — for as long as the neighbour stays loaded. |
| One shared `Handle<StandardMaterial>`, built once at `Startup`, reused by every chunk mesh | Same reasoning as `scene.rs`'s one shared cube mesh, applied to the material instead. A texture atlas replaces this once there is more than one visible block type. |
| Chunk mesh entities are tracked in a `HashMap<ChunkPos, Entity>` (`ChunkEntities`), not found by querying a marker component | A `ChunkUnloaded` needs to despawn one specific chunk's entity; a map lookup is one step, a query would scan every spawned chunk. |
| The world's `DirectionalLight` is spawned and despawned by `render/`, tied to `GameState::InGame`, not `world/` | It exists purely to make chunk meshes visible — the same category of thing as the material they're given, not world data. |

### Logging and process

| Decision | Why |
| --- | --- |
| Log config lives in `utils/log.rs` and is set on `DefaultPlugins` in `AppPlugin` | `LogPlugin` is configured *on* `DefaultPlugins`; routing it through `main.rs` would plumb settings down only to hand them back. |
| Log filters build on `DEFAULT_FILTER` rather than replacing it | Bevy's own defaults survive; ours stay additive. |
| Audits are periodic: `AUDIT.md` is rewritten each time from its own checklist, and holds only *open* items | Decisions — accepted or rejected — are recorded here, so a later audit can compare instead of re-proposing. |

---

## Reversals

Decisions that were made and then undone. Listed so the discarded option is
not re-proposed as if it were new.

| Was | Now | Why it changed |
| --- | --- | --- |
| `u16` for window width/height | `u32` | The 8 bytes saved are meaningless for a struct built once, and `u16` invites an overflow footgun (`1280 * 720` does not fit) plus a cast at every Bevy boundary. |
| Sibling-file modules (`menu.rs` + `menu/`) | `mod.rs` style | Every module here has submodules, so the sibling style doubled the tree for no benefit. |
| All states nested under `states/` | One top-level folder per state | Each state should own its own folder rather than being a file in a shared one. |
| One top-level folder per state | Every state under `states/`, still one folder each | The top level had started mixing states with infrastructure. Grouping kept the folder-per-state property that was the original point. |
| UI camera `ClearColorConfig::None` ("draw over the world, don't erase it") | Clear to `Color::NONE`, premultiplied-alpha output | `None` is only correct when two cameras share an intermediate texture, which requires matching MSAA. Ours do not, so the UI texture was never cleared: a stuck, black-looking pause overlay and a frozen-looking view. See 2026-09-26. |
| Pause overlay alpha `0.75` → `0.45` → `0.3` | `0.5` | Every value "looked black" because of the UI-texture accumulation bug above, not the alpha. Tuned back up once that was fixed. |
| `regrab_on_focus_regained`, `Window::focused = true` after F10/F11, and a 0.25 s Escape debounce | Removed | Built for focus-loss and double-fire theories that the diagnostic logs disproved. |
| Live window resize via `window.resolution = WindowResolution::new(..)` | `window.resolution.set(..)` | Reset the OS scale factor to `1.0`, so the window came back smaller after leaving fullscreen on scaled displays. |
| Descend on Left Ctrl, sprint while holding Left Shift | Descend on Left Shift; sprint by double-tap, or holding Left Ctrl in `Hold` mode | Requested. |
| Sea-level marker: two lines through the camera's own `(x, z)`, re-centring every frame | A grid: one X/Z cross per loaded chunk *column*, fixed to that column's centre, both axis-colored | Requested: a grid tied to world positions reads as a spatial reference; two lines chasing the camera don't. See 2026-09-28 (3). |
| `paused/` as a top-level sibling of `ingame/` | `states/ingame/paused/` | The folders now say what the state machine already said: `Paused` exists inside `InGame`. |
| `camera_2d` / `camera_3d` | `camera_ui` / `camera_world` | Named for what each camera shows rather than how it renders. |
| Engine plugin config in `utils/engine.rs` | Inline in `AppPlugin::build`, next to the domain plugin list | Disabling `AudioPlugin`/`GilrsPlugin` and swapping in our window/log plugins is deciding what the app is made of — composition, not an adapter that turns our config into one Bevy value. |
| `window/` inside `utils/` | `window/` as a top-level module | A two-file lifecycle split (`setup.rs` + `toggles.rs`) is a domain, not a small complete utility — a sibling of `camera/` and `input/`. |
| `utils/` as "finished adapters between settings and the engine" | `utils/` as small, complete, domain-less modules | Once `window/` and the engine config moved out, only `log.rs` remained, and "adapter" no longer described the rule. |
| `[lints.rust]` table in `Cargo.toml` | `#![deny(unsafe_code)]` in `main.rs` — **but the table was never actually deleted** | The "Even Better TOML" extension showed a false error for the table. The move is only half done; see `AUDIT.md` 2.4. |
| Shared `states/screen.rs` helper | Per-state `screen.rs` | Self-contained states were preferred over DRY, since the screens are placeholders meant to diverge. |
| `Paused` as a `GameState` variant | `InGameState::Paused` sub-state | Enforcing "only pausable from in-game" structurally beats a runtime guard. Done once `InGame` owned real resources, as planned. |
| `ingame/screen.rs` placeholder | `ingame/scene.rs` | The flat colour was scaffolding; the 3D scene replaces it. |
| `ingame/scene.rs` (six placeholder cubes and a light) | Removed; the light moved to `render/`, the cubes deleted outright | Real chunk meshes replace them, per the module's own doc comment ("replaced wholesale by the voxel world"). See 2026-09-27. |
| `world::World` (chunk-data resource) | `world::LoadedChunks` | `World` shadowed Bevy's own ECS `World` type in a file that glob-imports `bevy::prelude::*`. Flagged in audit #2, fixed alongside the render work it made unavoidable to ignore. |
| `RENDER_DISTANCE` as a bounding square | A circle, by squared distance, shared by loading and eviction through one function | A square's corners are `sqrt(2)` chunks away — more chunks loaded than the radius implies, and inconsistent pop-in by direction. See 2026-09-27. |
| Hand-rolled face-culled mesher in `render::mesh` (own `Face`/`FACES` table, no merging, no neighbour awareness) | The `binary_greedy_meshing` crate (real greedy merging, neighbour-aware padding) | Requested: reference the `cgerikj/binary-greedy-meshing` algorithm for a "highly performant" mesher. A maintained Rust port of exactly that algorithm already existed on crates.io — used instead of hand-porting the C++ or hand-rolling greedy merging ourselves. See 2026-09-27. |
| Flat placeholder floor in `world::generation` (`fill_below_height` over the whole chunk, identical at every position) | A `noise::Fbm<Perlin>` heightmap, sampled in world space | Requested: real, "interesting" terrain via the `noise` crate. See 2026-09-27. |
| `GameConfig`/`WindowConfig`/`RenderConfig` structs | Plain `pub const` values | 68 lines of type machinery for 5 values. Nothing read the config after startup, so the types earned nothing. |

---

## Change log

### 2026-09-19

**Dependency fix.** Pinned `encase`/`encase_derive`/`encase_derive_impl` to
`0.12.1`. `0.12.2` bumped `syn` to 3 in a patch release, conflicting with
`bevy_macro_utils` on `syn 2` and breaking the `bevy_encase_derive` proc
macro. *Perf: none. Modularity: none — purely a build fix.*

**`main.rs` reduced to an entry point.** Module declarations, the
`windows_subsystem` attribute, and a call to `app::run`. The attribute is
gated on `not(debug_assertions)` so dev builds keep their console for logs.
*Perf: none. Modularity: setup stays in one place instead of leaking into the
entry point.*

**Plugin-per-domain composition.** Every domain is a Bevy `Plugin`, all
registered in `app::plugin::AppPlugin`. *Perf: none — `Plugin::build` runs
once at startup and a system registered via a plugin runs exactly as fast as
one registered inline. Modularity: the whole game reads as one list.*

**State machine.** `GameState` with `Loading`/`Menu`/`InGame`/`Paused`, one
module per state. *Perf: none. Modularity: adding behaviour to a state
touches only that state's folder.*

**`config.rs` reduced to constants.** Removed three structs and two `Default`
impls; 68 lines became 27. *Perf: negligibly positive — one startup `String`
allocation saved, zero effect on frame time. Modularity: better now (three
fewer types, `window.rs` lost a parameter), with one real cost — constants
cannot be loaded from a file, so a `settings.toml` would need a struct back.
Narrower than it looks, since a settings menu would mutate the live `Window`
component rather than these startup values.*

**Log filtering.** Silenced `wgpu_hal::vulkan::instance`, which emitted five
errors per startup about Steam and Epic overlay layers that are not
installed. *Perf: none. Modularity: none. Tradeoff: genuine Vulkan instance
errors go quiet too — acceptable because a real failure stops the app with a
clearer message, but comment it out first when diagnosing a GPU problem.*

**State transition logging.** `log_state_change` reports every transition,
skipping identity transitions. *Perf: negligible — an empty `MessageReader`
is a cursor check.*

**`app` submodule visibility tightened.** `config`, `log`, `plugin`,
`states`, and `window` are now private; only `GameState` is re-exported.
*Perf: exactly zero — visibility is erased before codegen. Modularity: `app`'s
coupling surface dropped from five modules to one type plus `run()`, and
reaching into its internals is now a compile error instead of an unnoticed
dependency.*

**Debug keybinds gated.** `debug_jump_to_state` and its key table are
`#[cfg(debug_assertions)]`, and the keys moved into a `DEBUG_JUMPS` table.
*Perf: the system no longer exists in release, so it costs nothing there
rather than nearly nothing. Modularity: adding a state to the shortcuts is
one table row. Verified `cargo clippy --release` is clean, so the exclusion
leaves no dead code.*

**Plugin registration grouped and ordered.** `AppPlugin` now registers in
three labelled steps — engine, shared infrastructure, states — instead of one
flat six-tuple, and records why `DefaultPlugins` must come first.
*Perf: none. Modularity/reading: the six plugins were not peers, and now do
not read as if they were. The ordering note prevents a real failure mode —
registering `GameStatePlugin` first produces no compile error, only a runtime
warning and a state machine that never transitions.*

**Window resolution units documented — later found to be wrong, see
2026-09-26.** Noted that `WindowResolution::new` takes physical pixels, so on
a display with OS scaling the window is smaller than "1280 wide" suggests.
This was an assumption from the constructor's parameter names
(`physical_width`/`physical_height`), not verified against how `bevy_winit`
actually uses the value — it turned out to be backwards for the one code path
that matters at startup. Left here uncorrected in place, rather than edited
away, precisely because acting on an unverified doc comment is what caused
the bug it led to.

**MSAA disabled on the UI camera.** Bevy defaults `Msaa` to `Sample4`, which
allocates a 4x multisampled render target and resolves it every frame. The UI
is axis-aligned quads and font-atlas text, where multisampling changes
nothing visible. *Perf: removes a 4x render target and its per-frame resolve
for zero visual cost — the largest single win found so far. Modularity: `Msaa`
is a per-camera component, so the 3D camera picks its own level
independently.*

**Explicit camera draw order.** The UI camera is now `order: 1` instead of
the default `0`. *Perf: none. Bug prevention: when the 3D camera arrives at a
lower order, the UI sits on top by construction. Two cameras sharing order
`0` on one target leaves draw order unspecified, which fails as a confusing
visual bug rather than an error.*

**`camera_2d` made private.** Nothing outside `camera` uses it. *Perf: zero.
Modularity: matches the `app` pattern — the folder exposes `CameraPlugin` and
nothing else.* A `pub(crate) use camera_2d::UiCamera;` re-export was tried and
reverted: nothing imports it yet, so it was just an unused import. Add it when
a real consumer appears.

**Key bindings centralised in `input/`.** Eleven bindings that were inline
`KeyCode` literals across `camera_3d.rs`, `pause.rs`, and `states.rs` are now
named constants in one module, with a test asserting no key is bound to two
actions. `LOOK_SENSITIVITY`, `MOVE_SPEED`, and `SPRINT_MULTIPLIER` moved there
too, as player-tunable settings. *Perf: none — constants inline identically.
Modularity/reading: a key clash was previously only findable by grepping three
files; it is now a failing test. Camera draw order and the pitch clamp
deliberately stayed in `camera_3d` — the first is a rendering detail, the
second a safety limit, and neither is a preference.*

**Runtime window toggles.** Added `FULLSCREEN` alongside `BORDERLESS` in
config (both start `false`), bound `F10`/`F11` in `input/`, and added
`WindowControlPlugin` in `app/window.rs` to mutate the live `Window`
component. *Perf: two systems doing a single `just_pressed` check per frame —
negligible, and they early-return before touching the window query. Nothing
re-creates the window; Bevy applies the component change. Modularity: window
vocabulary stayed in `app/window.rs`, keys stayed in `input/`, config holds
only starting values — each of the three files gained the part that belongs
to it.*

**Settings unified into `config/`, adapters split into `utils/`.**
`app/config.rs` became `config/window.rs`, `input/mod.rs` became
`config/input.rs`, and `app/log.rs` and `app/window.rs` moved to `utils/`.
`app/` now holds only `plugin.rs` and `states.rs`. `input/` is kept as an
empty signpost module reserved for future input behaviour. *Perf: none — pure
module reorganisation, identical codegen. Readability: settings had two homes
and now have one; `app/` dropped from six files to three, leaving only the
parts that shape how the game is assembled.*

Recorded because it was discussed and decided rather than assumed: a `utils/`
folder is normally an anti-pattern, because "miscellaneous" has no membership
test and becomes a junk drawer. It was adopted here with an explicit bar —
**finished adapters only, completeness rather than size** — written into
`utils/mod.rs` so the criterion travels with the folder. Revisit if anything
lands there that is still growing a design.

**`camera_2d` renamed to `camera_ui`.** The module and its plugin
(`Camera2dPlugin` -> `UiCameraPlugin`) now say what the camera is *for*. It
only ever drew the UI, and "2d" suggested it depended on the `2d` engine
feature, which it does not. The module doc now explains why a UI camera is
built from Bevy's `Camera2d` component, since that is exactly the confusion
the old name caused. Moved with `git mv` so history follows the file. *Perf:
none. Readability: the name matches the `UiCamera` marker it spawns.*

**`unsafe_code` deny moved from `Cargo.toml` to `main.rs`.** The `[lints]`
table is valid Cargo, but the "Even Better TOML" extension shows a false error
for it that no manifest edit can clear. As a crate attribute it is equivalent
for this single crate. *Perf: none. Readability: no false error for anyone
opening the manifest. Cost: `[lints]` would also cover future `tests/`,
benches, and workspace members, which the attribute does not — so it moves
back when those exist.* See `AUDIT.md` 4.6 for the full diagnosis, including
the first attempt that did not work.

**Stale documentation corrected.** `ARCHITECTURE.md` still described `Paused`
as a flat sibling of `InGame`, debug keys `1`-`4`, every state as a flat
colour, the 3D camera as not yet existing, an `input/` folder, and an
`ingame/screen.rs` that was deleted. All rewritten against the code, and a
link to the removed `INGAME_FOUNDATION.md` was repointed. *Readability: a doc
that contradicts the code is worse than no doc.*

**Per-frame work pass (audit 1.4).** Key-triggered systems became run
conditions; `fly` and `look` exit before querying or doing maths when there is
no input; `AudioPlugin` and `GilrsPlugin` are disabled. *Perf: runtime — on a
typical idle frame, three systems no longer execute at all, and the camera
controls no longer query or do quaternion maths when the player is still.
Startup — no audio device is opened and no gamepad backend is initialised.
Build — none; all runtime changes, no feature edits. Modularity: `fly`'s
key-reading moved into a pure `axis()` function, which is now unit-tested
without Bevy. Verified with a 15-second run: clean startup, states and
sub-states transition, scene spawns.*

**Cargo.toml Step 2 claim corrected.** The comment said dropping the `2d`
feature removes `bevy_sprite`. It does not: `bevy_ui` depends on `bevy_sprite`
directly, so only `bevy_sprite_render` goes. Step 2 is still worth doing
alongside Step 1, but it is a small win, and the comment now says so.

**Tier 2 restructure: states grouped, camera renamed.** Every state moved
under `states/`, with the state machine in `states/mod.rs` (formerly
`app/states.rs`) and `paused/` inside `ingame/`. `camera_3d.rs` became
`camera_world.rs` (plugin `WorldCameraPlugin`). Registration now nests the
same way as the folders: `AppPlugin` adds `GameStatePlugin`, which adds the
top-level states, and `InGamePlugin` adds `PausedPlugin`. All moves used
`git mv`. *Perf: none — module reorganisation only. Readability: the top
level is now five infrastructure folders plus `states/`; the state hierarchy
is visible in the tree; both cameras are named for what they show; adding a
state touches only its parent's `mod.rs`.*

**`Loading` now exits to `Menu`.** A `finish` system gated on
`in_state(Loading)` moves on once boot loading is done — today on the first
frame. Before this, a release build (no debug keys) sat on the loading screen
forever. The wider loading design is written up in `LOADING.md` rather than
built.

**`cargo test --release` fixed.** The key-uniqueness test named the three
debug keys unconditionally, but they only exist in debug builds, so the test
failed to compile in release. The debug entries are now added under
`#[cfg(debug_assertions)]`. The bug predated this change; it surfaced when
the release lint was run with `--all-targets`.

**Tier 3: controls split from the camera.** `camera/camera_world.rs` held both
the camera entity and the player controls. The controls moved to a new
`input/` folder — `look.rs`, `movement.rs` (with `axis()` and its tests), and
`cursor.rs` — registered by `GameInputPlugin` and gated on `Playing`. The
camera keeps its marker, draw order, MSAA, start position, `LookAngles`, and
spawn/despawn; it re-exports `WorldCamera` and `LookAngles` for `input/` to
use. `input/` depends on `camera/`, never the reverse. The pitch clamp moved
with `look`, the control that enforces it. Inside `input/`, `config::input`
is imported as `controls` so a bare `input::` never reads as the module
itself. *Perf: none — the same systems with the same run conditions,
registered from a different plugin. Modularity: `camera_world.rs` went from
237 lines of two concerns to 79 of one; each control is its own file.*

**Tier 3: window split by lifecycle.** `utils/window.rs` became
`utils/window/` with `setup.rs` (once, at startup) and `toggles.rs` (every
frame, for the life of the game). The fullscreen mode both use is in
`window/mod.rs`, and re-exports kept every call site unchanged. *Perf: none.
Modularity: each file has one lifecycle.*

**Two more files doing two jobs, found by sweeping the codebase.**
`states/mod.rs` held the state machine and the debug jump keys; the keys
moved to `states/debug.rs`, compiled out of release as one module instead of
through a cfg on each item. `app/plugin.rs` held the composition root and the
engine plugin configuration; the configuration moved to `utils/engine.rs`,
beside the window and log adapters it uses, so `AppPlugin` only assembles the
game. *Perf: none. Modularity: every file now has one job.* — **Partly
reverted, see below.**

**Correction: `utils/engine.rs` folded back into `AppPlugin`, `window/`
promoted out of `utils/`.** Challenged on the reasoning above: deciding which
Bevy plugins run — swapping in our window/log plugins, disabling
`AudioPlugin`/`GilrsPlugin` — is a composition decision, indistinguishable in
kind from adding a domain plugin, not an adapter that turns our config into
one Bevy value the way `window/` and `log.rs` do. It moved back inline into
`AppPlugin::build`, next to the domain plugin tuple it belongs beside. At the
same time, `window/` moved out of `utils/` to the top level: a two-file split
by lifecycle is a domain in its own right (a sibling of `camera/` and
`input/`), not a small complete utility. `utils/` is back to holding only
`log.rs`. See the reversals table above. *Perf: none — both changes are
reorganisation only. Readability: one file lists everything the app is made
of, engine plugins included, matching what `AppPlugin`'s own doc comment
already claimed; `utils/` stays narrow enough that "does this belong in
utils?" keeps having a clear answer.*

---

### 2026-09-26

**Fixed: window returned smaller than it started after leaving fullscreen.**
Reported as: open at 1280x720, which the OS scales normally; press F11 twice
(fullscreen, then back) and the window comes back noticeably smaller, as if
OS scaling stopped applying.

Root cause, confirmed against `bevy_winit` 0.19.1 source rather than assumed:
the *same* `WIDTH`/`HEIGHT` numbers are handled by two different code paths
depending on when they're set.

- **At window creation** (`window::setup`), Bevy hands winit a `LogicalSize`
  built from `window.width()`/`height()` — and since a freshly-constructed
  `WindowResolution` defaults `scale_factor` to `1.0`, those logical values
  are numerically equal to `WIDTH`/`HEIGHT`. Winit then converts *logical to
  physical* using the monitor's real DPI. `1280` logical becomes `1920`
  physical at 150% scaling — the window looks the same size as it would
  anywhere else.
- **On a live window** (`window::toggles`, restoring from fullscreen), Bevy
  compares `resolution.physical_width()`/`physical_height()` directly and
  calls `winit_window.request_inner_size()` with them as **exact physical
  pixels** — no DPI conversion at all.

`toggle_fullscreen` was doing `window.resolution = WindowResolution::new(WIDTH, HEIGHT)`
— constructing a *fresh* `WindowResolution`, which resets `scale_factor` to
its `1.0` default and feeds `1280`/`720` into the physical-pixels path. At
150% scaling that is visually two-thirds the size the window opened at.

Fix: `window.resolution.set(WIDTH as f32, HEIGHT as f32)` instead. `.set()`
mutates the *existing* `WindowResolution` in place — so it keeps whatever
`scale_factor` Bevy has been tracking live from the OS — and internally
multiplies by that scale factor before writing the physical fields. Same
logical-to-physical conversion as window creation, just computed by us
instead of by winit. *Perf: none, a startup/toggle-time fix only.
Correctness: the window now returns to the same visual size on any display
scaling, not just 100%.*

**Corrected the doc comments this bug traces back to.** `config::window::WIDTH`/`HEIGHT`
and `window::setup`'s resolution comment both asserted "physical pixels,"
reasoning from `WindowResolution::new`'s parameter names
(`physical_width`/`physical_height`) rather than from how `bevy_winit`
actually consumes the value. That assumption was never verified against the
winit integration before being written down — this is the second time in
this project an unverified comment about engine internals turned out
backwards (the first was the `[lints]` schema diagnosis). Both comments now
state which code path applies and why, with a cross-reference between
`config::window`, `window::setup`, and `window::toggles` so the two lifetimes
of the same constants aren't read in isolation again.

**Fixed: the pause overlay read as a completely different screen.** Reported
as: pausing should feel like an overlay on the game, but it looked like a
full screen swap instead — the same complaint the overlay was specifically
designed to avoid.

Before assuming the fix was cosmetic, the camera architecture was checked
against Bevy's actual UI-camera resolution (`DefaultUiCamera::get` in
`bevy_ui`), since `InGame`/`Paused` is the first state where the UI camera
and the world camera are ever alive at once — every earlier state only ever
had one camera, so this path had never been exercised. Confirmed correct:
with neither camera marked `IsDefaultUiCamera`, Bevy's fallback picks the
camera with the highest `order` targeting the primary window, which is the
UI camera (`order: 1` vs. the world camera's `0`) by construction. UI was
never misrouted to the wrong camera.

The actual cause was the overlay colour: `Color::srgba(0.05, 0.03, 0.08, 0.75)`
is 75% opacity of near-black over a lit 3D scene. At that strength a
near-black tint reads as solid, not dimmed — technically translucent, but
visually indistinguishable from the opaque Loading/Menu screens it was meant
to be unlike. Lowered to `0.45`. *Perf: none, a colour constant only.
Correctness: ruled out the more serious possible cause (wrong camera
targeting) by reading Bevy's source rather than assuming the visible-camera
architecture just worked because it compiled — worth remembering the next
time two cameras coexist for the first time in a new state.*

**Not visually confirmed** — this environment cannot render the game.
0.45 is a reasoned starting point (a common overlay strength), not a
measured one; it may still want tuning once seen.

**Fixed, for real this time: the pause overlay that never cleared, looked
black at any alpha, and the view that looked frozen on first entering the
game.** Reported as three separate bugs: controls do not engage on the first
`Menu -> InGame` (pressing `2` then `3` "fixed" it); the pause overlay looks
pitch black instead of translucent; after resuming, the overlay stays on
screen forever while the world visibly moves behind it.

They were one bug. Root cause, read from `bevy_render` and
`bevy_core_pipeline` 0.19.1 source:

- `prepare_view_targets` gives each camera an intermediate "main texture"
  keyed on `(target, usage, format, msaa)`. The UI camera is `Msaa::Off` and
  the world camera is `Sample4`, so **they do not share one**. The UI camera
  draws into a texture the world is never in.
- That texture's colour attachment only clears on the first use per frame
  *if it has a clear colour*. `ClearColorConfig::None` gives it none, so its
  load op is always `Load`: **it was never cleared**. `TextureCache` hands
  out the same GPU texture frame after frame, so each frame's UI landed on
  top of every earlier frame's.
- The `upscaling` node then blends that texture onto the window with alpha
  blending, because it is the second camera on the window.

Every symptom follows. A 0.3-alpha overlay drawn over its own previous
frame 10 times is 97% opaque, so any alpha looked black. After despawn,
nothing overwrites the texture, so the overlay and its text stay composited
over the live world indefinitely. That is also why minimising or resizing the
window did not help: the world *was* re-rendering. And because the world
camera's own main textures have the identical descriptor (same label,
size, format, sample count 1), the two cameras draw from one `TextureCache`
pool. The UI camera can be handed a texture that last held an opaque world
frame and blend it over the live one, which looks exactly like frozen
controls while input is in fact working (the diagnostic logs showed look
and movement firing from the first frame). Leaving and re-entering the game
reshuffles which texture each camera gets, which is why `2` then `3`
appeared to fix it.

Fix, in `camera::camera_ui`:

- `clear_color: ClearColorConfig::Custom(Color::NONE)` — clear the UI's own
  texture to transparent every frame.
- `output_mode: CameraOutputMode::Write { blend_state:
  Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING), clear_color: Default }`.
  The UI pipeline alpha-blends onto a transparent texture, which leaves
  premultiplied colour, so the default straight-alpha composite would
  multiply by alpha twice and darken text edges and the overlay. The window
  clear only applies to the first camera writing each frame: the world
  camera in-game, the UI camera in menus.

Kept `Msaa::Off` on the UI camera. The alternative fix, matching MSAA so
both cameras share one texture and `None` works as intended, would bring
back the 4x UI render target that entry removed. The overlay alpha went back
up to 0.5, since the earlier "too dark" reports were accumulation, not alpha.

*Perf: one transparent clear of the UI texture per frame, a single
full-screen fill, far cheaper than the 4x MSAA target the alternative
fix needed. Correctness: fixes all three reported bugs. It also drops the
old rule that states without a world camera must paint an opaque
background; the UI camera no longer relies on it.*

**The wrong turns, recorded so they are not retried.** Each was checked and
ruled out before the real cause was found:

- **Overlay alpha too strong** (0.75 → 0.45 → 0.3). Never the cause; see
  above. The earlier entry today blaming the colour is wrong.
- **OS focus loss dropping the cursor grab**, answered with a
  `regrab_on_focus_regained` system and a `Window::focused = true` request
  after the F10/F11 toggles. Logs showed no `WindowFocused` events at all
  during the failing transition, and input arriving normally. Both removed.
- **Escape double-firing** and flipping straight back to `Paused`, answered
  with a 0.25 s debounce. Logs showed exactly one toggle per press. Removed.
- **Non-recursive despawn** and **sub-state `OnExit` not firing**, both
  disproved from Bevy source. A diagnostic system confirmed the
  `PausedScreen` entity count went `1 -> 0` on every resume.

The lesson: once logs prove the ECS state is right and the picture is still
wrong, stop changing game logic and read the render path. Here that meant
`prepare_view_targets` and the `upscaling` node, not `bevy_ui`.

**Descend moved to Left Shift; sprint is now a double-tap, not a held key.**
Requested change: descend should be Left Shift instead of Left Control, and
sprint (triple speed) should trigger by pressing any of `W`/`A`/`S`/`D` twice
in quick succession, the way Minecraft does it, rather than by holding a key.

`DOWN` took `ShiftLeft`, which is where `SPRINT` used to live, so `SPRINT`'s
dedicated keybinding is gone rather than reassigned — there is no held-key
alternative left to give it. `config::input::DOUBLE_TAP_WINDOW` (0.3s) is the
new tunable: the max gap between two presses of the *same* movement key that
counts as a double-tap. `input::movement::fly` tracks the last key tapped and
when (`Local<Option<(KeyCode, f32)>>`), and a `Local<bool>` for whether sprint
is currently engaged; a double-tap turns it on, and releasing every movement
key (the existing `intent == Vec3::ZERO` early return) turns it back off,
rather than sprint being re-decided every frame from a held key's state.

The detection logic (`is_double_tap`) is a pure function taking the tapped
key, the current time, and the last tap, so it is unit-tested without Bevy —
same pattern as `axis`. *Perf: four extra `just_pressed` lookups per frame,
only reached once a movement key is already held (the early return above
still fires first). Correctness: matches the requested double-tap behaviour
for all four movement keys, not just one.*

**Sprint trigger made a config toggle; the hold key it replaced got its own
config constant.** Follow-up request: let double-tap and "hold a key" both
exist, selected by a setting, rather than double-tap being the only option.

`config::input::SprintMode` is a two-variant enum (`DoubleTap`, `Hold`), and
`SPRINT_MODE` picks which one is live — a `const`, like every other setting
in this module, not a runtime option (there is no settings menu yet).
`SPRINT_HOLD_KEY` is `ControlLeft`: the traditional sprint modifier, and free
again now that `DOWN` sits on `ShiftLeft`. `input::movement::fly` branches on
`SPRINT_MODE` once per frame — `Hold` reads `keys.pressed(SPRINT_HOLD_KEY)`
directly, `DoubleTap` keeps the tap-tracking logic from the entry above.

Because `SPRINT_MODE` is a `const`, only the selected variant is ever
constructed, and rustc's `dead_code` lint flags the other — correctly, from
the compiler's point of view, since nothing evaluates to it. Allowed with a
comment on the enum rather than restructured around it: this is what a
compile-time (not runtime) toggle looks like, the same shape as
`#![deny(unsafe_code)]`'s local `#[allow]` in `main.rs`. Both variants were
built and clippy/test-verified locally before settling on `DoubleTap` as the
shipped default. *Perf: none — a `match` on a `const` compared to the
previous single code path. Correctness: `Hold` reproduces the pre-double-tap
behaviour exactly, just reading a different key.*

**Field of view made a config value.** Requested: a way to change FOV.
There was nothing to change — `camera_world` spawned `Camera3d::default()`
with no `Projection`, so Bevy's `#[require(Camera, Projection)]` silently
filled one in at its own default (45°, confirmed from `bevy_camera` source).
That default lived only in Bevy, not as a value this project owned.

New `config::camera` module (mirroring `config::window` and `config::input`
— `config/`'s stated shape is "adding a category means adding a file") holds
`FOV_DEGREES`. `camera_world` now spawns an explicit
`Projection::Perspective(PerspectiveProjection { fov: FOV_DEGREES.to_radians(), ..default() })`
instead of leaning on the required-component default, converting
degrees to radians at the one place that needs radians rather than storing
the setting in the unit the player would find harder to reason about.
*Perf: none, a value moved from an implicit engine default to an explicit
one set once at spawn. Reading: FOV is now visible in the settings surface
alongside every other tunable, instead of needing to be known as "a Bevy
default" to find.*

**`world/` built: chunk coordinates, storage, and a placeholder generator.**
First real slice of the voxel world, per the target shape `CLAUDE.md` already
described. Scoped deliberately to *data* only — no meshing, no rendering, no
real terrain — because each of those is its own sizable piece of work and
`render/` is documented as a separate domain from `world/` for exactly this
reason: mixing "what blocks exist" with "how they reach the screen" is what
the split is meant to prevent from the start, not something to allow once and
clean up later.

`config::world` (`CHUNK_SIZE = 32`, `RENDER_DISTANCE = 0`) already existed
before this change; it only needed wiring into `config/mod.rs`. Added on top:
`ChunkPos` (a chunk coordinate, kept distinct from a block coordinate so the
two can't be passed to the wrong place), `Chunk` (a flat `Vec<Block>`
indexed by a computed offset — not nested `Vec`s, since meshing will walk
every block in every loaded chunk), a two-variant `Block` (`Air`/`Solid`),
and `generation::generate`, a pure `ChunkPos -> Chunk` function producing a
flat placeholder floor (solid in the bottom half, identical at every
position) rather than real terrain. `World`, a resource holding loaded
chunks in a `HashMap<ChunkPos, Chunk>`, and `load_chunks_around_player`, a
system gated on `GameState::InGame` that loops over every position within
`RENDER_DISTANCE` of the player's chunk (evaluating to just the one chunk at
the configured `0`) and generates whichever aren't loaded yet.

The two sea-level constants stay commented out in `config::world` exactly as
handed over — real generation needs a heightmap concept that doesn't exist
yet, so reading them now would mean pretending they mean something they
don't. `RENDER_DISTANCE`, unlike them, is live: the loop is written over the
actual configured radius rather than special-cased to "just this chunk", so
raising it later is a config edit, not a code change.

Left alone on purpose: `scene.rs`'s placeholder cubes, still the only thing
actually visible in `InGame` — they say in their own doc comment that they
are "replaced wholesale by the voxel world", and that replacement is a
rendering-layer change (`render/`), not this one. `[profile.dev]`'s
`opt-level = 0` for our own crate is also untouched, per the standing note in
`Cargo.toml` and `AUDIT.md` 1.3 to raise it once generation is a real,
measured cost rather than 32,768 flat writes per chunk at `RENDER_DISTANCE`
`0` — not yet.

*Perf: generation is a nested loop over one chunk's blocks (`CHUNK_SIZE^3` =
32,768 writes), run once per newly-entered chunk, not per frame — negligible
even unoptimised. Storage is one `HashMap` lookup per frame while `InGame`,
gated by a run condition so it costs nothing in any other state. Modularity:
`world/` does not import anything rendering-related, so `render/` can be
built against it later without either module needing to change shape.*

**Audit #2.** A full scan of the code, docs, manifest and dependency tree
(method and checklist at the bottom of `AUDIT.md`). `AUDIT.md` was rewritten
from scratch with 21 open items; audit #1's items were all either done and
already recorded here, or carried forward with their current status. No
code changed in this pass. What changed in this file:

- **Standing decisions rebuilt and grouped by area.** Seven rows were stale.
  They named files that no longer exist (`app/log.rs`, `window.rs`,
  `config.rs`), described "one persistent camera" when there are two, or
  contradicted each other (two different definitions of `utils/`, and two
  rows for debug gating). All were corrected in place. The old `utils/`
  definition moved to Reversals.
- **Decisions from the last two sessions added as rows:** Left Shift descends;
  the two sprint modes and their compile-time toggle; the double-tap rules;
  FOV in degrees as an explicit `Projection`; the logical-pixel `.set()`
  rule for live window resizing; the UI/world camera MSAA split and draw
  order; the overlay alpha; every `world/` design choice (data-only scope,
  flat storage, `ChunkPos` flooring, a resource rather than entities, the
  horizontal render-distance square, commented-out sea-level constants,
  pure generation); and the rule that audits are periodic, with decisions
  landing here.
- **Reversals added:** the three diagnostic-era additions that were removed
  (focus re-grab, focus request, Escape debounce), the overlay alpha
  sequence, the live-resize fix, the control rebinding, and the
  `[lints.rust]` move, which audit #2 found was only half done.
- **"Known open items" removed.** It was stale in three of its four bullets:
  "the repository has no commits", "nothing has been visually verified", and
  a renamed function. It also duplicated what `AUDIT.md` tracks. Open items
  now live in one place.

Two claims made by earlier work were found to be false, and are corrected
in `AUDIT.md` rather than here, since they are open items:

- Audit #1 said `.cargo/config.toml` existed with staged linker settings.
  Git has never tracked it (1.3).
- The `unsafe_code` lint was said to have "moved to `main.rs`". The
  `Cargo.toml` table was never deleted (2.4).

The lesson for the next audit is to check claims about files against git,
not against the log.

### 2026-09-27

**Chunks render: `render/` built, `LoadedChunks` streams in both
directions, and the placeholder scene is gone.** First working slice of
"a world where multiple chunks can spawn and despawn" — until now `world/`
only ever generated data, nothing was ever unloaded, and nothing was drawn.

**Render distance is now a circle, not the bounding square.** Discussed
before building it: a square radius includes corners `sqrt(2)` chunks away,
noticeably farther than the radius implies (about 27% more chunks for the
same nominal distance), and pops chunks in at an inconsistent distance
depending on the direction of travel — most games use a circle for exactly
this reason. `in_render_distance(pos, center, radius)` is the one function
both the load loop and the new eviction step call, so they can't disagree
about a chunk sitting at the boundary. Five unit tests pin the shape down,
including the specific corner case a square would get wrong. At the
configured `RENDER_DISTANCE = 0` a circle and a square are identical (both
are just the center chunk), so this cost nothing to get right immediately
rather than reworking it once `RENDER_DISTANCE` actually changes shape.

**Chunks now unload, not just load.** `load_chunks_around_player` evicts
every loaded chunk that `in_render_distance` no longer accepts, before
loading whatever's newly in range, so the two steps never leave both an old
and new chunk in the same slot. This is also what the "despawn" half of
"spawn and despawn" needed at the data level — previously walking between
chunks only ever grew `LoadedChunks`, never shrank it (audit #2, 3.2).
Leaving `GameState::InGame` still resets the whole map in one step rather
than evicting chunk-by-chunk, since re-entering should generate fresh.

**`world::World` renamed to `LoadedChunks`.** It shadowed Bevy's own ECS
`World` type in a file that glob-imports `bevy::prelude::*` — flagged in
audit #2 (2.3), fixed now because `render/` needing to reference it made
the collision risk real rather than theoretical.

**`ChunkLoaded`/`ChunkUnloaded` messages connect `world/` and `render/`.**
`render/` needed to know when a chunk's data changed without polling
`LoadedChunks` every frame regardless of whether anything did. `world/`
fires a message on each load and each eviction; `render/` only touches the
ECS on a frame something actually happened. `world/` has no idea `render/`
exists — it just declares message types and writes to them — keeping the
dependency one-way, `render/` -> `world/`.

**New `render/` module: `mesh.rs`, `material.rs`, `mod.rs`.** `mesh.rs`
builds a `Mesh` from a `&Chunk` in the chunk's own local space, only
emitting a face where the neighbouring block isn't solid — a naive mesher
emitting all six faces of every solid block would be roughly 390,000
vertices for a half-solid chunk, almost all of them faces buried against a
neighbour the camera can never see. This is face culling only, not full
greedy meshing (merging coplanar faces into larger quads); that's a further
optimisation for later. A chunk edge always counts as exposed, since
neighbouring chunks aren't consulted yet — harmless at `RENDER_DISTANCE = 0`
(there's only ever one chunk), and flagged for when that changes.
`material.rs` builds one shared green `StandardMaterial`, reused by every
chunk mesh — a texture atlas replaces it once there's more than one visible
block type. `mod.rs` reacts to the two messages: `spawn_chunk_meshes` meshes
a newly-loaded chunk and spawns it (`Mesh3d`, the shared material, a
`Transform` at `ChunkPos::origin`, and the `ChunkPos` itself as a
component); `despawn_chunk_meshes` looks the entity up in a
`HashMap<ChunkPos, Entity>` and removes it. Leaving `GameState::InGame`
clears every chunk entity in one pass, independent of `ChunkUnloaded`,
since `world/`'s exit-time reset doesn't message per chunk either.

**`ChunkPos` widened to a `Component`, and `Chunk::empty`/`set_block`
widened to `pub(crate)`.** The former tags each mesh entity with the chunk
it renders. The latter let `render/mesh`'s tests build chunks by hand to
exercise face culling directly (an isolated block gets all six faces; two
adjacent blocks cull the one face between them; an empty chunk produces no
geometry) without going through generation — `Chunk::block` had already
lost its `#[allow(dead_code)]` the moment the mesher became a real caller
(audit #2, 4.3).

**`ingame/scene.rs` deleted.** The six placeholder cubes are gone outright,
as requested — real chunk meshes replace them, exactly what the module's
own doc comment said would happen. Its `DirectionalLight` moved to
`render/mod.rs` instead of disappearing with the rest: chunk meshes need a
light to be visible at all, and lighting the world is the same category of
job as giving it a material, not a leftover of the old scene.

**World camera spawn height raised, `8, 6, 16` to `8, 22, 16`.** Audit #2
(4.1) flagged that the old height was inside the generated ground —
harmless while nothing rendered, but the first thing anyone would have hit
the moment chunk meshes existed. Fixed alongside rather than left as a
"why am I underground" bug to debug separately. Still a constant, not
derived from the world — that's real spawn-point logic for later, once
terrain isn't a flat placeholder.

*Perf: eviction and loading are each one `HashMap` operation per candidate
chunk per frame while `InGame`, same order of cost as before. Meshing runs
once per `ChunkLoaded`, not per frame — a few thousand vertices for the one
currently-loaded chunk, negligible even unoptimised. Modularity: `render/`
depends on `world/` and nothing else; `world/` still doesn't know `render/`
exists. Correctness: verified with 24 tests (11 new — 5 for the circular
distance, 3 for face culling, and existing chunk/generation coverage
unchanged), a clean `cargo clippy -- -D warnings`, and a boot run confirming
no panic on startup. Not driven into `InGame` interactively — this
environment can't send keypresses to the running window — so the actual
on-screen result (does the chunk look right, is the winding correct so
faces don't render inside-out) still needs your eyes.*

**Confirmed working, then: `RENDER_DISTANCE` raised to `1`, and a chunk-lock
testing toggle added.** With rendering now in and confirmed, `RENDER_DISTANCE`
moved from `0` to `1` — a 5-chunk plus shape (the centre and its four
straight-line neighbours; the diagonal corners a bounding square would
include stay excluded, per the circle decision above). Requested alongside:
a way to freeze chunk streaming for testing, so a fixed set of chunks can be
inspected without new ones loading or old ones unloading as the camera moves.

New `world::ChunkLock` (a plain `bool` resource, unconditional in every
build) gates `load_chunks_around_player` through a `chunk_loading_unlocked`
run condition — locked, the system doesn't run at all, so whatever was
loaded the moment it engaged simply stays. Only *toggling* the lock is
debug-only: `world::debug` (new, `#[cfg(debug_assertions)]`) registers an
`F9` hotkey (`config::input::TOGGLE_CHUNK_LOCK`, added to the existing debug
key section) that flips it, but only if the new
`config::debug::TESTING_TOOLS_ENABLED` is also `true`.

That's two independent gates on purpose, not redundancy: `#[cfg(debug_assertions)]`
means a testing feature cannot exist in a release binary, structurally,
matching `states::debug`'s standing decision for the jump keys.
`TESTING_TOOLS_ENABLED` means a *debug* build doesn't default to every
testing feature being live either — being a debug build isn't the same as
actively testing right now, and the fewer things a stray keypress can do
during ordinary dev play, the fewer surprises. This is meant as the template
for every testing feature the user mentioned more of coming: state always
exists and reads the same everywhere; only the code that *changes* that
state is gated, twice.

*Perf: one extra `bool` resource read per frame while `InGame` (the run
condition), zero cost in release (the toggle path doesn't exist to even
check). Modularity: `world/debug.rs` mirrors `states/debug.rs` exactly —
same cfg, same "delete once no longer needed" shape — so a reader who
already knows one recognises the other. Correctness: `cargo check`,
`cargo clippy -- -D warnings`, `cargo test` (24 pass, unchanged — no new
pure logic here worth a unit test beyond what a boot-and-press-F9 check
covers) and a boot run all clean. Not confirmed by eye: does `F9` actually
freeze the display; that needs the game running interactively, which this
environment can't do.*

**Chunk lock can now start pre-engaged.** Requested: a way to have the world
start locked without pressing `F9` first. `config::debug::CHUNK_LOCK_INITIALLY_ENGAGED`
(debug builds only) drives `ChunkLock`'s `Default` impl (previously derived,
always `false`; now hand-written).

Guarded by `TESTING_TOOLS_ENABLED` as well as its own value — not because it
was asked for, but because without that guard, setting only
`CHUNK_LOCK_INITIALLY_ENGAGED = true` while leaving `TESTING_TOOLS_ENABLED`
at its default `false` would start the world locked with **no way to unlock
it**, since the `F9` hotkey itself is only registered when testing tools are
enabled. Both constants now have to agree for the world to start locked. In
release, `ChunkLock` still always defaults to `false` — a separate `cfg`'d
constant, not a runtime check of the debug-only ones, since those don't
exist to check in a release compile. *Perf: none, a startup value. Modularity:
none, a one-line addition to the pattern the previous entry already set.*

**Audit #2's tier 1, worked through one item at a time.** 1.1 and 1.2 done;
1.3's file restored (staged, not enabled); 1.4, 1.5, and 1.6 explicitly
deferred rather than left as generic "carry over" — see `AUDIT.md` for the
current text of each. Recorded here is what changed in code for 1.1 and 1.2.

**1.1 — chunk storage and generation now agree on block order.**
`Chunk::index` stored blocks with `z` slowest; `generation::generate` looped
with `y` innermost — every write during generation jumped 32 slots ahead
instead of landing on the next one, and did it once per block through a
bounds-checked `set_block`. Changed the formula to `x + z*S + y*S²` ("YZX",
the layout Minecraft uses): now every block sharing a height is contiguous
in memory, since terrain is naturally described that way ("solid up to this
height"). `generation::generate` dropped its triple-nested loop entirely in
favour of a new `Chunk::fill_below_height`, which fills the whole placeholder
floor as one slice fill. `set_block` lost its only real caller in the
process — kept for future per-block edits (block placing/breaking, a
non-flat heightmap), `#[allow(dead_code)]` rather than `#[expect]` because
it's already called from this file's own tests and `render::mesh`'s: the
lint fires in a plain `cargo clippy` but not in `cargo test`, where those
callers exist, and `#[expect]` demands the lint fire in *every* build it's
compiled into — it doesn't, and using it produced exactly the "unfulfilled
expectation" warning that predicts, caught and reverted to `#[allow]` in the
same pass. A new test pins `x`-fastest/`z`-next/`y`-slowest directly, so a
future change to the formula fails a test immediately rather than silently
degrading whatever reads the layout next.

**1.2 — chunk loading now costs nothing when the player hasn't moved.**
`load_chunks_around_player` ran every frame `GameState::InGame` was active,
including all of `Paused` and every frame spent standing still, doing a
query, a float-to-chunk-coordinate conversion, and a `HashMap` lookup per
candidate chunk regardless of whether anything could have changed. Added
`Changed<Transform>` to the query. While `Paused`, `look`/`fly` (the only
systems that ever write to the camera's `Transform`) are already gated to
`Playing`, so the `Transform` genuinely never changes there — the query
comes back empty and the system does nothing. Standing still gets the same
treatment. Looking around without moving still triggers a re-check (`Changed`
tracks the whole component, not just translation), which is a coarser
saving than a hand-rolled "did the chunk actually change" comparison would
give, but it's the built-in query filter `CLAUDE.md` names first, applied
exactly as the audit recommended.

*Perf: 1.1 turns a 16,384-iteration bounds-checked loop into one slice fill
per chunk generated — negligible at today's scale (one chunk, once) but the
kind of thing that compounds once generation runs constantly during
streaming. 1.2 skips the entire system, not just its body, on any frame the
camera didn't move — previously guaranteed cost every frame while `InGame`,
now zero on the common "standing still" and "paused" frames. Correctness:
`cargo check`, `cargo clippy -- -D warnings`, and `cargo test` (27 pass, 3
new — the index-formula pin, `fill_below_height`'s exact range, and its
out-of-range panic) all clean from a fresh `cargo clean -p openmc_b`, plus a
boot run confirming no panic on startup.*

**1.3 and 1.4 acted on — Windows linker enabled, Bevy features trimmed,
`noise` added.** All three requested explicitly, and made **without
building to confirm any of them** — also requested explicitly.

`.cargo/config.toml`: uncommented the Windows block, so `rust-lld.exe` is
now the active linker for this project rather than MSVC's default. Verified
`rust-lld.exe` actually exists on this machine's toolchain by checking the
filesystem (`<toolchain>/lib/rustlib/x86_64-pc-windows-msvc/bin/`) before
enabling it, so it will at least be *found* — whether linking with it
succeeds is the thing a build would confirm. Linux and macOS blocks stay
commented, as reference, not because they're wrong, but because this
project only targets Windows right now and enabling them would be enabling
something nobody can test.

`Cargo.toml`: `bevy`'s feature list changed from the implicit default
(`["2d", "3d", "ui", "audio"]`) to an explicit `default-features = false,
features = ["3d", "ui"]` — audit #2's Step 1 (drop `audio`) and Step 2 (drop
`2d`) together, both already fully staged and reasoned through the
dependency graph in that audit entry. Step 3 (dropping glTF, since this game
loads no models) was deliberately *not* applied: it requires hand-listing
every `3d` sub-feature this project actually needs (`bevy_pbr`,
`bevy_core_pipeline`, `bevy_render`, `bevy_anti_alias`, ...), and getting
that list right on the first try with no build available to catch a missing
one is a real way to hand someone a broken build. Also added `noise = "0.9"`
— checked directly against the crates.io index rather than guessed, since
that's the current, non-yanked latest — ahead of any code reading it, in the
same pass as the feature trim specifically so one rebuild covers both
changes instead of two separate ones later.

**This is the one entry in this log where "verified" is not part of the
perf/correctness note**, because none of the usual checks were run:

*Perf: expected to shrink the compiled binary and, if `rust-lld` genuinely
helps here, shorten every subsequent incremental build — neither measured.
Modularity: none, dependency configuration only. Correctness:
**unconfirmed.** `cargo check`/`clippy`/`test`/`build` were not run for this
change, at explicit request — three things need a real build to know for
certain: that the UI still renders correctly without the `2d` feature, that
`noise` resolves and compiles cleanly against this dependency set, and that
linking with `rust-lld.exe` actually succeeds rather than merely being
found on disk. `docs/AUDIT.md` 1.7 exists specifically to make sure that
build happens, and happens before anything else changes, so if something
breaks it's isolated to these three edits.*

**That build happened, and it found exactly one break.** Dropping the
`audio` feature removed `bevy::audio::AudioPlugin` from existence, and
`app::plugin::AppPlugin` still had `.disable::<bevy::audio::AudioPlugin>()`
in its `DefaultPlugins` chain — the same line whose own comment predicted
this exact failure ahead of time ("when the `audio` feature is later
dropped ... `AudioPlugin` stops existing and the line below stops
compiling. Delete it at the same time"). It wasn't deleted in the same pass
because that pass was explicitly code-only, no build, so nothing surfaced it
until rust-analyzer did. Fixed: the line is gone, and the surrounding
comment now explains that audio is disabled the *stronger* way — a Cargo
feature removed, so `bevy_audio` isn't even in the dependency tree — rather
than the runtime-disable Gilrs still uses (which stays, since Bevy doesn't
gate the gamepad backend behind a feature at all).

With that fixed, the deferred verification ran: `cargo check` (fresh, ~3
minutes — the full rebuild every dependency-graph or linker change causes,
exactly as flagged), `cargo fmt`, `cargo clippy -- -D warnings`, `cargo
build` (confirmed `target/debug/openmc_b.exe` freshly linked — the actual
test of `rust-lld.exe`, since `check`/`clippy` don't invoke the linker for a
binary crate), `cargo test` (27 pass, unchanged), and a boot run reaching
`Menu` with no panics or render errors, confirming the UI still draws
without the `2d` feature. All clean. `docs/AUDIT.md` 1.7 is resolved and
removed — the three 2026-09-27 dependency/build changes are now confirmed
working, not just applied.

**Real terrain generation, and the mesher swapped for `binary_greedy_meshing`.**
Two requested changes landed together, since both touch the same two files
(`world::generation` and `render::mesh`) and the second depends on data the
first produces.

**Terrain.** `world::generation::generate` no longer fills a flat floor. Each
`(x, z)` column now gets its own height from `noise::Fbm<Perlin>` — several
octaves of Perlin noise summed together (`config::world::TERRAIN_OCTAVES`,
`_FREQUENCY`, `_PERSISTENCE` tune the shape; `_SEED` and `_AMPLITUDE` the
identity and range). Sampled in *world* space (`ChunkPos::origin() + local`),
not chunk-local space, so neighbouring chunks' terrain lines up at the seam
instead of every chunk looking like its own island. Every height is clamped
to `0..CHUNK_SIZE`: this is still bounded by the single vertical layer of
chunks `load_chunks_around_player` produces, not the full world height the
commented-out sea-level constants describe — that's real, separate work for
when multiple vertical layers exist, deliberately out of scope here (as
discussed: "we are just generating the top level chunks... good for now").
The audit #2 1.1 contiguous-fill optimisation stays meaningfully alive
despite terrain no longer being flat: since no column can go below
`base_height - amplitude`, that whole band is bulk-filled with
`Chunk::fill_below_height` in one slice fill, and only the (much smaller)
band a column's actual height might land in is set block-by-block. Four
tests cover determinism, that height actually varies, that it never leaves
the chunk, and that different positions produce different terrain.

**Meshing.** `render::mesh` no longer hand-rolls face culling. It now uses
[`binary_greedy_meshing`](https://github.com/Inspirateur/binary-greedy-meshing),
a maintained, MIT-licensed Rust port of the reference algorithm at
[cgerikj/binary-greedy-meshing](https://github.com/cgerikj/binary-greedy-meshing)
— the repository named for this work — used as a real dependency rather
than a hand-port of the C++ or a hand-rolled equivalent, per
`CLAUDE.md`'s own standing preference for adopting a proven implementation
over writing new code that meets the same need. It's generic over chunk size
(a `const` parameter), so it works with our `CHUNK_SIZE` directly rather
than requiring its demo's fixed 62. Where the old mesher only culled a face
against a solid block *within the same chunk*, this one doesn't just cull —
it *merges* adjacent same-type faces into the largest quad it can, using
bitwise operations to do it fast (a flat 32×32 floor becomes one quad
instead of 1,024).

Using it properly needs the input **padded**: a chunk's own blocks in the
middle, one block of neighbouring data on every side, so the mesher can tell
whether a boundary face is hidden by whatever's next door.
`write_neighbor_padding` fills that padding from `LoadedChunks` wherever a
face-adjacent neighbour happens to be loaded — real cross-chunk face
culling, closing the gap flagged in `docs/AUDIT.md` 1.6 (a chunk edge always
counted as exposed, which had gone from theoretical to an active cost the
moment `RENDER_DISTANCE` rose above `0`). Confirmed from the crate's own
source, not assumed: only the six face-adjacent neighbours are ever read,
since its face culling never looks at a cell's diagonal neighbour, so
reading diagonal chunk data would be pure waste.

Padding alone isn't enough, though: a neighbour meshed *before* this chunk
existed was built assuming this side was open air, and nothing would ever
tell it otherwise. `render::spawn_chunk_meshes` now also re-meshes every
already-spawned face neighbour of a chunk that just loaded, using the new
`ChunkPos::face_neighbors`. Three tests cover an empty chunk producing no
geometry, a fully solid chunk with no neighbours merging into exactly six
quads total (proof the merging is real — a face-culled-only mesher would
still emit far more), and a solid neighbour on one side removing exactly
that one face's quad.

*Perf: generation's cost per chunk is unchanged in shape (still one pass
over the chunk, now with a noise sample per column instead of none) and
still runs once per `ChunkLoaded`, not per frame. Meshing trades the old
mesher's face-culled-but-unmerged output for real merged quads — far fewer
vertices per chunk for anything with large flat regions, which most terrain
is — at the cost of also re-meshing up to 6 neighbours per newly-loaded
chunk, bounded and infrequent (only on `ChunkLoaded`, not per frame).
Modularity: `render/` still depends only on `world/`; `world/` still has no
idea `render/` exists. Correctness: 29 tests pass (5 new for terrain, 2 new
mesher tests replacing the 3 old ones), `cargo clippy -- -D warnings` clean,
a boot run confirms no panic on startup. Not confirmed by eye: the actual
in-game look of the terrain, and whether a real multi-chunk seam (at
`RENDER_DISTANCE = 2`, loading and re-meshing several chunks as the player
moves) is actually invisible — the unit tests prove the mechanism works in
isolation, but this environment can't drive the game interactively to see
the assembled result.*

**Chunk generation moved to a background thread (audit #2, 1.6).**
`load_chunks_around_player` no longer calls `generation::generate` inline —
it decides what's wanted (unchanged: eviction, then finding new positions in
range) and spawns a `bevy::tasks::AsyncComputeTaskPool` task for each
newly-wanted position, tracked in a new `PendingChunks` resource
(`HashMap<ChunkPos, Task<Chunk>>`, the same shape as `LoadedChunks` itself).
A new system, `apply_generated_chunks`, polls every pending task once per
frame (`block_on(poll_once(task))`) and, for whichever have finished,
removes them from `PendingChunks`, inserts them into `LoadedChunks`, and
fires `ChunkLoaded` — exactly the same message, at exactly the same point in
the pipeline, so `render/` needed zero changes; it never knew or cared where
the chunk data came from.

The two systems have deliberately different run conditions.
`load_chunks_around_player` keeps its existing
`in_state(InGame) + chunk_loading_unlocked + Changed<Transform>` gate — it's
still about *deciding what's wanted*, which still can't change if the player
hasn't moved. `apply_generated_chunks` only requires `in_state(InGame)`:
a background task can finish on any frame, including one where the player
is standing still, so it can't be gated on movement the way the request
side can. It also isn't gated on `chunk_loading_unlocked` — locking stops
*new* work from being requested, but work already dispatched should still
land when it finishes rather than being stranded.

Cancellation matters as much as spawning here. `PendingChunks` is `retain`-ed
by the same `in_render_distance` check `LoadedChunks` already used for
eviction, in the same system, on the same frame — dropping a `Task` cancels
it, so a chunk the player has already left doesn't keep generating for
nothing. `clear_loaded_chunks` (on leaving `GameState::InGame`) now clears
`PendingChunks` too, not just `LoadedChunks` — without that, a task started
in one visit to `InGame` could finish after a *later* visit had already
begun, and insert into that new session's fresh `LoadedChunks` as if it
belonged there.

`generation::generate` needed no changes at all to make this work — it was
already a pure function with no ECS access, exactly the property that was
called out when it was first written as what would make this possible.

*Perf: chunk generation (a noise sample per column plus a bulk fill) no
longer blocks the frame that requests it — up to `RENDER_DISTANCE`'s full
set of newly-wanted chunks can be in flight on background threads
simultaneously instead of generating one after another on the main thread.
Modularity: `render/` required no changes — it reacts to `ChunkLoaded`
exactly as before, indifferent to where the chunk came from. Correctness:
`cargo check`, `clippy -- -D warnings`, and `test` (29 pass, unchanged — this
is ECS/async plumbing, not new pure logic; the kind of thing an integration
test that builds a real `App` would cover, which is `AUDIT.md` 4.2's
still-open gap, not something added here) all clean, plus a boot run
confirming no panic on startup. Not confirmed by eye: that chunks actually
still appear correctly once generated asynchronously — this environment
can't drive the game interactively to watch the "queuing" -> "generated"
log sequence play out or see the result on screen.*

**Meshing itself is still synchronous** — only generation moved this round,
matching what was actually asked for. See `AUDIT.md` 1.6a for the shape a
future async-meshing pass would need (the mesh build itself can move to a
task the same way; spawning/updating the entity can't, and has to stay in a
polling system on the main thread).

### 2026-09-28

**Vertical chunk loading ("Option A"), and four visual debug features
requested alongside it.**

**Vertical loading.** `CHUNKS_ABOVE_SEA_LEVEL`/`CHUNKS_BELOW_SEA_LEVEL`
(`config::world`) went from commented-out and unread to active, each set to
`1` for now (target `20`/`12` once loading at this shape is confirmed
solid). Two designs were on the table for how a column's height interacts
with the player's own vertical position: a radius around the player (load
only nearby layers, the same shape `RENDER_DISTANCE` already uses
horizontally) or a fixed extent per column (every in-range column loads its
whole height regardless of where the player stands). The fixed extent —
"Option A" — was chosen: the sea-level constants describe a property of the
*world* (how tall it is), not of the player, so a vertical radius would make
the loaded shape depend on wherever the player currently happens to be,
contradicting that. `in_render_distance` gained `above`/`below` parameters
and now checks a new `in_vertical_range(y, above, below)` alongside its
existing horizontal circle; `load_chunks_around_player`'s loop gained a
third, innermost loop over every `y` in that vertical extent for each
in-range `(dx, dz)` column. Both `LoadedChunks`'s and `PendingChunks`'s
eviction, and the load loop itself, all still funnel through the one
`in_render_distance` function, so the "no two rules can disagree at the
boundary" property from 2026-09-27 holds for the vertical axis too.

`world::generation::generate`'s doc comment and trivial-chunk shortcuts
needed re-framing to match: the terrain band (`TERRAIN_AMPLITUDE` around
absolute world height `0`) was always independent of how many vertical
layers get loaded, but the code and its comments previously talked about it
relative to "the one vertical layer `load_chunks_around_player` produces" —
language written when only one layer of chunks ever existed. Nothing in
`generate` itself needed to change: the trivial-chunk shortcuts (solid below
the band, air above it, bulk-fill the guaranteed-solid part of an
overlapping chunk) were already expressed in absolute chunk-position terms,
which is exactly what makes them keep working — and get more valuable, not
less — once `CHUNKS_ABOVE_SEA_LEVEL`/`_BELOW_SEA_LEVEL` are raised: most
chunks in a tall world will sit entirely outside the ±10-block terrain band
and cost nothing to generate. `camera_world::START_POSITION`'s `y` moved
from `22.0` to `15.0` to match — the old value was tuned for the previous
flat-floor placeholder terrain, not the new sea-level-centred band.

Three new `generation` tests cover the trivial-chunk shortcuts directly
(fully above the band is all air, fully below is all solid, a chunk
overlapping the band's lower edge has both ground and air); one test from
2026-09-27 was removed (`a_different_vertical_layer_is_never_in_range` — its
premise, that `y != center.y` is never in range, is exactly what Option A
makes false) and four were added to `world::mod` covering the new vertical
semantics (independence from the player's own height, the sea-level
boundary itself, the extent's edges, and that it scales with `above`/
`below`). One test needed a mid-review fix: the first draft of "a chunk
below sea level still has both ground and air" sampled only a single row
(`z = 0`) across the chunk's 32-block `x` span — at `TERRAIN_FREQUENCY =
0.01`, a 32-block line covers only ~0.32 noise cycles, so it's entirely
possible (and, here, actually happened) for that one row to stay on one
side of zero without ever crossing it, even though the noise field as a
whole genuinely varies (already proven by a separate test). Broadened to
scan the full 32×32 face, matching the pattern `every_column_height_stays_
within_the_chunk` already used, rather than guessing at coordinates that
happened to cross zero.

**Debug features.** Four testing-only visual aids were requested to go
with vertical loading, all following the exact `TESTING_TOOLS_ENABLED` +
`#[cfg(debug_assertions)]` shape `world::debug` established:

- **Wireframe toggle (`F8`)**, to visually confirm the greedy mesher is
  actually merging faces rather than drawing them individually. Needs GPU
  features (`POLYGON_MODE_LINE`, `IMMEDIATES`) requested at startup — added
  to `app::plugin`'s `DefaultPlugins` via a custom `RenderPlugin`/
  `WgpuSettings`, debug builds only, since release never needs wireframes.
  Confirmed from `bevy_pbr::wireframe`'s own source that an adapter lacking
  these features gets a warning and a no-op, not a crash.
- **A three-mode chunk-bounds grid (`F7` to cycle: none, outline, or outline
  plus centre axes)**, drawn with `bevy_gizmos` around whichever chunk the
  camera is in. `ChunkGridMode` (the enum) and `CHUNK_GRID_INITIAL_MODE`
  (its starting value) both live in `config::debug`, mirroring
  `config::input::SprintMode` — settings live in `config`, behaviour in the
  module that owns the feature.
- **A lock for that grid (`Alt+F7`)**, freezing it on its current chunk
  instead of following the camera — the same idea as `world::ChunkLock`
  freezing chunk loading. Shares its physical key with the plain cycle
  action rather than spending a second key, since they're the same gesture
  at two commitment levels; the two systems reading `TOGGLE_CHUNK_GRID`
  guard against each other with `input_pressed(DEBUG_MODIFIER)` /
  `not(...)` so a single keypress only ever fires one of them. Its starting
  state (`CHUNK_GRID_INITIALLY_LOCKED`) can't be a plain `Default` impl —
  locking needs an actual camera position, and none exists at resource-
  construction time — so it's applied by a system that runs every frame
  while `InGame`, does nothing until a camera exists to read a position
  from, and then applies the lock exactly once via a `Local<bool>` guard.
- **A sea-level marker (`F6`)**, two long lines at absolute world height `0`
  crossing under the camera, to see at a glance where
  `CHUNKS_BELOW_SEA_LEVEL` actually starts without reading a coordinate.

All four hotkeys were added to `config::input`'s hand-maintained binding
list (and its no-two-actions-share-a-key test), plus a new `DEBUG_MODIFIER`
(`Left Alt`) for the lock combination — a dedicated modifier rather than
reusing `SPRINT_HOLD_KEY` (`Left Ctrl`), since this is an unrelated debug
gesture, not a second meaning for a gameplay key.

*Perf: vertical loading multiplies chunk count by
`CHUNKS_ABOVE_SEA_LEVEL + CHUNKS_BELOW_SEA_LEVEL` (currently `2`, so no
change from before at `1`/`1`); the trivial-chunk shortcuts in `generate`
are what keeps this affordable once that reaches `20`/`12`. The four debug
features cost nothing when `TESTING_TOOLS_ENABLED` is `false` — none of
their systems are added to the schedule at all. Readability: sea level's
vertical extent is now visible on screen instead of only in a doc comment.
Modularity: `render::debug` depends on `world::ChunkPos` and
`config::debug`/`config::world`/`config::input`, the same dependency shape
every other debug module in the project already has; no new dependency
direction introduced. Correctness: `cargo check`, `clippy -- -D warnings`,
and `test` (35 pass: +6 net — 3 new in `generation`, 4 new and 1 removed in
`world::mod`, unchanged elsewhere) all clean. Not confirmed by eye: whether
the vertical extent actually renders correctly at `1`/`1` (a full column of
2 chunk-layers loading and meshing without gaps or seam artifacts at the
new sea-level boundary), and whether any of the four hotkeys behave as
intended in an actual running window — this environment cannot drive the
game interactively to check either.*

### 2026-09-28 (2)

**First real play session against the 2026-09-28 batch above**, and three
fixes from what it turned up: `bevy_render`'s mesh slab allocator logging
`Use-after-free` errors during large loading bursts, and two of the four new
debug visuals not reading as intended once actually seen on screen — the
kind of feedback this environment's non-interactive boot checks can never
surface, so worth recording as a distinct source of truth from "the tests
pass."

**Mesh slab allocator errors.** The play log showed
`bevy_render::slab_allocator: Use-after-free: attempted to copy element data
for an unallocated key`, repeatedly, immediately after each burst of newly
generated chunks. Root cause: `render::spawn_chunk_meshes`'s neighbour
re-mesh step (added 2026-09-27 for cross-chunk face culling) re-meshed a
neighbour once per newly-loaded chunk that touched it, with no
deduplication — and vertical loading can land a large batch in one frame
(13 columns × 2 vertical layers = 26 chunks at the current `1`/`1` sea-level
settings, all visible in the log's `queuing chunk` lines). Two ways this
produced redundant work: a neighbour shared by several newly-loaded chunks
in the same batch got rebuilt once per toucher instead of once, and a
neighbour that was *itself* part of the batch got rebuilt again even though
its own spawn already produced a fully neighbour-aware mesh (every chunk in
a batch is already inserted into `LoadedChunks` by the time meshing runs, so
order within the batch doesn't affect correctness — only how many times the
same mesh got rebuilt). Each redundant rebuild replaces a `Mesh3d` handle,
dropping the previous one, all within the same frame — same-frame
add-then-drop churn is the known trigger shape for this class of Bevy
renderer bug.

Fixed by computing the full set of newly-spawned positions first, then
deriving the set of neighbours to re-mesh as "every face-neighbour of a
spawned chunk that is not itself spawned this batch," via a `HashSet` —
each unique neighbour is now re-meshed at most once per frame, regardless of
how many newly-loaded chunks touch it. For the initial full-world load
burst this eliminates nearly all neighbour re-meshing outright (an interior
chunk's neighbours are almost always also in the same batch); for a
steady-state load/unload cycle it still correctly stitches new chunks
against already-loaded ones, just without the duplicate work. This is a
genuine perf/correctness fix independent of whether it's the *complete*
explanation for the logged errors — same-frame handle churn was the
strongest available lead, and cutting it to the necessary minimum is right
regardless. *Not independently reproduced after the fix* — this environment
cannot drive the game through an equivalent loading burst to confirm the
error is gone, only that `cargo test` (35 pass), `clippy -- -D warnings`,
`fmt`, and a boot run stay clean.

**Chunk-grid axes didn't read as connected to the cube.** In play, standing
inside a chunk (32 blocks is roomy — the box's near face fills the view)
made the `OutlineAndAxes` centre-crossing lines look like a small, detached
crosshair floating in space rather than something belonging to the cube:
correct in world-space size and position (confirmed against `bevy_gizmos`'
own source — `Gizmos::cube` draws a unit cube transformed by the given
`Transform`, so `half = size / 2.0` was always dimensionally right), but
under the strong perspective of standing close to a large box, a line whose
only contact with the cube is a single point at each face's centre doesn't
read as connected to it. Fixed by giving every one of the cube's six faces
its own "+" — two lines in the *other* two axes' colours, spanning that
face — so each axis visibly continues onto the cube's surface at both ends
instead of dangling. `draw_face_cross` is the shared helper for all six.
Colours stayed exactly as they were (requested explicitly to keep them) —
red/green/blue now pulled into named constants (`AXIS_X_COLOR`/`_Y_COLOR`/
`_Z_COLOR`) since the same three now recur across nine line calls instead
of three.

**Sea-level marker was two indistinguishable lines that felt omnipresent.**
Both the X and Z lines were the same colour, so two crossing lines read as
one indistinct smudge rather than two identifiable axes; recoloured to the
same `AXIS_X_COLOR`/`AXIS_Z_COLOR` the chunk grid now uses, so the same
colour vocabulary applies everywhere in the debug overlay. Span trimmed from
`(RENDER_DISTANCE + 1) * CHUNK_SIZE` to exactly `RENDER_DISTANCE *
CHUNK_SIZE`, matching what was actually asked for ("the length of the
render distance"). The lines still re-centre on the camera's current
`(x, z)` every frame, unchanged — marking sea level under wherever the
player currently stands is the whole point, and nothing in the feedback
asked for that to become a fixed-in-world-space marker instead.

*Perf: the mesh-churn fix is a straightforward reduction in redundant work,
not a new cost anywhere — the `HashSet` construction it adds is bounded by
one loading batch's size, tiny next to the meshing work it's now skipping.
The two visual fixes cost a handful of extra gizmo line calls (12 more per
frame for the chunk grid, in `OutlineAndAxes` mode only, itself gated behind
`TESTING_TOOLS_ENABLED`) — irrelevant next to the mesh generation they sit
alongside. Correctness: `cargo fmt`, `clippy -- -D warnings`, `test` (35
pass, unchanged — none of these three fixes touch pure logic with its own
tests), and a boot run all clean. Confirmed by the user's own play session
for what prompted this entry; not yet re-confirmed by eye after the fixes
themselves, for the same reason as always — this environment cannot drive
the game interactively.*

### 2026-09-28 (3)

**A second real play session against the previous entry's fixes**, which
found the actual root cause of the slab allocator errors, and turned up two
more visual notes on the debug overlay.

**The `Use-after-free` errors were not the previous entry's mesh churn after
all.** They persisted, unchanged in frequency, even on the very first
loading burst — and that burst, worked through by hand against the
2026-09-28 (2) fix, produces *zero* neighbour re-mesh calls (every neighbour
of every chunk in an initial full-world load is itself also in that same
batch, so the dedup step filters all of them out). That ruled out the
previous diagnosis outright: something else was producing the errors, and it
wasn't fixed by that entry, only coincidentally left just as frequent as
before. Looked up rather than guessed at a second time: Bevy 0.19's
`MeshAllocator` skips *allocating* a mesh with zero vertices but still runs
the copy step for it regardless, and that copy step is what logs this
specific error — for a mesh that was genuinely never allocated in the first
place, not one freed while still in use. Despite the name, nothing unsafe
happens. This fires constantly here: a fully air chunk (rare at `1`/`1`
sea-level chunks, common once raised) or a fully solid *interior* chunk with
every face culled by its own neighbours (very common — most chunks in the
middle of a big loading burst) both produce a zero-vertex mesh from
`chunk_mesh`. Silenced via the log filter in `utils::log`, the same pattern
already used there for the Vulkan overlay-layer noise: `SILENCED` gained
`bevy_render::slab_allocator=off`, with a doc comment recording the
confirmed cause and the search that found it, rather than papering over it
with no explanation. The 2026-09-28 (2) mesh-churn dedup fix stays — it's a
real reduction in redundant work for the steady-state load/unload case, just
not what these specific log lines were about.

**Chunk grid: the per-face crosses were reverted** — *wrongly; the
diagnosis below was mistaken and the crosses are back. See 2026-09-28 (4).*
Seen in play, standing
close to a chunk, the request that prompted the per-face "+" additions
(2026-09-28 (2)) didn't read the way it was meant to: one face's cross
(whichever one ends up roughly facing the camera at a distance) shows up as
a small, visually detached square in the middle of the view rather than
something clearly connected to the rest of the cube — mistakeable for an
unrelated crosshair overlay, which is exactly how it was first described.
Reverted to just the three centre-crossing lines, confirmed to look right on
their own. See Reversals.

**Sea-level marker: rebuilt as a real grid.** The previous version drew
exactly two lines, centred on the camera's own `(x, z)` and re-centring
every frame — which meant a line segment was almost always somewhere near
the middle of the view no matter which way the camera looked, reading as a
stray disconnected mark rather than a spatial reference tied to the world.
Rebuilt to match what was actually asked for: one X-axis and one Z-axis
line per currently-loaded chunk *column* (deduplicated across that column's
vertical layers, which would otherwise draw the identical pair on top of
itself once per layer), each spanning exactly that column's width and
crossing at its centre. Adjacent columns' lines land edge-to-edge, so the
individual crosses combine into one continuous grid over the whole loaded
area — genuinely tied to world position instead of following the camera.
Needed a new accessor, `LoadedChunks::positions`, returning every loaded
position without needing to already know one to ask `chunk` about — the
first thing in `render::debug` to need to enumerate *what's* loaded rather
than look up one specific chunk.

*Perf: the log filter change and the chunk-grid revert are both pure
subtractions. The sea-level grid now costs two `gizmos.line` calls per
loaded chunk *column* per frame instead of two total — bounded by however
many columns are loaded (13 at the current `RENDER_DISTANCE`), still
trivial next to meshing, and gated behind `TESTING_TOOLS_ENABLED` like
everything else here. Correctness: `cargo fmt`, `clippy -- -D warnings`,
`test` (35 pass, unchanged), and a boot run all clean. Confirmed by the
user's own play session for what prompted this entry; the fixes themselves
are not yet re-confirmed by eye, for the usual reason.*

### 2026-09-28 (4)

**The thing stuck in the middle of the screen was the UI camera drawing
gizmos.** A third play session showed it plainly: after (3), a small yellow
square with a red and green cross, and a separate red segment, both stayed
fixed at screen centre no matter where the world camera looked. Nothing
drawn in world space can do that. Something has to be rendering it through a
camera that never moves, and the UI camera is exactly that: a `Camera2d`,
orthographic, parked at the origin looking down −Z. Gizmos render to every
camera whose render layers intersect the gizmo config's, and every camera
starts on layer `0`. So the UI camera drew its own flat copy of every gizmo
on top of the world. The chunk cube seen face-on became the square: X (red)
horizontal, Y (green) vertical, and Z pointing straight at it, collapsing to
nothing. The sea-level X lines became the red segment.

This means (3)'s chunk-grid revert was a misdiagnosis. The per-face crosses
never caused the square, since it appeared before they existed. They were
the requested behaviour all along and are restored, now written as a
three-row table (face normal, the two in-face axes and their colours)
looped over both sides, instead of six near-identical calls.

Fixed at the source: `render::debug` moves the default gizmo group onto
`GIZMO_LAYER` (`1`) at startup, and adds that layer to the world camera
(`RenderLayers::layer(0).with(GIZMO_LAYER)`, keeping `0` for chunk meshes)
via an `Added<WorldCamera>` system. That camera is spawned by
`camera::camera_world`'s own `OnEnter(InGame)` system, and there's no
guaranteed order against another plugin's system in the same schedule. The
UI camera is untouched, still on layer `0` only, so no UI rendering path is
involved in the fix. The alternative of moving the UI camera off layer `0`
was rejected for that reason.

Worth knowing beyond debug tooling: any gizmo added later, anywhere, draws
through the UI camera too unless it uses a config group on a layer only the
world camera has. Today that's only `DefaultGizmoConfigGroup`, set here.

*Perf: no change worth measuring. The UI camera now skips gizmo drawing it
was doing for no benefit. Correctness: `cargo fmt`, `clippy -- -D
warnings`, `test` (35 pass), and a boot run (no warnings or errors) all
clean. The on-screen result still needs the user's own confirmation.*

### 2026-09-29

**A player entity, a third-person camera, and (same request, split across
the turn) gravity, terrain collision, and two game modes.**

**The player.** Until now the "player" was the world camera itself —
`camera_world.rs` held `WorldCamera`, `LookAngles`, and the starting
position, and `input/` steered the camera directly. Requested: an actual
player body (`assets/Player.glb`, a Blockbench model) in third person. New
`player/` module: a `Player` entity carrying `LookAngles` (moved here from
the camera), a `Transform`, and a `WorldAssetRoot` loading the model —
`WorldAssetRoot` because that's what `SceneRoot` is called in this Bevy
version; confirmed from `bevy_gltf`'s and `bevy_world_serialization`'s own
source rather than guessed from a possibly-stale mental model of the API.
The model needed no rotation offset: decoding its glTF UV data directly (not
assumed from eyeballing the mesh) showed the head's face texture on the
−Z-normal face, which already matches `Transform::forward`. `input::look`
now writes both look angles onto the player and turns its body by yaw only,
never pitch, so the model stays upright at any camera angle; `input::
movement` now moves the player instead of the camera, along its own level
`right()`/`forward()`. `world::load_chunks_around_player` and the debug
chunk grid both switched from following the camera to following the player
— in third person they can be several blocks apart, in different chunks.

**Third person.** `camera::follow` (new) places the world camera each frame:
`config::camera::THIRD_PERSON_DISTANCE` back from a pivot at
`THIRD_PERSON_PIVOT_HEIGHT` above the player's feet, along the player's own
`LookAngles`, always facing the pivot — so looking around orbits the camera
around the player rather than turning it in place, and the player stays
centred. Runs in `PostUpdate`, before `TransformSystems::Propagate`, after
`input/`'s `Update` systems have moved the player, so the camera reads this
frame's position rather than lagging one frame behind; gated on `Changed`
player transform/angles, so an idle player leaves it untouched. The
placement itself is the pure, unit-tested `third_person_transform` —
covered directly rather than through the ECS, the same pattern
`world::generation::generate` and `player::physics`' collision use.

**Gravity and collision.** The player is now a box
(`config::player::HITBOX_WIDTH`/`HITBOX_HEIGHT`) that gravity pulls down
(`GRAVITY`, capped at `TERMINAL_VELOCITY`) and a jump (`JUMP_SPEED`, ground
only) pushes up, unless flying. It moves one axis at a time — vertical
first, so landing settles before any sliding — and an axis that would push
it into a solid block stops flush against that block instead of cancelling
the whole move, which is what lets it slide along a wall rather than stick
to it. Moves are capped under one block per step so a fast fall can't
tunnel through a thin floor. Solidity comes from the new
`LoadedChunks::is_solid`, which treats an *ungenerated* chunk inside the
world's configured height as solid (generation is async, so a player can
reach a chunk before its data exists; treating the gap as solid means they
wait for it rather than falling through) and anything outside that height as
air (so the space above the world stays flyable). All of this — the
per-axis stepping, the sub-block stepping, the three solidity cases — is
plain functions over an `is_solid` closure, tested directly against
hand-written solid/air functions, needing no `LoadedChunks` or `App` at all.
An idle, grounded (or idle, flying) player returns before touching
`Transform`, which is what keeps chunk loading and the camera follow idle
alongside it, not just physics itself.

**Two game modes.** `config::player::GameMode` (`Survival`/`Creative` —
Minecraft's names, kept only as placeholders) is read from a new
`ActiveGameMode` resource, starting in Survival and switched with `F4`.
Every existing testing-tool hotkey (`world::debug`'s chunk lock,
`render::debug`'s wireframe/chunk-grid/sea-level toggles) is now also gated
on `in_creative`, a run condition reading that resource — layered on top of
the existing `TESTING_TOOLS_ENABLED` gate, not replacing it. Flight exists
only in Creative: double-tapping Space takes off, double-tapping it again
while flying engages a double-speed ascent for as long as it's held
(`FAST_ASCENT_MULTIPLIER`), and double-tapping Left Shift lands. A plain
`Flying(bool)` component, not part of `MovementIntent`, since it's state
that persists across frames rather than a per-frame ask; switching mode
resets it unconditionally (off in Survival, `CREATIVE_STARTS_FLYING` in
Creative) so a mode swap can never strand the player flying somewhere
flight isn't supposed to exist. All the double-tap gestures — sprint, take
off, fast ascent, land — share one tap-tracking pass over all six
movement/vertical keys and `DOUBLE_TAP_WINDOW`'s timing, with a completed
double-tap clearing the record so a third tap starts a fresh pair rather
than chaining onto the pair before it.

`input::movement` no longer touches `Transform` at all — it only writes the
player's `MovementIntent` (a level horizontal velocity, a vertical one for
flight, and whether a jump was asked for). `player::physics` is what turns
that into an actual position, deciding what the ground, a jump, or a wall
allow. Splitting "what the keys ask for" from "what actually happens" is
what makes each side testable without the other: `movement`'s tests check
the intent a given key combination produces, `physics`'s check what a given
intent and terrain layout do to a position, and neither needs the other to
exist.

*Perf: physics and camera-follow both return early whenever the player is
idle, before writing `Transform` — the same "skip it outright" shape
`CLAUDE.md` asks for, and the reason an idle player costs chunk loading,
camera placement, and physics all in one shot rather than three separate
checks. The collision box's per-step sub-division is bounded (`MAX_STEP`
per step) so a fast fall costs a handful of iterations, not one per block
fallen. Readability: `world/` and the debug grid following the player
instead of the camera fixes a latent inaccuracy third person would have
otherwise introduced silently — audit item 3.3 flagged this exact
"finds the player through the camera" coupling before third person made it
actually wrong, not just imprecise. Modularity: `player/` now owns
everything about the player's own state and rules (mode, physics, model);
`camera/` only reads it, `input/` only writes to it, neither depends on the
other. Correctness: `cargo fmt`, `clippy -- -D warnings`, and `test` (60
pass: +21 — camera-follow's placement tests, `ChunkPos::of_block`,
`LoadedChunks::is_solid`, `player::physics`'s collision suite, and
`input::movement`'s gesture tests) all clean, plus a boot run with no
panics, warnings, or errors. Not confirmed by eye, for the usual reason this
environment can't drive the game interactively: whether the model actually
renders with its texture, whether it visibly faces the way it walks, how
the third-person distance and pivot height feel, whether gravity and
jumping feel right at the chosen constants, and whether the F4 mode switch
and the double-tap gestures (sprint, take off, fast ascent, land) all
trigger the way they're meant to in an actual play session.*

### 2026-09-30

**Two small fixes to the gravity/mode batch above, and a new config doc.**

**Sprint no longer survives a wall.** Reported after trying the previous
batch: running toward a wall with `DoubleTap` sprint engaged, colliding, then
jumping over the obstacle while still holding the movement key kept sprint
engaged the whole way through — it never got the *only* thing that turns it
off (every movement key released), since the key never was released.
`player::physics::Motion` gained `horizontal_collision`, set from
`Moved.blocked.x || .z` at the end of `apply_physics` — deliberately never
`.y`, which is `true` every single frame while walking on flat ground (that's
landing, not a wall) and would otherwise misfire sprint's reset constantly.
`input::movement::cancel_sprint_on_collision` reads it the next frame and
clears `Gestures::sprinting`, gated on the new
`config::player::RESET_SPRINT_ON_COLLISION` (default `true`) so the behaviour
is a config choice, not a hardcoded one. Pulled into its own one-line
function specifically so it has a direct unit test instead of only being
reachable through the full `apply_physics`/`read_movement` system pair.

**A noclip toggle for Creative.** Requested as a debug feature: the ability
to switch terrain collision off and on while testing. `player::physics`
gained `CollisionEnabled`, a resource read at the top of `apply_physics` —
off, a frame's `MovementIntent` is applied straight to `Transform` with no
block checks at all, and both `grounded` and `horizontal_collision` are
forced false rather than keep whatever they were before collision was
switched off. New `player::debug` module (F5 /
`config::input::TOGGLE_COLLISION`) follows the exact two-gate shape every
other testing feature here already does — `TESTING_TOOLS_ENABLED`,
`in_creative`, and an `_INITIALLY_*` constant
(`config::debug::COLLISION_INITIALLY_DISABLED`) guarded by both, so
collision can never start disabled with no hotkey able to turn it back on.
`game_mode::toggle_game_mode` now also unconditionally re-enables collision
on every mode switch, in either direction — leaving Creative with it off
must never strand the player noclipping through Survival, mirroring the
reset `Flying` already gets on the same switch.

**`docs/CONFIG.md`.** Requested: a dedicated doc for `config/`'s own
structure, now that it spans six files and two recurring patterns (the
settings-surface enum, the two-gate debug toggle). Deliberately describes
*structure and convention*, not current values — those stay in the `///`
comments next to each constant, the only place they can't drift out of sync
with the code. Linked from `README.md`'s doc list and `ARCHITECTURE.md`'s
Configuration section.

*Perf: no change worth measuring — one new boolean field, one new resource
read, one new early-exit branch. Modularity: the noclip toggle follows
`player/`'s own established debug-module shape rather than inventing a new
one. Correctness: `cargo fmt`, `clippy -- -D warnings`, and `test` (62 pass:
+2, the collision-reset function's two cases) all clean, plus a boot run
with no panics, warnings, or errors. Not confirmed by eye: whether the
sprint reset actually feels right in play, and whether F5 visibly toggles
collision — this environment still can't drive the game interactively.*

---

## Open items

Everything identified but not yet done lives in [`AUDIT.md`](AUDIT.md),
ranked by tier. This file only records decisions once they are made.
