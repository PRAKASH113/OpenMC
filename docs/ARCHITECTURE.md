# Architecture

High-level design of openmc_b: how the pieces fit together and why. Per-item
API documentation lives in rustdoc (`///` comments) instead — this document
covers the things rustdoc cannot, like cross-module structure and the
reasoning behind it.

> Status: early. Sections describing systems that do not exist yet are marked
> **planned**. Everything unmarked reflects code that is in the repo today.

## Module layout

```text
src/
├── main.rs                  # The only loose file: module declarations, crate attributes
├── app/                     # How the game is assembled
│   ├── mod.rs               #   run()
│   └── plugin.rs            #   AppPlugin: the composition root
├── states/                  # The state machine and every state
│   ├── mod.rs               #   GameState, InGameState, GameStatePlugin
│   ├── debug.rs             #   Debug-build-only jump keys (compiled out of release)
│   ├── loading/             #   Boot loading (solid), exits to Menu
│   │   ├── mod.rs
│   │   └── screen.rs
│   ├── menu/
│   │   ├── mod.rs
│   │   └── screen.rs
│   └── ingame/              #   A loaded world
│       ├── mod.rs
│       ├── pause.rs         #     Escape toggles Playing <-> Paused
│       └── paused/          #     Sub-state: lives inside its parent
│           ├── mod.rs
│           └── screen.rs
├── camera/                  # The camera entities: what they are, how they start
│   ├── mod.rs               #   CameraPlugin: registers every camera
│   ├── camera_ui.rs         #   Draws the interface
│   └── camera_world.rs      #   Renders the world
├── input/                   # Player controls: what input does to the view
│   ├── mod.rs               #   GameInputPlugin: all gated on Playing
│   ├── look.rs              #   Mouse -> view rotation, pitch clamp
│   ├── movement.rs          #   Keys -> flight, axis() + tests
│   └── cursor.rs            #   Lock/hide the cursor while playing
├── config/                  # Every configurable value, grouped by what it configures
│   ├── window.rs            #   Title, size, borderless, fullscreen, present mode
│   ├── input.rs             #   Key bindings, sensitivity, speed, sprint mode
│   ├── camera.rs            #   Field of view
│   ├── world.rs             #   Chunk size, render distance, world height
│   └── debug.rs             #   TESTING_TOOLS_ENABLED (debug builds only)
├── window/                  # Building the window, and changing it at runtime
│   ├── mod.rs               #   Shared fullscreen mode
│   ├── setup.rs             #   Builds the window, once, at startup
│   └── toggles.rs           #   F10/F11 at runtime
├── world/                   # Voxel/chunk data: coordinates, storage, generation
│   ├── mod.rs               #   LoadedChunks, ChunkLock, load/evict, tests
│   ├── chunk.rs             #   ChunkPos, Chunk (flat block array), tests
│   ├── block.rs             #   The Block type
│   ├── generation.rs        #   ChunkPos -> Chunk, tests
│   └── debug.rs             #   Chunk-lock hotkey (debug builds only)
├── render/                  # Turning loaded chunks into what's on screen
│   ├── mod.rs               #   RenderPlugin: spawns/despawns chunk meshes, the world light
│   ├── mesh.rs              #   Chunk -> Mesh, face-culled, tests
│   └── material.rs          #   The shared chunk material
└── utils/                   # Small, complete, unrelated-to-each-other pieces
    └── log.rs               #   Log filters
```

**The top level separates states from infrastructure.** Everything under
`states/` is a place the player can be; everything beside it is machinery
that serves every state. A new state never adds a top-level folder.

**Inside `states/`, the folders mirror the state hierarchy.** A top-level
state gets a folder in `states/`; a sub-state gets a folder *inside its
parent's* — which is why `paused/` is in `ingame/`. The layout and the state
machine describe the same shape, so neither can drift from the other
without it being visible.

**Registration follows the same nesting.** `AppPlugin` adds
`GameStatePlugin`; that adds the machine plus the top-level states; each
state adds its own sub-states (`InGamePlugin` adds `PausedPlugin`). Adding a
state touches its parent's `mod.rs` and nothing above it.

