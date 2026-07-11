# Controls

All keyboard/mouse configuration lives in `src/config.rs` (`camera` for FOV, `controls` for everything below) — that's the one place to change a binding or tune sensitivity/speed. Everything that reads input lives in `src/input/`.

## Global (any state)

- **1 / 2 / 3** — jump directly to Loading / Menu / Playing (`src/input/mod.rs::switch_state`). Dev/debug convenience, not intended to survive into a real menu-driven flow.

## Playing only

- **Tab** — swap `ControlMode::Dev` ⇄ `ControlMode::Player` (`src/input/playing.rs::toggle_control_mode`). See `docs/player-physics.md`.
- **Mouse** — look (yaw/pitch), locked and hidden while playing. Same in both control modes.
- **Escape** — toggle Paused ⇄ Playing (`src/input/playing.rs::toggle_pause`). Only scheduled while `GameState::Playing`, which is also the only time `PauseState` exists at all — see `docs/architecture.md` for why Pause can't be reached any other way.
- **Shift+1** — toggle the terrain wireframe debug overlay (`src/input/playing.rs::toggle_terrain_wireframe`). Bare `Digit1` is reserved for the global Loading-state jump above, so this only fires when Shift is held too — see `docs/world-generation.md`.

### `ControlMode::Dev` (default)

Free-fly, no gravity or collision — the original movement scheme, unchanged.

- **WASD** — move relative to look direction (forward/back/strafe).
- **Space / Left Shift** — fly up / down.

### `ControlMode::Player`

Gravity + a two-block-tall collision box against the terrain — see `docs/player-physics.md`.

- **WASD** — move horizontally, relative to look direction (no flying — projected onto the ground plane).
- **Space** — jump, only while `Grounded`.

All of the above (except the global state-jump keys) live in `src/input/playing.rs` and are gated on `GameState::Playing` (movement/look additionally require `PauseState::Unpaused` — everything freezes, mouse unlocks, while paused).

## Key bindings (`config::controls::KeyBinds`, all `KeyCode`)

| Action | Default |
|---|---|
| forward | `KeyW` |
| backward | `KeyS` |
| left | `KeyA` |
| right | `KeyD` |
| up | `Space` |
| down | `ShiftLeft` |

Mouse sensitivity (`MOUSE_SENSITIVITY`) and `ControlMode::Dev`'s free-fly speed (`MOVE_SPEED`) are plain `f32` consts alongside `KeyBinds` in the same module. `ControlMode::Player`'s on-foot speed (`WALK_SPEED`) and jump/gravity tuning live in `config::player` instead — see `docs/player-physics.md` for why they're deliberately separate constants from `MOVE_SPEED`.
