# Configuration

How `config/` is organized, the conventions every file in it follows, and —
below that — every setting it currently holds, grouped by file, with its
value and what it does.

The reference section is a snapshot: `config/`'s source, with its `///`
comments, is still the source of truth, and this file can drift out of date
the moment a value changes without the doc being updated in the same commit.
When in doubt, or when this file looks stale, read the constant itself —
its doc comment carries the full reasoning that a table row here only
summarizes.

## One file per thing configured

Each file in `config/` groups settings by *what they affect*, not by type:

| File | Owns |
| --- | --- |
| `window.rs` | Title, size, borderless, fullscreen, present mode |
| `input.rs` | Key bindings, mouse sensitivity, movement speeds, sprint |
| `camera.rs` | Field of view, third-person distance and pivot height |
| `world.rs` | Chunk size, render distance, world height, terrain noise |
| `player.rs` | Game mode, gravity, jump, terminal velocity, hitbox |
| `debug.rs` | The testing-tools master switch, and every testing feature's starting state |

Adding a setting means picking the file for what it configures — if none
fits, that's usually a sign a new domain needs its own file, not that an
existing one should absorb an unrelated concern.

## What belongs here vs. in the domain module

`config/` holds plain values: constants, and the occasional small enum for a
choice with more than two states. It never contains logic, and never touches
the engine — turning a value into an actual Bevy setting (a `Projection`, a
`WindowResolution`, a spawned entity) is the consuming module's job, not
`config`'s.

The reverse also holds: a value that isn't actually a *setting* — something
a person would tune — stays with the code that owns it instead of moving
here for its own sake. Camera draw order and the look-pitch clamp are both
examples: they're implementation details or safety limits, not preferences,
so they live in `camera_world.rs` and `input::look` respectively, not in
`config::camera`.

## The settings-surface enum pattern

A handful of settings are a choice between named modes, not a single value:
`config::input::SprintMode`, `config::debug::ChunkGridMode`,
`config::player::GameMode`. All three follow the same split:

- **The enum itself, and which variant is active, live in `config`.** That's
  the part someone would actually reach in to change.
- **What each variant *does* belongs to the module that acts on it** —
  `input::movement` for `SprintMode`, `render::debug` for `ChunkGridMode`,
  `player::game_mode` for `GameMode`.

This keeps a mode switch a one-line change in `config` (or, for `GameMode`
and `ChunkGridMode`, a runtime hotkey) without the module that interprets it
needing to change at all.

## The two-gate debug pattern

Every testing-only feature (`world::debug`, `render::debug`, `player::debug`)
is gated twice, not once:

1. **`#[cfg(debug_assertions)]`** — compiled out of release entirely. A
   testing tool that could theoretically reach a shipped build is one that
   wasn't gated tightly enough.
2. **`config::debug::TESTING_TOOLS_ENABLED`** — a plain runtime `bool`, debug
   builds only. Being a debug build isn't the same as *actively testing*
   right now, so this stays independent: with it `false`, a testing hotkey's
   system is never even added to the schedule, not merely skipped.

On top of both, every testing hotkey also requires
[`crate::config::player::GameMode::Creative`] — Survival is meant to play
the way the game ships, so testing tools stay out of it regardless of the
two gates above.

A few features go further and expose an `_INITIALLY_*` constant —
`CHUNK_LOCK_INITIALLY_ENGAGED`, `WIREFRAME_INITIALLY_VISIBLE`,
`COLLISION_INITIALLY_DISABLED`, and the like — so a feature can start already
active instead of needing its hotkey pressed first. Each of these is itself
gated on `TESTING_TOOLS_ENABLED`: without that guard, a feature could start
in a state its own hotkey doesn't exist to undo (a permanently locked world,
collision that never comes back on), since the hotkey is only registered
when testing tools are enabled in the first place.

The state a testing feature acts on (`world::ChunkLock`,
`player::physics::CollisionEnabled`) is itself a normal, unconditional
resource or component — only the *hotkey that changes it* lives behind
`#[cfg(debug_assertions)]`. That's what lets the systems that read it (chunk
loading, player physics) stay identical in every build: always "off" in
release, since nothing there can ever set it otherwise, rather than needing
an `Option` or a second code path.

## Adding a setting

- Decide which file it configures (the table above), or whether it needs a
  new one.
- If it's a genuine player-facing choice with more than two states, consider
  the settings-surface enum pattern.
- If it's a testing-only toggle, follow the two-gate pattern, including an
  `_INITIALLY_*` constant if starting pre-activated is useful.
- Document its contract with a `///` comment — what it does, its unit if
  it's a physical quantity, and *why* the default is what it is when that
  isn't obvious. That comment is the source of truth this file deliberately
  doesn't duplicate.

See [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) for how each domain module
that *reads* these settings fits together, and
[`docs/IMPROVEMENTS.md`](IMPROVEMENTS.md) for why particular values or
patterns were chosen.

---

## Reference: every setting, by file

