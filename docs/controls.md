# Controls

All keyboard/mouse configuration lives in `src/config.rs` (`camera` for FOV, `controls` for everything below) — that's the one place to change a binding or tune sensitivity/speed. Everything that reads input lives in `src/input/`.

## Global (any state)

- **1 / 2 / 3** — jump directly to Loading / Menu / Playing (`src/input/mod.rs::switch_state`). Dev/debug convenience, not intended to survive into a real menu-driven flow.

## Playing only

- **WASD** — move relative to look direction (forward/back/strafe).
- **Space / Left Shift** — up / down.
- **Mouse** — look (yaw/pitch), locked and hidden while playing.
- **Escape** — toggle Paused ⇄ Playing (`src/input/playing.rs::toggle_pause`). Only scheduled while `GameState::Playing`, which is also the only time `PauseState` exists at all — see `docs/architecture.md` for why Pause can't be reached any other way.

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

Mouse sensitivity (`MOUSE_SENSITIVITY`) and move speed (`MOVE_SPEED`) are plain `f32` consts alongside `KeyBinds` in the same module.
