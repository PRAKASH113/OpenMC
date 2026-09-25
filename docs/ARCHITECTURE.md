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
├── main.rs              # The only loose file: module declarations, calls app::run
├── app/                 # App assembly
│   ├── mod.rs           #   run()
│   ├── config.rs        #   Window/startup settings (plain data)
│   ├── window.rs        #   config -> Bevy WindowPlugin translation
│   ├── log.rs           #   Log filters
│   ├── states.rs        #   GameState enum + GameStatePlugin
│   └── plugin.rs        #   AppPlugin: the composition root
├── camera/
│   ├── mod.rs           #   CameraPlugin: registers every camera
│   └── camera_2d.rs     #   The UI camera
├── loading/
│   ├── mod.rs           #   LoadingPlugin
│   └── screen.rs
├── menu/
│   ├── mod.rs           #   MenuPlugin
│   └── screen.rs
├── ingame/
│   ├── mod.rs           #   InGamePlugin
│   └── screen.rs
└── paused/
    ├── mod.rs           #   PausedPlugin
    └── screen.rs
```

Each state is a top-level module that owns everything belonging to it, so
work on one state never touches another's files. `app/states.rs` declares the
`GameState` enum and registers the state machine; it deliberately knows
nothing about what any individual state contains.

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

`app::states::GameState` is the top-level state machine: `Loading`, `Menu`,
`InGame`. `Loading` is the default, so it is what the game starts in.
`GameStatePlugin` in the same module installs it.

`InGameState` (`Playing` / `Paused`) is a **sub-state** of `GameState::InGame`
— Bevy creates it on entering that state and removes it on leaving, so
pausing outside a loaded world is unrepresentable rather than merely
discouraged. See [`INGAME_FOUNDATION.md`](INGAME_FOUNDATION.md).

Each variant has a matching top-level module owning what that state shows and
what runs while it is active. A state's module registers its systems against
`OnEnter`/`OnExit` for its own variant, so adding behaviour to a state means
working inside that state's folder and nowhere else. The four plugins are
registered in `AppPlugin` alongside everything else.

`Paused` is currently a sibling of `InGame` in one flat enum. If pausing
later needs the in-game world to keep existing while its systems stop,
Bevy's `SubStates` is the tool for that — worth revisiting then, not now.

### Placeholder screens

Every state currently renders the same shape of thing: a full-screen colour
with the state's name on it, in that state's own `screen.rs`. Each owns its
colour, label, and marker component, so the four are independent — they are
meant to diverge completely as real content arrives, and none of them should
grow a shared abstraction on the way there.

Two temporary pieces to delete later, both marked in the source:

- `app::states::debug_jump_to_state` — number keys `1`-`4` jump straight to a
  state, since nothing drives transitions yet.
- The placeholder screens themselves.

The camera is deliberately **not** part of this. `camera::CameraPlugin`
registers cameras that are spawned at startup and outlive every state, so
state screens only spawn their own content and never contend over the view.

## Cameras

`camera.rs` registers one plugin per camera kind. Today that is only
`camera_2d`, which renders the UI. The 3D camera for the voxel world drops in
as `camera/camera_3d.rs` with its own plugin added to the tuple in
`CameraPlugin` — no other file changes.

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

These constants describe **startup** only. Window size, present mode and
decorations are all live-mutable on the `Window` component, so a settings
menu would change that component directly and persist to its own file rather
than touching these. Constants become a struct the day something needs to
load them from disk at startup — not before.

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

`app::states::log_state_change` logs every state transition, skipping
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
