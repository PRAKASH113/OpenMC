# Architecture

## State Machine

`src/app/states.rs` defines two states:

- **`GameState`**: `Loading`, `Menu` (default/starting state), `Playing`. Each has its own screen module (`src/loading/`, `src/menu/`, `src/playing/`) that spawns its content `OnEnter` and despawns it `OnExit`. Number keys 1/2/3 jump directly between them (`src/input/mod.rs`) — a deliberate dev/debug shortcut, not meant to survive into a real menu-driven flow.
- **`PauseState`** (`Unpaused`/`Paused`): a Bevy `SubStates`, scoped to `GameState::Playing` via `#[source(GameState = GameState::Playing)]`. This is the key design point: `PauseState` *only exists* while playing — there's no direct way to enter Paused from Menu/Loading, because there's nothing to toggle outside of Playing. This is enforced structurally, not by convention.

### Why pause layers on top of Playing instead of replacing it

Every other state transition (Loading → Menu → Playing) fully tears down the previous screen and spawns the next one from scratch. Pause is different on purpose: pressing Escape must **not** despawn the camera/sky/sun/ground, because the whole point of pausing is that the frozen game world stays visible behind a dim overlay.

This is why Paused is a separate `SubStates` rather than a fourth `GameState` variant (which is what the very first version of this project did, and had to be corrected away from): if Paused were a `GameState` value, entering it would fire `OnExit(GameState::Playing)`, tearing down everything Playing owns. As a `SubStates` layered on top of `GameState::Playing`, entering `PauseState::Paused` changes nothing about `GameState` at all — Playing's `OnEnter`/`OnExit` never fire, so its world persists untouched.

Practical consequences of this split:
- `src/paused/mod.rs`'s `PausedPlugin` reacts to `OnEnter`/`OnExit(PauseState::Paused)`, spawning/despawning only a translucent overlay + "Paused" text — no camera of its own (the Playing scene's `Camera3d` is still active and is what Bevy UI renders onto).
- `src/input/playing.rs`'s `movement_and_look` system is gated on `in_state(GameState::Playing).and_then(in_state(PauseState::Unpaused))` — movement freezes while paused, but nothing is torn down.
- The cursor is released (visible, free) on `OnEnter(PauseState::Paused)` and re-locked on `OnEnter(PauseState::Unpaused)`, independent of the Playing-wide lock/release on `OnEnter`/`OnExit(GameState::Playing)`.
- If `GameState` changes away from `Playing` while paused (e.g. a debug number-key jump), Bevy's sub-state machinery still correctly fires `OnExit(PauseState::Paused)` as part of removing the sub-state — confirmed by reading `bevy_state`'s `internal_apply_state_transition`, which is the same core transition function used for every kind of state, computed state, and sub-state. So the pause overlay never leaks/dangles.

## Module Layout

- `src/main.rs` — entry point only: builds the `App` with window/log plugins, hands off to `app::AppPlugin` + `input::InputPlugin`.
- `src/config.rs` — all configuration in one place: `window`, `camera` (FOV), `controls` (`KeyBinds`, mouse sensitivity, move speed). See `docs/controls.md`.
- `src/app/` — `states.rs` (above), `screen.rs` (shared 2D-screen spawn helper), `mod.rs` (`AppPlugin`, the composition root that wires every other plugin together).
- `src/input/` — the entire control scheme. See `docs/controls.md`.
- `src/loading/`, `src/menu/` — placeholder 2D screens (colored background + text), stubs for later.
- `src/paused/` — the pause overlay (see above).
- `src/playing/` — the actual 3D scene: camera, sky dome + sun (`docs/sky-and-sun.md`), chunked terrain (`docs/world-generation.md`). Owns its own screen lifecycle like every other `GameState` variant.
- `src/dev_tools/` — the performance HUD. See `docs/performance.md`.

## Documentation Index

- `docs/architecture.md` — this file: state machine, module responsibilities.
- `docs/sky-and-sun.md` — sky dome + sun design decisions, planned future sky features.
- `docs/world-generation.md` — chunk manager, generator, binary greedy mesher: the "one entity per chunk" pipeline.
- `docs/controls.md` — keybinds, mouse look, pause toggle.
- `docs/performance.md` — the diagnostics HUD and optimization decisions made so far, and why.

**Keep this index and the module-layout summaries above up to date as things change** — when a change is significant enough to need explaining, put the explanation in the relevant `docs/*.md` file (or a new one, added to the index above) rather than growing `CLAUDE.md` — it should stay a short pointer, not the detailed record.
