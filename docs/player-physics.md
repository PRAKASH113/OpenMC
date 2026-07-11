# Player Physics: Gravity, Collision, Control Modes

## `ControlMode`: Dev vs. Player

`src/playing/player.rs`'s `ControlMode` is a `SubState` of `GameState::Playing` (same pattern as `PauseState` — only exists while Playing), toggled with **Tab** (`input::playing::toggle_control_mode`):

- **`Dev`** (default): the original free-fly camera, completely unchanged — no gravity, no collision, Space/Shift fly up/down. Every terrain/lighting/performance round so far used this mode, and it stays the default so that work isn't disrupted.
- **`Player`**: gravity constantly pulls down, a collision box stops the camera at solid terrain, WASD is horizontal-only (no flying), Space jumps only when grounded, and a crosshair appears center-screen.

Switching modes resets `PlayerVelocity` and `Grounded` to zero/false (`player::reset_physics`, `OnEnter(ControlMode::Player)`) so the first frame in Player mode starts from a known state, not whatever was left over from the last time it was active.

## The collision box

`config::player`: `WIDTH = 0.6`, `HEIGHT = 2.0` (two blocks tall, as requested), `DEPTH = 0.6` (width/depth weren't specified, so picked to match a typical humanoid footprint at this world's block scale — close to Minecraft's own 0.6-wide player box). `EYE_HEIGHT = 1.7` is how far the camera sits above the box's feet — near the top, not dead center, matching how real eyes sit near the top of a body.

The camera's `Transform` is still the single source of truth for the player's position (no separate "player body" entity) — each frame, `player_movement_and_look` derives the collision box's feet position as `camera.translation - Vec3::Y * EYE_HEIGHT`, resolves movement against that, then converts back: `camera.translation = new_feet + Vec3::Y * EYE_HEIGHT`.

## Gravity, jump, and movement speed tuning

`config::player`: `GRAVITY = 32.0` blocks/s² — chosen to match Minecraft's own constant (`0.08 blocks/tick²` at 20 ticks/s = 32 blocks/s²) rather than real-world 9.8 m/s², which reads as floaty at this block scale. `TERMINAL_VELOCITY = 40.0` caps fall speed — see "Why the collision resolver sweeps the whole path" below for why an unbounded fall speed would be dangerous, not just unrealistic.

`JUMP_VELOCITY = 8.4` (`v²/(2·g) ≈ 1.1` blocks) — lowered from an initial `9.0` (`≈1.27` blocks, matching Minecraft's own jump height almost exactly) after feedback that it read as too high; the request was "just a bit over one block."

`WALK_SPEED = 5.0` is a **separate** constant from `controls::MOVE_SPEED` (`ControlMode::Dev`'s free-fly speed, still `8.0`) — they used to be the same shared constant, but slowing down on-foot walking would have also slowed down `Dev` mode's fast-travel flying, which every terrain/lighting/performance round in this project has relied on. Splitting them means each can be tuned independently: `Dev` stays fast for inspecting the world, `Player` walks at a slower, more human pace.

## Why chunk data had to stop being thrown away

Until now, `world::manager::load_pending_chunks` generated a chunk's raw `ChunkData`, fed it into the mesher, and discarded it — only the rendered `Mesh` survived (and even that only for chunks with any visible geometry — see `docs/world-generation.md`'s "Vertical chunk stacking" for buried chunks that skip spawning an entity at all). Collision needs to answer "is this world position solid" at runtime, which a `Mesh` can't answer efficiently. `ChunkManager.loaded` now stores a `LoadedChunk { entity: Option<Entity>, data }` per chunk instead of just an `Entity`, and `ChunkManager::is_solid(block: IVec3) -> bool` looks up the block directly: `false` for `AIR`, for `WATER` (opaque-looking but not physically solid — the player walks/falls through it, see `docs/world-generation.md`'s "Block types"), and for any chunk that isn't currently loaded (a player standing right at the render-distance edge falling into an unloaded chunk is treated as air rather than an invisible wall).

Memory cost is negligible: `PADDED_SIZE * PADDED_HEIGHT * PADDED_SIZE` bytes per chunk (34×33×34 ≈ 38KB) × chunks currently loaded (now several vertical layers per horizontal column, see `docs/world-generation.md`) ≈ still a few MB total — consistent with `docs/optimisations.md`'s earlier finding that chunk data was never where this project's memory was going.

## The collision resolver (`player::resolve_movement`)

Axis-by-axis resolution (X, then Y, then Z — order doesn't actually matter here since each axis only reads the box's *already-resolved* position from the previous axis, never the original pre-frame position), so the player slides along a wall or floor instead of stopping dead the instant *any* single axis would collide.

For each axis: tentatively apply that axis's full share of `delta`, and if the box would overlap a solid block anywhere along that movement, snap back to the exact block boundary nearest the start position, in the direction of travel — computed directly (closest colliding block's own edge), not approximated by stepping. Blocks are unit-aligned, so this is exact for any `delta` magnitude, not just small per-frame steps.

### A real bug the test suite caught before this ever ran in-game

The first version only checked the box's **final** position for overlaps, not the path it swept through to get there. That's wrong: a large enough single-frame `delta` (a big fall, or a lag spike) can jump clean past a thin obstacle — the final position doesn't overlap anything even though the path did, so the player tunnels straight through undetected. 4 of the 6 test cases in `player.rs`'s `#[cfg(test)]` module caught this immediately (`lands_exactly_on_top_of_the_ground` had the box falling *through* a floor at y=4, resting at y=-10 instead of y=5). Fixed by querying the box's full swept volume for each axis (the current position on the other two axes, extended across the *entire* range this axis moves through) instead of just the endpoint. This is exactly the scenario `TERMINAL_VELOCITY` exists to keep small in normal play — the fix makes the resolver correct regardless, but capping fall speed keeps the swept query volume (and thus the number of blocks checked) small too.

Verified with 6 tests (free fall, landing exactly on a floor, sliding along a wall while another axis moves freely, stopping at a boundary approached from the negative direction, a no-op when delta is zero, and the large-single-frame-delta tunneling case) — all check the actual invariant that matters (the resolved box never overlaps a solid block), not just one hand-picked expected position, following the same "verify programmatically since visual testing isn't available to me" approach as `mesher.rs`'s greedy-merge tests.

## Crosshair

A simple centered "+" (`player::spawn_crosshair`/`despawn_crosshair`, `OnEnter`/`OnExit(ControlMode::Player)`) — two small `Node` rectangles (16px long, 2px thick) positioned via percentage-centering plus a negative pixel offset to center on their own size, a standard Bevy UI centering trick. No image asset needed for something this simple.

## Not yet done

- No head bob, no crouch, no sprint — WASD is a flat `WALK_SPEED` in Player mode.
- No swimming mechanics — water is non-solid (see above) but there's no buoyancy, no reduced gravity/move speed in water, no drowning. Walking into water currently just means falling straight through it under normal gravity.
- No block placement/breaking yet — the crosshair is purely visual so far, not wired to any interaction.
- Collision is discrete-per-frame, not continuously swept across *diagonal* movement (each axis is swept independently, which is exact for axis-aligned movement but is the standard, well-tested simplification for combined diagonal motion too — matches what most simple voxel-game collision implementations do).