`main.rs` stays deliberately thin. It holds only what Rust requires the crate
root to hold: `mod` declarations for top-level modules, and crate-level
attributes — currently `windows_subsystem`, which hides the console window in
release builds while keeping it in dev builds so logs stay visible. Setup
logic belongs in `app`, never here.

Modules use the `mod.rs` style: every module is a folder, and `mod.rs` is its
root. `main.rs` is the only file sitting loose in `src/`.

Both this and the sibling-file style (`menu.rs` next to `menu/`) are valid in
current Rust. This one is used here because every module in this project has
submodules, so the sibling style would pair each folder with a file and
double the tree for no benefit — and because Bevy itself is written this way,
which keeps our layout and the engine's readable as one.

**Planned** domain modules, each registering its own Bevy `Plugin`:
`world/` (voxel and chunk data, generation, storage), `render/` (meshing,
chunk rendering, materials), `player/` (the player's body — physics,
gravity, collision — and block interaction), `ui/` (HUD, debug overlay).
Reading movement *intent* from the keys stays in `input/`; `player/` will
own what that intent does to a body in the world.

## Composition

Every domain is a Bevy `Plugin`, and `app::plugin::AppPlugin` is the single
place they are registered. `app::run` does nothing but add `AppPlugin` and
run, so the list of what the game is made of reads top-to-bottom in one file
— including Bevy's own plugins. `AppPlugin` builds `DefaultPlugins` with our
window and logging swapped in and the unused ones (`AudioPlugin`,
`GilrsPlugin`) disabled, then adds our domains. Configuring which engine
plugins run was tried as a separate `utils::engine` adapter and folded back
in: deciding what the app is made of is composition, not adaptation, whether
the plugin is ours or Bevy's — see `IMPROVEMENTS.md`.

Plugins are a build-time tool, not a runtime one — `Plugin::build` runs once
at startup and a system registered through a plugin runs exactly as fast as
one registered inline. So splitting by domain costs nothing at runtime; it is
purely for keeping each domain's wiring in its own file. Performance work
belongs in data layout and query design, not here.

## States

`states::GameState` is the top-level state machine: `Loading`, `Menu`,
`InGame`. `Loading` is the default, so it is what the game starts in, and it
moves on to `Menu` by itself once boot loading is done — today that is the
first frame, since there is nothing to load yet. `GameStatePlugin` in the
same module installs the machine and every state.

`Loading` is *boot* loading only. World generation and the soft loading
overlay are designed but not built; see [`LOADING.md`](LOADING.md).

`InGameState` (`Playing` / `Paused`) is a **sub-state** of `GameState::InGame`.
Bevy inserts it on entering that state and removes it on leaving, so pausing
outside a loaded world is unrepresentable rather than merely discouraged —
there is no state to write to. No code guards this; the `#[source(...)]`
attribute on the enum is the whole enforcement.

Each state has a matching module owning what it shows and what runs while it
is active, registered against `OnEnter`/`OnExit` for its own variant, so
adding behaviour to a state means working inside that state's folder and
nowhere else. `paused/` keys off `InGameState::Paused` rather than a
`GameState` variant.

Only three places in the crate change state: `states::loading::finish`
(`Loading -> Menu`), `states::ingame::pause::toggle` (Escape,
`Playing <-> Paused`), and the debug jump keys below. Nothing transitions
`Menu -> InGame` except the debug key yet — that waits for a real menu.

### Screens

`loading/` and `menu/` render a placeholder: a full-screen opaque colour with
the state's name. `paused/` renders a *translucent* overlay, because the world
is still loaded underneath and showing it is what distinguishes pausing from
leaving. `ingame/` renders the 3D scene rather than a screen. Each owns its
own marker component, so they are independent and meant to diverge
completely as real content arrives.

Temporary pieces to delete later, both marked in the source:

- `states::debug::jump_to_state` — keys `1`-`3` jump to `Loading`, `Menu`,
  `InGame`. There is deliberately no key for `Paused`; it is only reachable
  by pausing. Jumping to `Loading` bounces straight back to `Menu`, since
  loading has nothing to wait for. Debug builds only.
- The placeholder screens in `loading/` and `menu/`.

## Cameras

