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
├── player/                  # The player: its entity, model, game mode, and physics
│   ├── mod.rs               #   Player, LookAngles, MovementIntent, Flying, PlayerPlugin
│   ├── game_mode.rs         #   Active mode, F4 toggle, in_creative run condition
│   ├── physics.rs           #   Gravity, jumping, terrain collision, tests
│   └── debug.rs             #   F5 noclip toggle (debug builds only)
├── camera/                  # The camera entities, and the third-person follow
│   ├── mod.rs               #   CameraPlugin: registers every camera
│   ├── camera_ui.rs         #   Draws the interface
│   ├── camera_world.rs      #   Renders the world
│   └── follow.rs            #   Keeps the world camera behind the player, tests
├── input/                   # Player controls: what input does to the player
│   ├── mod.rs               #   GameInputPlugin: all gated on Playing
│   ├── look.rs              #   Mouse -> look angles and body yaw, pitch clamp
│   ├── movement.rs          #   Keys -> MovementIntent, double-tap gestures, tests
│   └── cursor.rs            #   Lock/hide the cursor while playing
├── config/                  # Every configurable value, grouped by what it configures
│   ├── window.rs            #   Title, size, borderless, fullscreen, present mode
│   ├── input.rs             #   Key bindings, sensitivity, speed, sprint mode
│   ├── camera.rs            #   Field of view, third-person distance and pivot
│   ├── player.rs            #   Game mode, gravity, jump, hitbox
│   ├── world.rs             #   Chunk size, render distance, world height, terrain noise
│   └── debug.rs             #   TESTING_TOOLS_ENABLED (debug builds only)
├── window/                  # Building the window, and changing it at runtime
│   ├── mod.rs               #   Shared fullscreen mode
│   ├── setup.rs             #   Builds the window, once, at startup
│   └── toggles.rs           #   F10/F11 at runtime
├── world/                   # Voxel/chunk data: coordinates, storage, generation
│   ├── mod.rs               #   LoadedChunks, ChunkLock, load/evict, tests
│   ├── chunk.rs             #   ChunkPos (+ face_neighbors), Chunk (flat block array), tests
│   ├── block.rs             #   The Block type
│   ├── generation.rs        #   ChunkPos -> Chunk, a noise heightmap, tests
│   └── debug.rs             #   Chunk-lock hotkey (debug builds only)
├── render/                  # Turning loaded chunks into what's on screen
│   ├── mod.rs               #   RenderPlugin: spawns/despawns/re-meshes chunk meshes, the world light
│   ├── mesh.rs              #   Chunk -> Mesh via binary_greedy_meshing, tests
│   ├── material.rs          #   The shared chunk material
│   └── debug.rs             #   Wireframe/chunk-grid/sea-level toggles (debug builds only)
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

**Planned** domain modules, each registering its own Bevy `Plugin`: `ui/`
(HUD, debug overlay). Block interaction, when it comes, belongs in
`player/`. Reading movement *intent* from the keys stays in `input/`, and
`player/` owns what that intent does to a body in the world.
Assets live in `assets/` at the crate root, which is where Bevy's asset
server looks when run through `cargo run` — currently just
`assets/models/player.glb`.

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

`camera/` owns the camera entities, plus one runtime behaviour in its own
file: **`follow.rs`, the third-person view.** Each frame the world camera
is placed `config::camera::THIRD_PERSON_DISTANCE` back from a pivot at
`THIRD_PERSON_PIVOT_HEIGHT` above the player's feet, along the player's
`LookAngles`, and faces the pivot. Looking around therefore swings the
camera around the player instead of turning it in place, and the player
stays centred. The maths is the pure `third_person_transform`, covered by
unit tests: the camera sits directly behind at neutral angles, is always the
configured distance from the pivot, always faces it, and rises above it when
looking down. The system runs in `PostUpdate` before
`TransformSystems::Propagate`, after `input/` has moved and turned the
player in `Update`, so the camera lands on where the player is this frame
rather than lagging one behind. It skips itself when neither the player's
transform nor its look angles changed. There's no camera collision yet, so
it can clip into terrain behind the player.

Nothing in `camera/` reads input. Dependencies: `camera/` reads `player/`,
and `input/` writes to `player/`. Neither touches the other.

## Input

`input/` holds the player's controls — code that *interprets* input every
frame and applies it to the player. One file per control:

- **`look.rs`** turns the mouse into the player's `LookAngles` and turns the
  body to match. Only yaw reaches the body's transform, so the model never
  tilts; pitch only moves the camera. It also holds the pitch clamp that
  stops the view flipping over.
