# OpenMC

An open-source, Minecraft-like voxel sandbox game written in Rust on top of
the [Bevy](https://bevyengine.org/) game engine.

OpenMC is a hobby project built in the open. The goal is to make a real,
playable voxel game while keeping the codebase clean enough that anyone can
pick it up, understand it, and contribute.

> **Status: early development.** The game currently has a loading screen, a
> main menu, a pause screen, and a basic 3D in-game scene with a free-flying
> camera. Terrain, chunks, and block placing/breaking are next.

## Goals

Development follows four pillars, in priority order: **performance**,
**readability**, **modularity**, then **everything else** (docs, tests,
polish). See the [Code of Conduct](#code-of-conduct) for what each one means
in practice.

## Getting Started

### Requirements

- [Rust](https://www.rust-lang.org/tools/install), version **1.98.1** or
  newer (run `rustup update` if you are older)
- A GPU with Vulkan, DirectX 12, or Metal support
- On Linux, Bevy's system dependencies — see
  [Bevy's Linux setup guide](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md)

### Build and run

```sh
cargo run
```

The first build compiles all of Bevy and takes a while. Later builds are
much faster. For a quick check that the code compiles without building the
binary, use `cargo check`.

### Controls

| Key             | Action                   |
| --------------- | ------------------------ |
| `W` `A` `S` `D` | Move                     |
| `Space`         | Fly up                   |
| `Left Ctrl`     | Fly down                 |
| `Left Shift`    | Sprint                   |
| `Esc`           | Pause / resume           |
| `F10`           | Toggle borderless window |
| `F11`           | Toggle fullscreen        |

All key bindings live in [src/config/input.rs](src/config/input.rs).

## Project Layout

The code is organized by game domain, with one Bevy plugin per domain:

```text
src/
├── main.rs      # Entry point — wires the app together, nothing else
├── app/         # App setup, game states, plugin registration
├── config/      # Settings: window, key bindings
├── camera/      # UI camera and 3D world camera
├── loading/     # Loading screen state
├── menu/        # Main menu state
├── ingame/      # The actual game world
├── paused/      # Pause screen state
└── utils/       # Small shared helpers
```

For more detail, see the docs folder:

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — how the pieces fit together
- [docs/IMPROVEMENTS.md](docs/IMPROVEMENTS.md) — why the code is the way it
  is, including decisions that were reversed
- [docs/AUDIT.md](docs/AUDIT.md) — known improvements that are not done yet

## Contributing

Contributions of all sizes are welcome — bug reports, ideas, docs fixes,
and code.

1. **Open an issue first** for anything bigger than a small fix, so we can
   agree on the approach before you spend time on it.
2. **Read the docs** above, especially the decisions table in
   `IMPROVEMENTS.md`, so a change doesn't undo an earlier deliberate choice.
3. **Keep it tidy** before opening a pull request:

   ```sh
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```

4. **Keep pull requests focused** — one feature or fix per PR, with a short
   description of what changed and why.
5. **Update the docs** in the same PR if your change alters how things are
   structured.

Not sure where to start? [docs/AUDIT.md](docs/AUDIT.md) lists work that has
already been identified and is waiting for someone to pick it up.

## Code of Conduct

Every change to OpenMC is judged against four pillars, **in priority
order**. When two of them conflict, the higher one wins — and say so in
your pull request instead of quietly trading one away.

### 1. Performance

This is a voxel game, so speed comes first — both at runtime and at build
time.

- Chunk generation, meshing, and per-frame systems are the hot paths. Don't
  ship an obviously wasteful hot path when a simple better one exists.
- Don't allocate inside systems that run every frame over large queries.
  Reuse buffers, and use `Vec::with_capacity` when the size is known.
- Use Bevy query filters (`With`, `Without`, `Changed<T>`) instead of
  filtering inside the system body.
- Store voxel data in flat, contiguous arrays, not nested `Vec`s.
- Profile before optimizing (`tracy` or `puffin`) — don't guess.
- Build speed counts too: don't add dependencies or feature flags you don't
  need, and don't change the build profiles in `Cargo.toml` without a
  measured reason.

### 2. Readability and discoverability

A newcomer should be able to guess where something lives and be right.

- Name files and folders for what they are *for*, not how they work.
- Every new feature should have one obvious home. If it's unclear where
  your code belongs, ask in the issue before writing it.
- Name systems for what they do (`spawn_chunk_meshes`, not `update`).
- Keep systems small. If one grows past ~50 lines, move the logic into a
  plain function the system calls.
- A layout that is easy to navigate beats one that is "technically
  correct" but hard to follow — this pillar outranks modularity.

### 3. Modularity

Nothing lives in a place it doesn't belong.

- Each game domain is its own Bevy `Plugin` that owns its components,
  resources, events, and systems.
- Plugins are registered in `AppPlugin` and nowhere else.
- Share only what other plugins need, through `pub` items in the module's
  `mod.rs`.
- Components and events are plain data — no logic on them.
- Every module is a folder with a `mod.rs`; `main.rs` is the only loose
  file in `src/`.

### 4. Everything else

Docs, tests, and polish are still expected — just never at the cost of the
three pillars above.

- **Docs:** every `pub` item gets a `///` comment explaining its contract,
  and every module gets a `//!` header saying what it owns. Bigger design
  notes go in [docs/](docs/). If you change a decision, add a row to
  [docs/IMPROVEMENTS.md](docs/IMPROVEMENTS.md).
- **Tests:** pull non-trivial logic out of Bevy systems into plain
  functions and unit-test those. There's no coverage percentage — the bar is
  "the non-trivial logic has tests."
- **Safety and errors:** `unsafe` code is denied. No `unwrap()`/`expect()`
  outside tests; use `thiserror` for typed errors and `anyhow` at the app
  boundary.
- **Style:** `cargo fmt` and `cargo clippy -- -D warnings` must pass.

### Be kind

Respect everyone who takes part. Disagree with ideas, never with people,
and help newcomers find their way.

## License

A license has not been chosen yet. Until one is added, please open an issue
before reusing the code elsewhere.