`camera/mod.rs` registers one plugin per camera kind:

- `camera_ui` draws the interface. Spawned at startup, lives for the whole
  run, `order: 1`, no MSAA. It clears its *own* texture to transparent and
  is composited over the world (see below). It is built
  from Bevy's `Camera2d` component, but it is a UI camera rather than a 2D
  game camera — nothing draws sprites through it.
- `camera_world` renders the world. Spawned on entering `InGame` and
  despawned on leaving, so nothing 3D renders while in a menu. `order: 0`,
  `Sample4` MSAA, and it does the clearing.

Both are named for *what they show*, not how they render — `camera_ui` and
`camera_world` rather than 2D and 3D.

**Why UI renders through `camera_ui` and not `camera_world`, once both exist
at once.** Neither camera carries Bevy's `IsDefaultUiCamera` marker. With
that marker absent, `bevy_ui`'s `DefaultUiCamera::get` (verified against
source, not assumed) falls back to the camera with the highest `order`
targeting the primary window — which is `camera_ui` at `order: 1`, above
`camera_world`'s `0`. So the draw-order convention already documented above
is doing double duty: it also decides where UI renders. `InGame`/`Paused` is
the first state where both cameras are alive together, so this path was
unexercised before then; if a third camera is ever added, whichever has the
highest order becomes the UI target unless one is marked
`IsDefaultUiCamera` explicitly.

`camera/` owns the camera *entities* only — what each camera is and how it
starts. Nothing in it reads input. The world camera carries a `LookAngles`
component (its orientation as yaw and pitch, created at spawn from the
starting view), and `crate::input` steers the camera through that and its
`WorldCamera` marker, both re-exported from `camera/mod.rs`. The dependency
runs one way: `input/` depends on `camera/`, never the reverse.

## Input

`input/` holds the player's controls — code that *interprets* input every
frame and applies it to the view. One file per control: `look.rs` (mouse to
rotation, including the pitch clamp that stops the view flipping over),
`movement.rs` (held keys to flight, with the pure `axis()` helper and its
tests), and `cursor.rs` (locking the mouse while playing, which only exists
so looking works). `GameInputPlugin` registers them, all gated on
`InGameState::Playing` so pausing freezes the view.

What deliberately lives elsewhere:

- **Which key does what** is a setting, in `config::input`. Inside `input/`
  it is imported as `controls`, because a bare `input::` there would read as
  the module itself.
- **One-shot key actions** belong to what they change, with the key as a run
  condition: Escape is in `states::ingame::pause`, F10/F11 in
  `window::toggles`. The split is continuous interpretation (here)
  versus a single action triggered by a key (with its target).

When a player body with physics arrives, `input/movement.rs` keeps reading
intent from the keys and `player/` takes over what that intent does.

Draw order is the one contract between them: the UI sits above the world.

**How the two cameras combine: separate textures, then compositing.** Bevy
gives each camera an intermediate texture keyed on its target, format *and
MSAA level*. The UI camera is `Msaa::Off` and the world camera is not, so
they draw into different textures, and each is then written to the window
in `order`: world first (replacing, after clearing the window), UI second
(blended on top). Two consequences, both verified against `bevy_render` /
`bevy_core_pipeline` source:

- **The UI camera must clear its texture every frame**, to `Color::NONE`.
  `ClearColorConfig::None` looks like "draw over the world", but the world is
  not in this texture. With `None` the texture is never cleared, so UI
  accumulates across frames and despawned UI stays on screen. This shipped
  as a real bug; see `IMPROVEMENTS.md`, 2026-09-26.
- **The composite uses premultiplied alpha**, because that is what UI
  blended onto a transparent texture leaves behind. Straight alpha would
  darken every translucent pixel.

The window itself is cleared by whichever camera writes to it first each
frame, so states without a world camera no longer need an opaque background
of their own.

## Startup configuration

`app::config` is a flat list of `pub const` values — title, size, borderless,
present mode — and nothing else. No structs, no `Default` impls, no types to
thread through the codebase. It is meant to be opened, read in a few seconds,
and edited, so anything that is not a knob someone would actually turn
belongs elsewhere.