- **`movement.rs`** turns held keys into the player's `MovementIntent`:
  which way to walk or fly, at what speed, and whether to jump. It doesn't
  move anything itself. Forward, back and strafe follow the body's own
  `forward()`/`right()`, which are always level because the body only has
  yaw, so walking forward while looking down stays along the ground. It's
  also where double-taps are read. A double-tap of a movement key sprints.
  In Creative, a double-tap of Space takes off, and a double-tap of Space
  while flying rises at double speed while held. A double-tap of Left Shift
  lands. Each piece is a plain helper with its own tests: `register_taps`,
  `update_gestures`, `movement_intent`, `axis`.
- **`cursor.rs`** locks the mouse while playing, which only exists so looking
  works.

`GameInputPlugin` registers them, all gated on `InGameState::Playing` so
pausing freezes the player.

What deliberately lives elsewhere:

- **Which key does what** is a setting, in `config::input`. Inside `input/`
  it is imported as `controls`, because a bare `input::` there would read as
  the module itself.
- **One-shot key actions** belong to what they change, with the key as a run
  condition: Escape is in `states::ingame::pause`, F10/F11 in
  `window::toggles`. The split is continuous interpretation (here)
  versus a single action triggered by a key (with its target).

The controls run before `player::PlayerPhysics` each frame, so physics
acts on this frame's keys.

## Player