### `window.rs`

| Constant | Value | What it does |
| --- | --- | --- |
| `TITLE` | `"OpenMC"` | Shown in the window header and the OS task switcher. |
| `WIDTH` | `1280` | Starting window width, logical pixels (points). |
| `HEIGHT` | `720` | Starting window height, logical pixels. |
| `BORDERLESS` | `false` | Start without the OS title bar and border. Only the starting value — `F10` flips it at runtime. |
| `FULLSCREEN` | `false` | Start in fullscreen. Only the starting value — `F11` flips it at runtime; leaving it restores `WIDTH` x `HEIGHT`. |
| `PRESENT_MODE` | `PresentMode::AutoVsync` | How finished frames reach the display. `AutoVsync`/`AutoNoVsync` pick the best mode the driver supports; `Fifo`, `Mailbox`, `Immediate` force one specific mode. |

### `input.rs`

Movement:

| Constant | Value | What it does |
| --- | --- | --- |
| `FORWARD` | `W` | Move the way the player is facing, level with the ground. |
| `BACKWARD` | `S` | Move opposite the player's facing. |
| `LEFT` | `A` | Strafe left. |
| `RIGHT` | `D` | Strafe right. |
| `UP` | `Space` | Jump on foot; double-tap to take off in Creative; rise while flying (double-tap and hold for double speed). |
| `DOWN` | `Left Shift` | Descend while flying; double-tap to land in Creative. Does nothing on foot yet. |

Game:

| Constant | Value | What it does |
| --- | --- | --- |
| `PAUSE` | `Escape` | Pause and resume. |
| `TOGGLE_GAME_MODE` | `F4` | Switch between `config::player::GameMode`s. Works in every mode. |

Window:

| Constant | Value | What it does |
| --- | --- | --- |
| `TOGGLE_BORDERLESS` | `F10` | Toggle the OS title bar and border. |
| `TOGGLE_FULLSCREEN` | `F11` | Toggle fullscreen; leaving it restores the configured window size. |

Control settings:

| Constant | Value | What it does |
| --- | --- | --- |
| `LOOK_SENSITIVITY` | `0.002` | Radians of view rotation per pixel of mouse movement. |
| `WALK_SPEED` | `4.3` | Movement speed on foot, blocks/second (Minecraft: ~4.3). |
| `FLY_SPEED` | `12.0` | Movement speed while flying, blocks/second, every axis. |
| `FAST_ASCENT_MULTIPLIER` | `2.0` | Extra rise speed after a double-tap of `UP` while flying, held. |
| `SPRINT_MULTIPLIER` | `3.0` | Speed multiplier while sprinting, on foot or flying. |
| `SprintMode` (enum) | — | `DoubleTap` (any movement key, stays engaged until all released) or `Hold` (a dedicated key). Which variant is live is a compile-time choice, not a runtime one. |
| `SPRINT_MODE` | `SprintMode::DoubleTap` | Which trigger is active. |
| `DOUBLE_TAP_WINDOW` | `0.3` (seconds) | How quickly two presses of a key must follow each other to count as a double-tap — sprint, take off, land, fast ascent. |
| `SPRINT_HOLD_KEY` | `Left Control` | Held to sprint when `SPRINT_MODE` is `Hold`. |

Debug (`#[cfg(debug_assertions)]`, and testing-tool keys additionally gated by `TESTING_TOOLS_ENABLED` and `GameMode::Creative` — see "The two-gate debug pattern" above):

| Constant | Value | What it does |
| --- | --- | --- |
| `DEBUG_GOTO_LOADING` | `1` | Jump straight to the loading state. |
| `DEBUG_GOTO_MENU` | `2` | Jump straight to the menu. |
| `DEBUG_GOTO_INGAME` | `3` | Jump straight into a world. The only way into a world at all right now, so not gated on `Creative`. |
| `TOGGLE_CHUNK_LOCK` | `F9` | Freeze whichever chunks are currently loaded. See `world::debug`. |
| `TOGGLE_WIREFRAME` | `F8` | Toggle wireframe rendering on every chunk mesh. See `render::debug`. |
| `TOGGLE_CHUNK_GRID` | `F7` | Cycle the chunk-bounds grid around the player's chunk. See `render::debug`. |
| `DEBUG_MODIFIER` | `Left Alt` | Held with `TOGGLE_CHUNK_GRID` to lock the grid instead of cycling it. |
| `TOGGLE_SEA_LEVEL_LINE` | `F6` | Toggle the sea-level marker grid. See `render::debug`. |
| `TOGGLE_COLLISION` | `F5` | Toggle the player's terrain collision (noclip). See `player::debug`. |

A unit test (`no_key_is_bound_to_two_actions`) fails if any two of the above share a key.

### `camera.rs`