The constants use Bevy's own enums where the setting *is* engine vocabulary
(`PresentMode` rather than a mirrored copy). Mirroring would lose the
fallback semantics — `AutoVsync` resolves to `FifoRelaxed` → `Fifo` and
`AutoNoVsync` to `Immediate` → `Mailbox` → `Fifo`, whichever the driver
supports — and would need updating every time Bevy's enum grows. Note this
also means "let Bevy decide" is already a *value* (`AutoVsync`), not a
separate mode of operation needing its own wrapper.

`app::window::primary_window_plugin` reads those constants and produces
Bevy's `WindowPlugin`, keeping every engine-shaped detail in one place:

- Bevy has no "borderless" field. It models the title bar as
  `Window::decorations` ("should decorations be drawn"), so `BORDERLESS` maps
  to the **inverse** of it.
- Present mode is a property of `Window`, not of a render plugin, so it is
  applied here too.

These constants describe **startup** only. Window size, present mode, mode
and decorations are all live-mutable on the `Window` component, so anything
changing them later mutates that component directly rather than touching
these. `app::window::WindowControlPlugin` already does exactly that for the
borderless and fullscreen toggles — the constants set where the window
starts, the systems change it afterwards.

Constants become a struct the day something needs to load them from disk at
startup — not before.

Two details in those toggles worth knowing. Fullscreen uses
`BorderlessFullscreen` rather than exclusive `Fullscreen`: it alt-tabs
instantly and does not change the display's video mode, which is what a
modern game is expected to do. And leaving fullscreen explicitly restores
`WIDTH` x `HEIGHT`, so the config values double as the remembered windowed
size. The toggles are deliberately not gated on any game state — being able
to leave fullscreen should not depend on where the player is in the game.

## Configuration

`config/` is the single answer to "where do I change a setting?". It holds
plain values only, grouped by what they configure — `window.rs` for the
window, `input.rs` for controls, `camera.rs` for the camera, `world.rs` for
world generation and layout, `debug.rs` for testing-only toggles — and
adding a category later (graphics, audio) means adding a file, not making a
decision.

Nothing in `config/` knows about the engine beyond the types a value needs.
Turning values into engine settings belongs to whoever consumes them.

`config::input` holds every key binding, so the full set is visible at once
and systems call `keys.pressed(input::FORWARD)` rather than naming a
`KeyCode` inline. A unit test there fails if two actions share a key. It
checks a hand-maintained list, so it is only as complete as whoever last
added a binding — that cost was accepted to keep the constants plainly
readable rather than generated by a macro.

`config::camera` holds field of view, in degrees — `camera_world` converts it
to radians when it builds the camera's `Projection`. Values that are not
player-facing settings stay with their owner instead: camera draw order is a
rendering detail, and the pitch clamp is a safety limit rather than a taste
setting, so both live in `camera_world`.

`config::debug` holds `TESTING_TOOLS_ENABLED`, the one switch every testing
feature's hotkey checks before registering itself — see World's `debug.rs`
paragraph for the two-gate shape this and `#[cfg(debug_assertions)]` form
together. It's the odd one out in `config/`: everything else here is a
player-facing setting; this exists purely for development.

`config::world` holds chunk size and render distance, both read by
`world/` (see the World section below), plus two constants — chunk layers
above and below sea level — that are written but not yet read anywhere,
left commented out at the point of use. They document the intended vertical
shape of the world ahead of the generation code that will need them, rather
than existing unread and risking silent drift from what generation actually
does.

Bindings are settings and live here; interpreting them is behaviour and
lives in `input/`. When action mapping, rebinding, or gamepad support arrive,
they go in `input/` too, and `config::input` keeps holding the values.

## Window

`window/` turns `config::window` into Bevy's window and keeps it updated, and
is split by lifecycle rather than kept as one file: `setup.rs` runs once,
before the app starts, to build the `WindowPlugin`; `toggles.rs` runs for the
life of the game, handling F10/F11 by mutating the live `Window` component,
which is how Bevy expects runtime window changes to be made. The one thing
both need, the fullscreen mode, sits in `window/mod.rs`.