`player/` owns the player entity: a `Player` marker, its `LookAngles` (yaw
and pitch in radians, stored rather than read back from a quaternion so they
don't drift and pitch clamps cleanly), a `Transform` at its feet, and a
`WorldAssetRoot` that loads `assets/models/player.glb`. In Bevy 0.19,
`WorldAssetRoot` is the component formerly called `SceneRoot`. The model is
a Blockbench export, 2 blocks tall with its origin between the feet. It
already faces −Z, Bevy's forward, which was checked by reading which side of
the head the face texture's UVs land on. It goes straight onto the player
entity with no rotation offset, and Bevy's glTF loader applies no coordinate
conversion by default. It is spawned on entering `InGame` and despawned on
leaving, at a fixed `SPAWN_POSITION` above the terrain band, looking toward
the world origin.

`world/` loads chunks around the player's position, and `render::debug`'s
chunk grid follows it. Both used to follow the camera. In third person, the
camera can be in a different chunk from the body it's following.

**`game_mode.rs`: two sets of rules.** `config::player::GameMode` has two
variants, `Survival` and `Creative`. Those names are placeholders borrowed
from Minecraft until the game settles on its own. The active one is the
`ActiveGameMode` resource. It starts at `config::player::INITIAL_GAME_MODE`
and flips on `F4` (`config::input::TOGGLE_GAME_MODE`) while playing. A
resource, not a component on the player, because it outlives a visit to a
world, and systems that never touch the player need it as a run condition.
`in_creative` is that condition. Every testing-tool key (`world::debug`,
`render::debug`, `player::debug`) carries it, so those keys do nothing in
Survival. What they already turned on stays on until you're back in
Creative to turn it off. The debug state-jump keys (`1`–`3`) are left
alone, since `3` is still the only way into a world at all. Switching mode
also resets flight (always off in Survival, and
`config::player::CREATIVE_STARTS_FLYING` — default `false` — in Creative)
and re-enables collision unconditionally, so leaving Creative with it
switched off (see `player::debug` below) can never strand the player
noclipping through Survival.

**`physics.rs`: gravity and terrain.** Each frame while playing, the player
moves by its `MovementIntent`:

- **On foot:** gravity (`config::player::GRAVITY`, capped at
  `TERMINAL_VELOCITY`) pulls it down, and a jump sets its upward speed to
  `JUMP_SPEED`, but only from the ground.
- **Flying** (`Flying`, Creative only): vertical speed comes straight from
  the intent and there's no gravity.

The player is a box, `HITBOX_WIDTH` by `HITBOX_HEIGHT`, standing on its
`Transform`. It moves one axis at a time, vertical first, and an axis that
would push it into a solid block stops flush against that block instead.
That per-axis rule is what makes it slide along walls rather than sticking.
Moves are split into steps of under one block, so a fast fall or a slow
frame can't tunnel through a one-block floor. Solidity comes from
`world::LoadedChunks::is_solid`, which covers three cases:

- **Outside the world's vertical extent:** air, so the space above the
  world's top stays flyable.
- **An ungenerated chunk inside the world:** solid. Generation is async, so
  the player waits for a chunk rather than falling through ground that
  hasn't arrived. A player whose box already overlaps such a chunk is
  "stuck" and doesn't move at all until it loads.
- **A loaded block:** its actual contents.

`CollisionEnabled`, a resource, gates all of this at once — see
`player::debug` below. Off, a frame's `MovementIntent` is applied to the
`Transform` directly, with no block checks at all: noclip.

The collision maths is plain functions over an "is this block solid"
callback (`move_and_collide`, `move_axis`, `overlaps_solid`), unit-tested
without a world. `Motion`, the physics state carried between frames, also
tracks whether the *last* move was stopped by a wall — `x`/`z` only, never
`y`, since landing blocks that axis every frame while walking on flat ground
and must never read as a wall hit. `input::movement` reads that flag to
cancel a sticky `SprintMode::DoubleTap` sprint the frame after a collision
(`config::player::RESET_SPRINT_ON_COLLISION`): running into a block and
jumping over it, still holding the movement key throughout, would otherwise
keep the old sprint engaged forever, since it only turns off on its own when
every movement key is released. Standing still on the ground, or hovering in
flight, skips the system before it touches `Transform`. That matters beyond
the saved work: chunk loading and the camera follow both run only when the
player's `Transform` changes, so an idle player keeps all three idle.

**`player/debug.rs` (debug builds only): noclip.** One hotkey,
`F5`/`config::input::TOGGLE_COLLISION`, flips `CollisionEnabled`. Same shape
as `world::debug` and `render::debug` — `TESTING_TOOLS_ENABLED`-gated
registration, plus `in_creative` on the hotkey itself, plus an
`_INITIALLY_*` constant (`config::debug::COLLISION_INITIALLY_DISABLED`)
guarded the same way `CHUNK_LOCK_INITIALLY_ENGAGED` is: only takes effect
alongside `TESTING_TOOLS_ENABLED`, so collision can never start disabled
with no hotkey able to turn it back on.

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
world generation and layout, `player.rs` for game mode and physics,
`debug.rs` for testing-only toggles — and adding a category later (graphics,
audio) means adding a file, not making a decision. See
[`docs/CONFIG.md`](CONFIG.md) for the conventions every file in it follows
(the settings-surface enum pattern, the two-gate debug pattern) in more
depth than fits here.

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
player-facing setting; this exists purely for development. It also owns
`ChunkGridMode` (the three states `render::debug`'s chunk-bounds grid can
be in) and every testing feature's `_INITIALLY_*` starting-state constant —
a settings-surface enum lives here the same way `config::input::SprintMode`
does, with the behaviour that interprets it living in whichever module owns
the feature.

`config::world` holds chunk size and render distance, plus
`CHUNKS_ABOVE_SEA_LEVEL`/`CHUNKS_BELOW_SEA_LEVEL` (now active, each set to
`1` while vertical loading is new — see World's `mod.rs` paragraph) and the
terrain-noise tuning constants `generation.rs` reads. All of it is read by
`world/`; see the World section below.

`config::player` holds `GameMode` and its starting value, gravity, jump
speed, terminal velocity, the hitbox, and `RESET_SPRINT_ON_COLLISION` —
everything `player/` reads to decide how the player moves and collides. See
the Player section below.

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
space, which is where `render/` places its mesh entity. `ChunkPos::face_neighbors`
returns the six chunks sharing a face with this one, in no particular
order — `render/` uses it both to find which neighbours' padding to read
when meshing and which already-spawned neighbours need re-meshing when a new
chunk arrives. All three are unit-tested.

**`generation.rs`** turns a `ChunkPos` into a `Chunk`. It is a pure function
(no ECS, no I/O), so it is directly unit-tested and, later, safe to move off
the main thread without touching anything else. Terrain is a heightmap: each
`(x, z)` column gets its own height from an [`noise::Fbm<Perlin>`] (fractal
Brownian motion — several octaves of Perlin noise summed together, tuned by
`config::world::TERRAIN_OCTAVES`/`_FREQUENCY`/`_PERSISTENCE`), everything
below that height filled solid, everything above left air. The noise is
sampled in *world* space (`ChunkPos::origin() + local (x, z)`), not
chunk-local space — the reason neighbouring chunks' terrain lines up at the
seam instead of each chunk looking like its own disconnected island.
`config::world::TERRAIN_AMPLITUDE` bounds how far the surface can stray from
sea level (absolute world height `0`) in either direction — the surface
never rises above `TERRAIN_AMPLITUDE` or sinks below `-TERRAIN_AMPLITUDE`,
regardless of how many chunk layers `CHUNKS_ABOVE_SEA_LEVEL`/
`_BELOW_SEA_LEVEL` load. That bound is what makes most loaded chunks
trivial: one entirely below the band is solid rock through and through
(one bulk `fill_below_height`, no noise sampled at all); one entirely above
it is solid air (`Chunk::empty()`, zero work). Only a chunk whose vertical
range actually overlaps the band costs a noise sample per column, and even
then the guaranteed-solid portion of *that* chunk (whatever falls below
`-TERRAIN_AMPLITUDE`) is still bulk-filled rather than checked block by
block — only the band a column's actual height might land in needs
per-block placement. This keeps generation cheap as
`CHUNKS_ABOVE_SEA_LEVEL`/`_BELOW_SEA_LEVEL` grow: most of a tall world sits
entirely outside the terrain band and costs nothing to generate. Seven unit
tests cover determinism (the same position always regenerates identically),
that height actually varies across a chunk, that no column's height escapes
`0..CHUNK_SIZE` within its own chunk, that two different positions produce
different terrain, and the three trivial/overlapping-chunk shortcuts
themselves.

**`LoadedChunks`** (in `mod.rs`) is a plain resource holding every generated
chunk in a `HashMap<ChunkPos, Chunk>` — not one entity per chunk, because
nothing about a chunk's data needs querying or despawning through the ECS;
that's what the mesh entities `render/` spawns are for. Named `LoadedChunks`
rather than `World`, deliberately: this file also glob-imports
`bevy::prelude::*`, which exports Bevy's own ECS `World` type, and a second
type of the same name in the same file is exactly the kind of thing that
silently means two different things depending on where you're standing.

`load_chunks_around_player` runs every frame the game is `InGame`. It finds
the player's current chunk *column* and, for every column within
[`config::world::RENDER_DISTANCE`] of it, queues generation for every
vertical layer that column should have — the column's entire height, from
`CHUNKS_ABOVE_SEA_LEVEL` above sea level down to `CHUNKS_BELOW_SEA_LEVEL`
below it, not just the layer the player happens to be standing on ("Option
A": every in-range column loads its whole fixed-height extent, as opposed
to a vertical radius around the player's own position, which was considered
and rejected — it doesn't match a world whose height is meant to be a fixed
property of the world, not of wherever the player is right now). It also
drops every loaded or in-flight chunk that has fallen out of range —
eviction runs first, so no map is ever briefly holding both an old and a
new chunk in the same slot at the boundary.

Generation itself does not happen inline in this system — it only decides
what's wanted and hands each newly-wanted position to a background task
(see "Async generation" below); loading and unloading still fire
`ChunkLoaded`/`ChunkUnloaded` as before, just from a different system.

**In range** is two independent checks, both in `in_render_distance`: a
horizontal circle (by squared distance) around the player's column, and
`in_vertical_range` — whether a chunk's `y` falls inside the fixed
`-CHUNKS_BELOW_SEA_LEVEL..CHUNKS_ABOVE_SEA_LEVEL` band. The horizontal
circle, not the bounding square a radius suggests: a square's far corners
are up to `radius * sqrt(2)` chunks away, about 27% more chunks loaded for
the same nominal distance than a circle, and they'd pop in and out at an
inconsistent distance depending on which way the player is moving. The
vertical check is a fixed extent, not a radius — unlike the horizontal
circle, it never shifts as the player moves up or down, since the world's
height is meant to be constant regardless of where in it the player is
standing. `in_render_distance` is the single source of truth for "is this
chunk in range", used identically by the load loop, `LoadedChunks`'s
eviction, and `PendingChunks`'s eviction, so none of the three can ever
disagree at the boundary — which would otherwise be its own bug class (a
chunk loaded by one rule and immediately evicted by a stricter one). Eight
unit tests pin this shape down: the horizontal cases from before (including
the diagonal-corner case that motivates the circle over the square), plus
the vertical ones (independence from the player's own height, the sea-level
boundary itself, the extent's edges, and that it scales correctly with
`above`/`below`).

On leaving `GameState::InGame`, `LoadedChunks` and `PendingChunks` are both
reset to empty in one step rather than evicted chunk-by-chunk — re-entering
generates fresh rather than reusing whatever was left over, and a task
still running from the previous visit can't finish late and insert into the
new session's map as if it belonged there. This does **not** fire
`ChunkUnloaded` for each dropped chunk; see Render for why.

**Async generation.** `generate` is a pure function (no ECS, no I/O — see
`generation.rs`), so calling it costs nothing to move off the main thread:
`load_chunks_around_player` spawns it on `bevy::tasks::AsyncComputeTaskPool`
for each newly-wanted position and tracks the in-flight `Task<Chunk>` in
`PendingChunks` (`HashMap<ChunkPos, Task<Chunk>>`, the same shape as
`LoadedChunks` itself), keyed so the same chunk is never queued twice while
one attempt is still running. A second system, `apply_generated_chunks`,
polls every pending task once per frame (`block_on(poll_once(task))`),
collects whichever have finished into a plain `Vec` first — removing an
entry while still iterating the map it came from isn't possible in safe
Rust, and polling an already-finished task a second time isn't part of a
future's contract — then inserts each into `LoadedChunks` and fires
`ChunkLoaded`, exactly as `load_chunks_around_player` used to do inline.
`render/` needed zero changes for this: it reacts to the same message at
the same point in the pipeline, indifferent to where the chunk data came
from.

The two systems deliberately have different run conditions.
`load_chunks_around_player` keeps `chunk_loading_unlocked` and
`Changed<Transform>` — deciding what's wanted still can't change unless the
player moved. `apply_generated_chunks` only needs `in_state(InGame)`: a
background task can finish on any frame, including one where the player is
standing still, and it isn't gated on the chunk lock either — locking stops
*new* work from being requested, but work already dispatched should still
land when it finishes rather than being stranded in `PendingChunks` forever.
Dropping a still-running `Task` (via `PendingChunks::retain`, driven by the
same `in_render_distance` check as `LoadedChunks`'s own eviction) cancels
it — there's no point letting a chunk finish generating for a position the
player already left.

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

**`mesh.rs`** builds a `Mesh` from a chunk, using the
[`binary_greedy_meshing`](https://github.com/Inspirateur/binary-greedy-meshing)
crate — a Rust port of the reference algorithm at
[cgerikj/binary-greedy-meshing](https://github.com/cgerikj/binary-greedy-meshing)
— rather than a hand-rolled mesher. This replaced an earlier hand-written
face-culled mesher (still only emitting a face where the neighbouring block
isn't solid, no merging) that lived here first; see Reversals in
`IMPROVEMENTS.md`. The crate doesn't just cull hidden faces, it *merges*
adjacent same-type faces into the largest quad it can, using bitwise
operations to do it fast — a flat 32×32 floor becomes one quad instead of
1,024. It's generic over chunk size (a `const` parameter on `bgm::Mesher`),
so it works with our own `CHUNK_SIZE` rather than requiring its demo's 62.

The crate needs its input **padded**: a chunk's own blocks in the middle,
surrounded by one block of neighbouring data on every side, so it can tell
whether a boundary face is hidden by whatever's next door.
`write_neighbor_padding` fills that padding from `LoadedChunks`, wherever a
face-adjacent neighbour happens to be loaded — this is what gives real
cross-chunk face culling, closing the gap the old mesher had (it never
looked past its own chunk's data at all; see `docs/AUDIT.md` 1.6). Only the
six face-adjacent neighbours are read, never a diagonal one: the crate's
face culling only ever looks at a cell's direct neighbour, so a diagonal
chunk's data is never actually consulted regardless of what it contains. A
neighbour that isn't loaded leaves its side of the padding as air, exactly
the old exposed-edge behaviour. Three unit tests cover an empty chunk
producing no geometry, a fully solid chunk with no neighbours merging into
exactly one quad per side (six total — proof the merging is real, not just
culling), and a solid neighbour on one side removing exactly that one face.

**`material.rs`** builds one shared `Handle<StandardMaterial>` at `Startup`
and every chunk mesh reuses it — the same reasoning the old placeholder
scene used for its one shared cube mesh, just on the material side. A
texture atlas (a sprite per block type) is what replaces this once there is
more than one visible block type worth telling apart.

**`mod.rs`** wires it together: `spawn_chunk_meshes` reacts to `ChunkLoaded`
by meshing the chunk and spawning an entity (`Mesh3d`, the shared material,
a `Transform` at the chunk's origin, and the `ChunkPos` itself as a
component). It then also **re-meshes any of that chunk's already-spawned
neighbours** — necessary precisely because of the padding scheme above: a
neighbour meshed *before* this chunk existed was built as if this side were
open air, and nothing else would ever tell it that's no longer true.
`despawn_chunk_meshes` reacts to `ChunkUnloaded` by looking the entity up in
`ChunkEntities` (a `HashMap<ChunkPos, Entity>`, so a single-chunk despawn is
one lookup rather than a scan) and despawning it. `despawn_all_chunk_meshes`
runs on leaving `GameState::InGame` and clears every remaining entity in one
pass — independent of `ChunkUnloaded`, since `world/`'s whole-map reset on
the same transition doesn't fire one message per chunk. This module also
owns the world's `DirectionalLight`: it exists purely so chunk meshes are
visible, which is the same category of thing as the material they're given,
and there's no better home for a single light yet — a dedicated lighting
module is the natural extraction point once there's more than one.

**`debug.rs` (debug builds only)** adds three visual testing aids, following
the same two-gate shape as `world::debug` (`#[cfg(debug_assertions)]` plus
`config::debug::TESTING_TOOLS_ENABLED`) and registered the same way — a
plain `if` around `app.add_systems(...)`, so with testing tools off none of
this exists in the schedule at all.

- **Wireframes** (`F8`) flip Bevy's own `WireframeConfig.global`, to check
  the greedy mesher is actually merging faces the way it claims to rather
  than drawing individual triangles. This needs GPU features
  (`POLYGON_MODE_LINE`, `IMMEDIATES`) requested up front, in `app::plugin`
  (debug builds only) via a custom `RenderPlugin`/`WgpuSettings` — an
  adapter that doesn't support them logs a warning and no-ops rather than
  crashing, so this is safe to leave configured even if it's ever run on
  hardware that can't do it.
- **The chunk-bounds grid** (`F7` to cycle, `Alt+F7` to lock) draws a
  gizmo cube around whichever chunk the player is currently in, in one of
  three modes owned by `config::debug::ChunkGridMode` (`None`, `Outline`,
  `OutlineAndAxes` — the outline, a line through the centre along each
  axis, and a matching "+" on each of the six faces, so the cube reads as
  eight sub-cubes). Locking (`ChunkGridLock`) freezes
  the grid on its current chunk instead of following the player, the same
  idea as `world::ChunkLock` freezing loading — both share the shape "press
  a key, something stops updating with the player's movement until pressed
  again." `F7` and `Alt+F7` share one physical key because they're the same
  gesture at two different commitment levels (cycle vs. lock), not two
  unrelated actions — the two systems that read `TOGGLE_CHUNK_GRID` guard
  against each other with `input_pressed(DEBUG_MODIFIER)`/`not(...)` so
  only one ever fires from the same keypress.
- **The sea-level marker** (`F6`) draws a grid at absolute world height `0`
  — one X-axis and one Z-axis line per currently-loaded chunk *column*
  (`LoadedChunks::positions`, deduplicated by `(x, z)` since every vertical
  layer of a column shares the same pair of lines), each spanning that
  column's own width and crossing at its centre. Adjacent columns' lines
  land edge to edge, so the individual crosses combine into one continuous
  grid over the whole loaded area, showing at a glance where
  `CHUNKS_BELOW_SEA_LEVEL` starts — the thing `generation.rs`'s terrain band
  is centred on — tied to world position rather than following the camera
  around (an earlier, camera-centred version is in Reversals).

Every gizmo here draws on its own render layer (`GIZMO_LAYER`), which is
added to the world camera only. Gizmos render to every camera whose layers
overlap theirs, and on the shared default layer the UI camera, a fixed
orthographic `Camera2d`, drew a flat copy of them pinned to the middle of
the screen. Any gizmo added elsewhere later needs the same treatment.

All three read an `_INITIALLY_*` constant from `config::debug` at startup
(`WIREFRAME_INITIALLY_VISIBLE`, `CHUNK_GRID_INITIAL_MODE`,
`SEA_LEVEL_LINE_INITIALLY_VISIBLE`), each itself only taking effect while
`TESTING_TOOLS_ENABLED` is on — the same reasoning `world::ChunkLock`'s
starting value uses. The chunk-grid lock's own starting state
(`CHUNK_GRID_INITIALLY_LOCKED`) is the odd one out: it can't just be a
`Default` impl, because locking needs an actual chunk position, and none
exists until the world camera has spawned. `apply_initial_chunk_grid_lock`
runs every frame while `InGame`, does nothing until a camera exists to read
a position from, and then applies the lock exactly once via a `Local<bool>`
"have I already done this" flag — a robust alternative to trying to order
this system after camera spawn explicitly, which would still race the first
time `InGame` is entered.

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
