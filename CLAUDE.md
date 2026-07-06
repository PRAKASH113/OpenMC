# OpenMC

A Minecraft-like voxel sandbox game written in Rust using the **Bevy** game engine.

## Project Goals

- Voxel-based world made of blocks (chunks, terrain generation, block placement/breaking).
- First-person player controller with camera and movement.
- Chunk meshing and rendering optimized for large voxel worlds.
- Basic survival/creative gameplay loop (inventory, block interaction).

## Tech Stack

- **Language:** Rust (2024 edition)
- **Engine:** Bevy (ECS-based game engine)
- Additional crates will be added as needed for noise/terrain generation, math, and voxel meshing.

## Conventions

- Follow idiomatic Bevy ECS patterns: Components, Resources, Systems, Plugins.
- Keep gameplay logic organized into Bevy plugins per feature (e.g. `world`, `player`, `rendering`).
- **Whenever a new crate is added to `Cargo.toml`, add a one-line comment next to it explaining its use in this project.**
- **This file stays short — a map, not the manual.** Detailed reasoning, design decisions, and "why" writeups belong in `docs/*.md`, indexed below. When a change is significant enough to need explaining, update (or add) the relevant doc in the same change — don't let this file or the docs drift from the code.

## Module Map

| Path | What's there |
| --- | --- |
| `src/main.rs` | Entry point: window/log plugins, hands off to `app::AppPlugin` + `input::InputPlugin`. |
| `src/config.rs` | All configuration, in one place: `window`, `camera` (FOV), `controls` (keybinds, mouse sensitivity, move speed). |
| `src/app/` | `states.rs` (`GameState`, `PauseState`), `screen.rs` (shared 2D-screen helper), `mod.rs` (`AppPlugin`, composition root). |
| `src/input/` | The entire control scheme. |
| `src/loading/`, `src/menu/` | Placeholder 2D screens (stubs). |
| `src/paused/` | Pause overlay, layered on top of Playing. |
| `src/playing/` | The 3D scene: camera, sky dome, sun, chunked terrain (`world/`) at full `RENDER_DISTANCE = 4`. |
| `src/dev_tools/` | Single performance HUD (FPS, frame time, triangle count, a `PlayingScreen`-scoped game-entity count, CPU/mem). |

## Documentation Index

- [`docs/architecture.md`](docs/architecture.md) — state machine (`GameState`/`PauseState`), why pause layers on top of Playing instead of replacing it, module responsibilities.
- [`docs/sky-and-sun.md`](docs/sky-and-sun.md) — sky dome + sun design; reconnected, see `docs/performance-investigation.md`.
- [`docs/world-generation.md`](docs/world-generation.md) — the chunk manager/generator/binary-greedy-mesher pipeline; one entity per chunk, never one per block. Fully reconnected at `RENDER_DISTANCE = 4`.
- [`docs/controls.md`](docs/controls.md) — keybinds, mouse look, pause toggle.
- [`docs/performance.md`](docs/performance.md) — the diagnostics HUD, optimizations made so far and why, candidates not yet done.
- [`docs/optimisations.md`](docs/optimisations.md) — forward-looking optimization roadmap (vertex packing, instancing, indirect draw batching); on hold, see `docs/performance-investigation.md`.
- [`docs/performance-investigation.md`](docs/performance-investigation.md) — **active**: the Playing scene was stripped to a baseline to isolate a real FPS-crash bug, confirmed healthy, and rebuilt back up one piece at a time (sky dome → single chunk → full render distance), checking stats and visual quality after each step.