It is a top-level module, a sibling of `camera/` and `input/`, rather than
tucked inside `utils/` — building and controlling the window is a whole
domain in its own right, not a small adapter alongside something else.

### Logical vs. physical size — the reason for the lifecycle split

`config::WIDTH`/`HEIGHT` are **logical pixels** (points): on a display at
150% OS scaling, "1280" is shown at 1920 real screen pixels, so the window is
the same visual size everywhere. That single fact is handled by two different
pieces of Bevy code depending on whether the window exists yet, which is
exactly why `setup.rs` and `toggles.rs` being separate files matters, not
just for lifecycle tidiness:

- **`setup.rs` (window creation).** With no scale factor override set — this
  project never sets one — Bevy hands winit a `LogicalSize` built from
  `window.width()`/`height()`. A freshly built `WindowResolution` defaults
  `scale_factor` to `1.0`, so those logical values equal `WIDTH`/`HEIGHT`
  numerically, and winit converts logical to physical using the real
  monitor DPI. Correct, and requires nothing extra from us.
- **`toggles.rs` (a live window).** Bevy instead compares
  `resolution.physical_width()`/`physical_height()` directly and requests
  that as an *exact* physical pixel size — no DPI conversion at all. Setting
  `window.resolution = WindowResolution::new(WIDTH, HEIGHT)` here — building
  a fresh value — resets `scale_factor` to its `1.0` default and feeds
  `WIDTH`/`HEIGHT` into this physical-pixels path, so the window comes back
  smaller than it opened at on any display above 100% scaling. This shipped
  as a real bug; see `IMPROVEMENTS.md`, 2026-09-26.
  The fix is `window.resolution.set(WIDTH as f32, HEIGHT as f32)`: `.set()`
  mutates the existing `WindowResolution` rather than replacing it, so it
  keeps the `scale_factor` Bevy has been tracking live from the OS, and
  performs the same logical-to-physical multiplication windows creation gets
  for free.

The rule this leaves behind: **never replace `window.resolution` wholesale
on a live window** (`window.resolution = WindowResolution::new(..)`) if the
values should be read as logical pixels — always `.set()`, which is
scale-factor-aware. `WindowResolution::new(..)` is only safe to reach for
during window *creation*, where Bevy's own conversion covers for it.

## World

`world/` owns voxel data: chunk coordinates, storage, and generation. It
knows nothing about turning that data into anything on screen — that's
`render/`'s job (below), and `world/` is written without depending on it,
the same way `input/` doesn't know `player/` is coming.

**`Chunk`** (`chunk.rs`) is a cube of blocks, [`config::world::CHUNK_SIZE`]
to a side, stored as one flat `Vec<Block>` indexed by a computed offset
rather than a nested `Vec<Vec<Vec<Block>>>` — one contiguous allocation, and
the layout `render/`'s mesher walks. **`ChunkPos`**, also in `chunk.rs`, is a
chunk coordinate (one unit = `CHUNK_SIZE` blocks) kept as its own type rather
than a bare `IVec3`, so a chunk coordinate and a block coordinate can never
be passed to the wrong place by mistake — and a `Component`, since `render/`
tags each chunk's mesh entity with the position it renders.
`ChunkPos::containing` finds the chunk holding a world-space position by
floor division, not truncation, which would misplace negative coordinates at
the origin; `ChunkPos::origin` is the reverse, the chunk's corner in world
space, which is where `render/` places its mesh entity. Both are
unit-tested.

**`generation.rs`** turns a `ChunkPos` into a `Chunk`. It is a pure function
(no ECS, no I/O), so it is directly unit-tested and, later, safe to move off
the main thread without touching anything else. What it builds today is a
flat placeholder floor — solid in the bottom half of every chunk, identical
regardless of position — not real terrain: that needs the sea-level concept
`config::world` documents but does not wire in yet (see the comment on
`CHUNKS_ABOVE_SEA_LEVEL`/`CHUNKS_BELOW_SEA_LEVEL` there). This exists so
storage, coordinates, and the load/generate cycle can be built and tested
against *something* before terrain generation is real.

