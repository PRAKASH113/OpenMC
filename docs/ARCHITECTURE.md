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
│   ├── loading/             #   Boot loading (solid), exits to Menu
│   │   ├── mod.rs
│   │   └── screen.rs
│   ├── menu/
│   │   ├── mod.rs
│   │   └── screen.rs
│   └── ingame/              #   A loaded world
│       ├── mod.rs
│       ├── scene.rs         #     The 3D scene
│       ├── pause.rs         #     Escape toggles Playing <-> Paused
│       └── paused/          #     Sub-state: lives inside its parent
│           ├── mod.rs
│           └── screen.rs
├── camera/
│   ├── mod.rs               #   CameraPlugin: registers every camera
│   ├── camera_ui.rs         #   Draws the interface
│   └── camera_world.rs      #   Renders the world; look and fly controls
├── config/                  # Every configurable value, grouped by what it configures
│   ├── window.rs            #   Title, size, borderless, fullscreen, present mode
│   └── input.rs             #   Key bindings, sensitivity, speed
└── utils/                   # Finished adapters between our settings and the engine
    ├── window.rs            #   Builds the window; runtime borderless/fullscreen
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
chunk rendering, materials), `player/` (controller, block interaction),
`ui/` (HUD, debug overlay).

## Composition

Every domain is a Bevy `Plugin`, and `app::plugin::AppPlugin` is the single
place they are registered. `app::run` does nothing but add `AppPlugin` and
run, so the list of what the game is made of reads top-to-bottom in one file.

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

- `states::debug_jump_to_state` — keys `1`-`3` jump to `Loading`, `Menu`,
  `InGame`. There is deliberately no key for `Paused`; it is only reachable
  by pausing. Jumping to `Loading` bounces straight back to `Menu`, since
  loading has nothing to wait for. Debug builds only.
- The placeholder screens in `loading/` and `menu/`.

## Cameras

`camera/mod.rs` registers one plugin per camera kind:

- `camera_ui` draws the interface. Spawned at startup, lives for the whole
  run, `order: 1`, no MSAA, and it does **not** clear the screen. It is built
  from Bevy's `Camera2d` component, but it is a UI camera rather than a 2D
  game camera — nothing draws sprites through it.
- `camera_world` renders the world. Spawned on entering `InGame` and
  despawned on leaving, so nothing 3D renders while in a menu. `order: 0`,
  `Sample4` MSAA, and it does the clearing.

Both are named for *what they show*, not how they render — `camera_ui` and
`camera_world` rather than 2D and 3D.

Draw order is the one contract between them: the UI sits above the world.
Because the UI camera never clears, **any state without a world camera must
paint an opaque full-screen background** — `loading/` and `menu/` do — or
bring its own camera.

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
window, `input.rs` for controls — and adding a category later (graphics,
audio) means adding a file, not making a decision.

Nothing in `config/` knows about the engine beyond the types a value needs.
Turning values into engine settings belongs to whoever consumes them.

`config::input` holds every key binding, so the full set is visible at once
and systems call `keys.pressed(input::FORWARD)` rather than naming a
`KeyCode` inline. A unit test there fails if two actions share a key. It
checks a hand-maintained list, so it is only as complete as whoever last
added a binding — that cost was accepted to keep the constants plainly
readable rather than generated by a macro.

Values that are not player-facing settings stay with their owner: camera draw
order is a rendering detail, and the pitch clamp is a safety limit rather than
a taste setting, so both live in `camera_world`.

There is no `input/` module yet. When input *behaviour* arrives — action
mapping, rebinding, gamepad support — it gets one, and `config::input` keeps
holding the values it reads. Bindings are settings; interpreting them is
behaviour.

## Utilities

`utils/` holds modules that adapt our settings to the engine and are
*finished*: `window.rs` creates the window and owns the runtime borderless
and fullscreen toggles, `log.rs` decides which log targets print.

The bar for belonging there is completeness, not size — a module still
growing a design belongs with the domain it serves. Keeping them out of
`app/` leaves that folder holding only the parts that shape how the game is
assembled: the composition root and the state machine.

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