| Constant | Value | What it does |
| --- | --- | --- |
| `FOV_DEGREES` | `60.0` | Vertical field of view. Bevy's default is 45°; a common "FPS" feel is 90-110°. |
| `THIRD_PERSON_DISTANCE` | `4.0` (blocks) | How far the camera sits back from its pivot. Minecraft's own third-person view uses 4. |
| `THIRD_PERSON_PIVOT_HEIGHT` | `1.6` (blocks) | Height above the player's feet the camera orbits and looks at — roughly eye level on the 2-block model. |

### `player.rs`

| Constant | Value | What it does |
| --- | --- | --- |
| `GameMode` (enum) | — | `Survival` (gravity always applies, no flight, testing tools off) or `Creative` (can fly, testing tools on). Names are placeholders borrowed from Minecraft. |
| `INITIAL_GAME_MODE` | `GameMode::Creative` | Which `GameMode` the game starts in. |
| `CREATIVE_STARTS_FLYING` | `false` | Whether entering Creative starts the player already flying. |
| `GRAVITY` | `32.0` (blocks/s²) | Downward acceleration while not flying. Minecraft's is about 32. |
| `JUMP_SPEED` | `8.9` (blocks/s) | Upward speed a jump starts with. Clears a one-block step (peak height ≈ 1.24 blocks). |
| `TERMINAL_VELOCITY` | `60.0` (blocks/s) | Fastest the player can fall, however long the fall. |
| `HITBOX_WIDTH` | `0.6` (blocks) | Width of the player's collision box on both horizontal axes — narrower than the 1-block model, like Minecraft's 0.6 hitbox. |
| `HITBOX_HEIGHT` | `1.9` (blocks) | Height of the collision box from the feet up — slightly under the model's 2 blocks. |
| `RESET_SPRINT_ON_COLLISION` | `true` | Whether hitting a wall cancels a `SprintMode::DoubleTap` sprint already engaged. Only affects `DoubleTap`. |

### `world.rs`

| Constant | Value | What it does |
| --- | --- | --- |
| `CHUNK_SIZE` | `32` (blocks) | Side length of a chunk on all three axes. |
| `RENDER_DISTANCE` | `6` (chunks) | Horizontal load radius around the player's chunk column, as a circle (by squared distance), not a bounding square. |
| `CHUNKS_ABOVE_SEA_LEVEL` | `1` | Chunk layers above sea level (`y = 0` up to `y = CHUNKS_ABOVE_SEA_LEVEL - 1`). Set low while vertical loading is new; intended target `20`. |
| `CHUNKS_BELOW_SEA_LEVEL` | `1` | Chunk layers below sea level. Set low for the same reason; intended target `12`. |
| `TERRAIN_SEED` | `0` | Seed for the terrain noise. Same seed, same world, every time. |
| `TERRAIN_OCTAVES` | `4` | How many octaves of noise are summed to build the heightmap (fractal Brownian motion). More = more detail, more cost. |
| `TERRAIN_FREQUENCY` | `0.01` | Cycles of the base noise octave per block. Smaller = broader, smoother hills. |
| `TERRAIN_PERSISTENCE` | `0.5` | How much each successive octave's contribution shrinks. Closer to 0 = smoother; closer to 1 = rougher. |
| `TERRAIN_AMPLITUDE` | `10.0` (blocks) | Height variation from sea level in either direction — the surface never strays past ±this value. |

### `debug.rs`

`#[cfg(debug_assertions)]` throughout — every constant here compiles out of release entirely.

| Constant | Value | What it does |
| --- | --- | --- |
| `TESTING_TOOLS_ENABLED` | `true` | Master switch for every testing feature's hotkey. `false` removes the hotkey systems from the schedule entirely, not just skips them. |
| `CHUNK_LOCK_INITIALLY_ENGAGED` | `false` | Whether chunk locking starts engaged on world load, instead of needing `F9` first. Only takes effect while `TESTING_TOOLS_ENABLED` is `true`. |
| `WIREFRAME_INITIALLY_VISIBLE` | `false` | Whether chunk wireframes are visible from the start, instead of needing `F8` first. |
| `ChunkGridMode` (enum) | — | `None` (default), `Outline`, or `OutlineAndAxes` — the chunk-bounds grid's display mode. |
| `CHUNK_GRID_INITIAL_MODE` | `ChunkGridMode::None` | Which mode the chunk-bounds grid starts in, instead of needing `F7` pressed first. |
| `CHUNK_GRID_INITIALLY_LOCKED` | `false` | Whether the chunk-bounds grid starts locked to the player's spawn chunk, instead of needing `Alt+F7` first. |
| `SEA_LEVEL_LINE_INITIALLY_VISIBLE` | `false` | Whether the sea-level marker is visible from the start, instead of needing `F6` first. |
| `COLLISION_INITIALLY_DISABLED` | `false` | Whether the player passes through terrain from the start, instead of needing `F5` first. Always re-enabled on entering Survival regardless. |

Every `_INITIALLY_*` constant above only takes effect while `TESTING_TOOLS_ENABLED` is also `true` — see "The two-gate debug pattern" above.