**`LoadedChunks`** (in `mod.rs`) is a plain resource holding every generated
chunk in a `HashMap<ChunkPos, Chunk>` — not one entity per chunk, because
nothing about a chunk's data needs querying or despawning through the ECS;
that's what the mesh entities `render/` spawns are for. Named `LoadedChunks`
rather than `World`, deliberately: this file also glob-imports
`bevy::prelude::*`, which exports Bevy's own ECS `World` type, and a second
type of the same name in the same file is exactly the kind of thing that
silently means two different things depending on where you're standing.

`load_chunks_around_player` runs every frame the game is `InGame`. It finds
the player's current chunk, generates every not-yet-loaded chunk within
[`config::world::RENDER_DISTANCE`] of it, and drops every loaded chunk that
has fallen back out of range — eviction runs first, so the map is never
briefly holding both an old and a new chunk in the same slot at the
boundary. Both directions fire a message, `ChunkLoaded` or `ChunkUnloaded`,
which is how `render/` finds out without polling `LoadedChunks` itself.

**In range** is a circle, by squared distance (`in_render_distance`), not
the bounding square a radius suggests — a square's far corners are up to
`radius * sqrt(2)` chunks away, about 27% more chunks loaded for the same
nominal distance than a circle, and they'd pop in and out at an inconsistent
distance depending on which way the player is moving. This one function is
the single source of truth for "is this chunk in range", used identically by
both the load loop and the eviction check, so the two can never disagree at
the boundary — which would otherwise be its own bug class (a chunk loaded by
one rule and immediately evicted by a stricter one). The vertical axis is
never part of the radius: a chunk one layer above or below the player is
never in range no matter how large `RENDER_DISTANCE` is, because vertical
range is the separate, not-yet-wired sea-level concept. Five unit tests pin
this shape down directly, including the specific case that motivates the
circle over the square (a diagonal chunk that a square would include).

On leaving `GameState::InGame`, `LoadedChunks` is reset to empty in one
step rather than evicting chunk-by-chunk — re-entering generates fresh
rather than reusing whatever was left over. This does **not** fire
`ChunkUnloaded` for each dropped chunk; see Render for why.

**`debug.rs` (debug builds only) freezes loading for testing.** `ChunkLock`
is a plain `bool` resource, defined unconditionally in `mod.rs` so
`load_chunks_around_player`'s run condition (`chunk_loading_unlocked`) reads
the same in every build — always unlocked in release, since nothing there
can ever set it otherwise. Only *toggling* it is debug-only: `world::debug`
registers a hotkey (`F9`, `config::input::TOGGLE_CHUNK_LOCK`) that flips it,
and registration itself is skipped unless
`config::debug::TESTING_TOOLS_ENABLED` is also `true` — a testing feature
existing at all (the `#[cfg(debug_assertions)]`) is a separate question from
whether it's active in this particular session (the `const bool`), so a
stray `F9` during ordinary debug-build play can't silently do something
unexpected. While locked, `load_chunks_around_player` does not run at all —
whichever chunks were loaded the moment it engaged stay loaded, however far
the player wanders, and nothing new generates or unloads. This is the shape
every future testing feature should follow: state that always exists
(cheap, uniform across builds), a toggle that only exists in debug
(`#[cfg(debug_assertions)]`), gated a second time by the shared
`TESTING_TOOLS_ENABLED` switch (`config::debug`) so debug builds don't
default to every testing feature being live.

`ChunkLock`'s `Default` starts it engaged rather than always `false` when
`config::debug::CHUNK_LOCK_INITIALLY_ENGAGED` says so — for testing a fixed
view without needing `F9` first. That constant is guarded by
`TESTING_TOOLS_ENABLED` too, not just its own value: without that guard, a
world could start permanently locked with no hotkey able to unlock it, since
the hotkey itself only exists when testing tools are enabled. In release
this is always `false`, unconditionally — there is no debug-only constant to
read, and no real build should ever start frozen.

## Render

`render/` owns turning `world/`'s chunk data into what the player sees:
meshing a chunk's blocks into a `Mesh`, giving it a material, and spawning
or despawning the entity that renders it as chunks load and unload. It
depends on `world/` — reading `LoadedChunks` and reacting to
`ChunkLoaded`/`ChunkUnloaded` — and `world/` has no idea `render/` exists,
the same direction as `input/` depending on `camera/`.

