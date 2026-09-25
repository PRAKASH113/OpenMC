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

- **Fast.** Chunk generation, meshing, and rendering are the hot paths and
  are treated that way. Build times matter too.
- **Easy to find your way around.** File and folder names say what they are
  for, so a newcomer can guess where something lives and be right.
- **Modular.** Each part of the game is its own Bevy plugin that owns its
  own data and systems.
- **Documented and tested** where it counts.

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

OpenMC should be a friendly place for everyone, whatever their experience
level or background. By taking part in this project you agree to:

- **Be respectful.** Treat others the way you would want to be treated.
  Disagree with ideas, never attack people.
- **Be welcoming.** Everyone was a beginner once. Answer questions patiently
  and help newcomers find their way.
- **Be constructive.** When reviewing or giving feedback, explain what could
  be better and why.
- **Be collaborative.** Credit other people's work, and be open to
  suggestions on your own.

Harassment, insults, discrimination, and personal attacks are not tolerated
in issues, pull requests, or any other project space. Contributions or
comments that break these rules may be removed, and repeated or serious
violations may lead to a ban from the project.

If you see or experience a problem, please contact the project maintainer.

## License

A license has not been chosen yet. Until one is added, please open an issue
before reusing the code elsewhere.
