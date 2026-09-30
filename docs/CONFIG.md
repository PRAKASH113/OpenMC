# Configuration

How `config/` is organized, and the conventions every file in it follows.
This is a map of the *structure*, not a list of current values — those live
in the source itself, next to the reasoning for each one, so they can't
drift out of sync with this file. Read the `///` comment on a constant for
what it does and why it's set the way it is.

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