**`mesh.rs`** builds a `Mesh` from a `&Chunk`, in the chunk's own local
space (a block at `(x, y, z)` is a unit cube from that corner to
`(x+1, y+1, z+1)`; the entity's `Transform`, built from `ChunkPos::origin`,
is what places it in the world). It only emits a face where the neighbouring
block **isn't** solid — a naive mesher emitting all six faces of every solid
block would produce roughly 390,000 vertices for a half-solid chunk, almost
all of them faces buried against a neighbour the camera can never see;
culling those is the difference between that and a few thousand. It does
**not** merge coplanar faces into larger quads (full greedy meshing) — that
further optimisation is left for later. A chunk edge always counts as
exposed, since neighbouring chunks aren't consulted yet; once
`RENDER_DISTANCE` is raised above `0` this draws (harmlessly, since it's
hidden behind the next chunk) extra faces at every chunk boundary, and
removing them needs `mesh.rs` to look past its own chunk's data. Three unit
tests cover an empty chunk producing no geometry, an isolated block getting
all six faces, and two adjacent blocks culling the one face between them.

**`material.rs`** builds one shared `Handle<StandardMaterial>` at `Startup`
and every chunk mesh reuses it — the same reasoning the old placeholder
scene used for its one shared cube mesh, just on the material side. A
texture atlas (a sprite per block type) is what replaces this once there is
more than one visible block type worth telling apart.

**`mod.rs`** wires it together: `spawn_chunk_meshes` reacts to `ChunkLoaded`
by meshing the chunk and spawning an entity (`Mesh3d`, the shared material,
a `Transform` at the chunk's origin, and the `ChunkPos` itself as a
component); `despawn_chunk_meshes` reacts to `ChunkUnloaded` by looking the
entity up in `ChunkEntities` (a `HashMap<ChunkPos, Entity>`, so a single-chunk
despawn is one lookup rather than a scan) and despawning it.
`despawn_all_chunk_meshes` runs on leaving `GameState::InGame` and clears
every remaining entity in one pass — independent of `ChunkUnloaded`, since
`world/`'s whole-map reset on the same transition doesn't fire one message
per chunk. This module also owns the world's `DirectionalLight`: it exists
purely so chunk meshes are visible, which is the same category of thing as
the material they're given, and there's no better home for a single light
yet — a dedicated lighting module is the natural extraction point once
there's more than one.

## Utilities

`utils/` holds small pieces that do one complete job and have no natural
domain of their own — currently just `log.rs`, which decides what log targets
print. The bar for belonging here is completeness, not size, so a folder
kept for exactly this purpose does not become a place where an in-progress
design collects by default.

`window/` and Bevy's own plugin configuration were both tried here first and
moved out: window setup grew into two files and a real lifecycle split, which
made it a domain rather than a small utility; and deciding which engine
plugins run is a composition decision, so it belongs in `AppPlugin` — see
`IMPROVEMENTS.md`.

## Logging

`app::log::log_plugin` configures Bevy's `LogPlugin`. It builds on
`bevy::log::DEFAULT_FILTER` rather than replacing it, so Bevy's own sensible
defaults survive and our entries are additive.

One target is currently silenced: `wgpu_hal::vulkan::instance`. The Vulkan
loader logs an ERROR for every overlay layer it cannot open, and Steam and
the Epic launcher both register layers that are often not installed. Five
errors fired on every startup, which buries real ones.

Log configuration lives beside the window translation rather than in
`main.rs` because `LogPlugin` is configured *on* `DefaultPlugins` — it has to
happen where the plugin group is built, and `main.rs` deliberately holds no
setup.

`states::log_state_change` logs every state transition, skipping
identity transitions.

## Decisions

Notable technical choices (chunk storage format, meshing algorithm, threading
model) get a short record in `decisions/` — context, decision, consequences —
once there is a decision worth recording. That folder does not exist yet.

## Keeping this current

A change that alters the structure described here updates this document in
the same commit. A stale architecture doc is worse than none, because it
misleads. See [`CLAUDE.md`](../../CLAUDE.md) for the full set of project
conventions.
