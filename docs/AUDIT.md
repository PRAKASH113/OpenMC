# Audit

**Audit #2 — 2026-09-26.** Replaces audit #1 (2026-09-19) completely; every
item from #1 was either done, recorded in [`IMPROVEMENTS.md`](IMPROVEMENTS.md),
or carried forward below with its current status.

**Updated repeatedly through 2026-09-27, not re-run in full.** Tier 1 was
worked through item by item rather than all at once: 1.1 (chunk storage
order) and 1.2 (`Changed<Transform>` filter) done; 1.3's linker config
restored, then actually enabled and confirmed working; 1.4 (Bevy feature
trim) done, confirmed working; 1.5 (`opt-level`) reconfirmed unchanged; 1.6
mostly done — real terrain generation (`noise`), the mesher swapped for
`binary_greedy_meshing` (cross-chunk face culling included), and chunk
generation itself moved to a background thread, leaving only meshing's own
main-thread cost open (1.6a, below). Along the way, building `render/` had
already separately resolved 2.3, 3.2, 4.1, and 4.3, and fixed a doc comment
2.5 flagged. All of the above are done and removed from this file; the full
history of *why*, in order, is in `IMPROVEMENTS.md`'s 2026-09-27 entries —
this file only tracks what's still open. Everything below is otherwise
unchanged from audit #2 and has not been re-checked against the current
code.

**Updated again, 2026-09-28.** Vertical chunk loading was wired in ("Option
A": every horizontally-in-range column loads its whole fixed height, from
`CHUNKS_ABOVE_SEA_LEVEL` down to `CHUNKS_BELOW_SEA_LEVEL`, previously
commented out and unread), alongside four testing-only visual debug features
(`render::debug`: a wireframe toggle, a three-mode chunk-bounds grid with a
lock, and a sea-level marker line) following the existing
`TESTING_TOOLS_ENABLED` pattern. No open item here was resolved or newly
raised by this round — see `IMPROVEMENTS.md`'s 2026-09-28 entry for the
detail. Not yet re-scanned: file/line/test counts below are updated, but the
pedantic/nursery and `cargo tree` passes were not re-run.

**Updated a third time, 2026-09-28, after an actual play session** (the
first this project has had against real chunk loading and the new debug
overlay). Found and fixed: redundant same-frame neighbour re-meshing during
large loading bursts (real, though not what caused the log errors it was
first suspected of); a genuine, confirmed-upstream Bevy 0.19 mesh-allocator
logging quirk, now silenced (see "Checked and found nothing to do"); the
chunk grid's per-face crosses read as a detached floating square up close
and were reverted; and the sea-level marker was rebuilt from two
camera-following lines into a real per-column grid. See `IMPROVEMENTS.md`'s
2026-09-28 (2) and (3) entries. No open item here was affected.

**Updated again, 2026-09-29.** A `player/` module arrived in two parts of
the same request: first the player model (`assets/models/player.glb`) with
a third-person camera, then gravity, terrain collision, and a
Survival/Creative game-mode switch. `world/` and the debug chunk grid now
load around the player, not the camera, which resolves 3.3 (removed). Two
new items came with the camera half: 4.7 (camera has no collision) and 4.8
(spawn is a fixed constant, not the real surface). 4.4 was updated with
what's now been seen and what's new to check. Also fixed in passing: 2.1
(README's controls table and module tree, both now stale in new ways —
folded into this round's fix rather than left for later). See
`IMPROVEMENTS.md`, 2026-09-29.

**Updated again, 2026-09-30.** Two fixes reported from actually playing the
2026-09-29 batch: a `DoubleTap` sprint that survived colliding with a wall
(now cancelled the frame after, gated by the new
`config::player::RESET_SPRINT_ON_COLLISION`), and a requested noclip toggle
for Creative (`player::debug`, `F5`). `docs/CONFIG.md` was also added,
documenting `config/`'s structure and conventions rather than its current
values. No open item here was resolved or newly raised. See
`IMPROVEMENTS.md`, 2026-09-30.

**Updated a second time, 2026-09-30.** 2.2, 2.4, 2.5, and 2.6 resolved: the
stale "Startup configuration" section in `ARCHITECTURE.md` deleted and its
two live points folded into "Configuration" and "Window" (plus a doc bug
caught in the same pass — `toggles.rs` doesn't run every frame); the
duplicate `unsafe_code` lint's `Cargo.toml` table deleted, confirmed still
needed by a live IDE diagnostic rather than assumed stale; all six stale doc
comments 2.5 listed fixed; `CLAUDE.md`'s module list updated with `player/`,
`world/`, and `render/`. `docs/CONFIG.md` also gained a full per-file value
reference on request, on top of its existing structure/convention sections.
`cargo fmt --check`, `cargo check`, and `cargo test` (62 pass, unchanged) all
clean. See `IMPROVEMENTS.md`, 2026-09-30 (2).

This file lists improvements that are identified but **not yet done**,
ranked by the four tiers in `CLAUDE.md`. Nothing here has been implemented.
When an item is acted on or rejected, move it to `IMPROVEMENTS.md` (a change
log entry, and a standing-decision row if it sets a rule) and delete it here.
Before proposing anything, check the standing decisions table in
`IMPROVEMENTS.md`: an idea that contradicts a row there needs a conversation
about that row, not a quiet change.

**Scope originally scanned (2026-09-26):** all 35 Rust files (1,899 lines,
largest 190), `Cargo.toml`, `Cargo.lock`, `.gitignore`, every file in
`docs/`, `README.md`, and the parent `CLAUDE.md`. Also run: `cargo clippy
-- -D warnings` (clean), `cargo test` (15 pass), a `clippy::pedantic` +
`clippy::nursery` pass, and `cargo tree -d`. **Now (2026-09-30) 46 files
(4,709 lines, largest 441 — `input/movement.rs`, still well under the
800-line limit but past the 200–400 typical range `CLAUDE.md` names, worth
watching if it keeps growing), 62 tests.** The pedantic/nursery and
`cargo tree` findings below are still only as of the 26th and have not been
re-run since — everything added since then (`render/`, `config/debug.rs`,
`world/debug.rs`, terrain generation, the mesher swap, async generation,
vertical chunk loading, `render::debug`, `player/`, third-person camera,
gravity and collision, game modes, noclip) has not been scanned by those two
passes at all. How to repeat this
scan is at the bottom of the file.

---

## Summary

| ID | Tier | Finding | Effort | Recommendation |
| --- | --- | --- | --- | --- |
| 1.5 | Perf | `opt-level = 0` for our crate | Tiny | Reconfirmed: measure once greedy meshing exists |
| 1.6a | Perf | Meshing (still) runs synchronously on the main thread | Small–Med | Deferred until asked for |
| 3.1 | Mod | `fly` is at the ~50-line limit, with sprint logic inline and untested | Small | Do |
| 4.2 | Other | Test gaps: look clamp, `Hold` sprint, plugin wiring | Small–Med | Do the first two |
| 4.4 | Other | Runtime checks still unconfirmed | — | Check by eye |
| 4.5 | Other | Release builds cannot log anywhere | Medium | Carry over: before a release |
| 4.6 | Other | Temporary scaffolding still to delete | — | Carry over: as replacements land |
| 4.7 | Other | Third-person camera has no collision; clips into terrain | Small–Med | When it gets in the way |
| 4.8 | Other | Player spawns at a fixed position, not on the terrain surface | Small | With player physics |

---

## Tier 1 — Performance

### 1.5 `opt-level = 0` for our crate — reconfirmed

Standing decision, reconfirmed: stays `0`. Meshing exists now (`render/`),
but as a straightforward face-culling pass over one small chunk at a time,
not yet the kind of tight, hot numeric loop the original note meant.
**Greedy meshing is the real trigger** — profile a chunk mesh at `0` and `1`
once that lands, and decide from the numbers rather than guessing.

### 1.6a Meshing still runs synchronously on the main thread

Chunk *generation* moved to a background thread (see `IMPROVEMENTS.md`,
2026-09-27) — meshing didn't. `render::spawn_chunk_meshes` still calls
`mesh::chunk_mesh` directly in the system that reacts to `ChunkLoaded`, and
that same system can re-mesh up to six already-spawned neighbours in the
same frame (the cross-chunk seam fix from the previous entry). At
`RENDER_DISTANCE = 2` a newly-loaded chunk can trigger up to 7 meshing calls
in one frame — still a plain function with no ECS access internally, so the
same `AsyncComputeTaskPool` treatment generation just got would apply
directly, but this wasn't part of what was asked for this round.

**Recommendation:** defer until asked for, the same way generation itself
was deferred before this round. If it's done, it needs a bit more care than
generation's version: spawning a mesh entity and updating an existing one's
`Mesh3d` handle both have to happen back on the main thread (`Commands` and
component mutation aren't `Send` operations you can do from a background
task), so only the `chunk_mesh` call itself moves to the task; the
spawn/update step stays in a polling system, the same shape
`apply_generated_chunks` already uses for generation.

---

## Tier 3 — Modularity

### 3.1 `fly` is at the ~50-line limit, with sprint logic inline

`input::movement::fly` is 46 lines of code. It reads movement intent, runs
the sprint state machine for both `SprintMode`s, and applies motion. Only
`is_double_tap` is unit-tested. The `Hold` branch has never been tested; it
was checked once by temporarily flipping the constant. `CLAUDE.md`: "a system
that grows past ~50 lines is a sign the logic inside it belongs in a plain
function".

**Recommendation:** move the sprint update into a plain function, e.g.
`update_sprint(keys: &ButtonInput<KeyCode>, mode: SprintMode, now: f32, state: &mut SprintState)`,
with `last_tap` and `sprinting` grouped into one `SprintState` struct. Pass
`mode` in as a parameter instead of reading `SPRINT_MODE` inside the
function, so tests can cover **both** modes whatever the constant says.
`fly` then passes `controls::SPRINT_MODE` in.

---

## Tier 4 — Everything else

### 4.2 Test gaps

15 tests exist, all pure logic. Remaining gaps:

- **Look pitch clamp.** The maths in `input::look::look` sits inside the
  system. Pull out `apply_look(angles, delta) -> LookAngles` and test the
  clamp at both limits. (Also suggested in audit #1 as item 4.4.)
- **`Hold` sprint mode.** Covered by 3.1's extraction.
- **Plugin wiring.** `CLAUDE.md` asks for "a thin integration test that
  builds an `App` … and calls `App::update()`". None exist. A first one: an
  `App` with `MinimalPlugins`, `StatesPlugin` and `GameStatePlugin`, updated
  twice, asserting `GameState::Menu` (checks the `Loading → Menu` exit).
  **Medium effort:** the state screens spawn UI and the world camera needs
  rendering types, so what the headless `App` can include has to be worked
  out. Worth doing once there is more wiring to protect.

### 4.4 Runtime checks still unconfirmed

Confirmed by you: the cursor lock and controls on first entering the game,
the pause overlay appearing and clearing, double-tap sprint and Shift to
descend, F11 restoring the window size, and — from the 2026-09-28 play
sessions' screenshots — chunks rendering as solid, right-side-out terrain.
Not yet confirmed by eye: changing `FOV_DEGREES`, `SprintMode::Hold` with
Left Ctrl, the F10 borderless toggle, and — new with `player/` — that the
player model loads and shows its texture, that it faces the way it walks
(its −Z facing was read off the file's UVs, not seen), and how the
third-person distance and pivot height feel in play.

### 4.7 Third-person camera has no collision

`camera::follow` places the camera a fixed distance behind the player with
no check against terrain. Backing the player up against a hill, or looking
up from low ground, puts the camera inside solid blocks. The usual fix is a
ray from the pivot toward the desired camera position, stopping at the first
solid block — which needs a voxel raycast over `LoadedChunks` that doesn't
exist yet (block interaction will need the same thing). **Do it when it
actually gets in the way**, ideally sharing that raycast.

### 4.8 Player spawns at a fixed position

`player::SPAWN_POSITION` is a constant (`8, 15, 16`), chosen only to clear
the ±10-block terrain band. The player floats above whatever the ground is
there. Finding the real surface means sampling the column's height from
`world::generation` (or the loaded chunk) at spawn. It only matters once
there's gravity, so **do it alongside player physics**.

### 4.5 Release builds cannot log anywhere — carried over

Release builds detach the console (`windows_subsystem = "windows"`), and no
file log exists, so a player's crash leaves no trace. Add a file layer via
`LogPlugin::custom_layer` before anything is handed to other people.

### 4.6 Temporary scaffolding still to delete — carried over

- `states/debug.rs` (jump keys 1–3): once real menu transitions exist.
- The placeholder screens in `loading/` and `menu/`: once the real UI exists.

`states/ingame/scene.rs` (six placeholder cubes and a light) is done — see
`IMPROVEMENTS.md`, 2026-09-27. The light moved to `render/mod.rs`; the cubes
are deleted, not replaced.

---

## Checked and found nothing to do

Recorded so the next audit doesn't spend time re-checking these without
reason:

- **`unwrap`/`expect`:** none outside tests.
- **`unsafe`:** none.
- **Per-frame allocations:** none. The only allocations are one-off:
  `format!` in the log setup, `to_string` for the window title, a mesh-handle
  `clone` when spawning the scene, and the chunk `Vec` at generation.
- **File and function size:** the largest file is 190 lines (`movement.rs`,
  more than half of it tests). No function is over the limit except the
  borderline `fly` (3.1).
- **Pedantic and nursery lints (37 warnings):** none worth acting on.
  - Redundant `pub(crate)` inside private modules: style only, and the
    explicit form documents intent.
  - `u32 → f32` and `f32 → i32` casts on `CHUNK_SIZE` and positions: exact
    for these magnitudes, and Rust's float→int `as` saturates, so it cannot
    be undefined.
  - Strict float comparisons against exact zero input (`delta == ZERO`,
    `intent == ZERO`): intentional; zero means "no input".
  - "Passed by value": Bevy `Res`/`Query` system parameters, and the
    `Copy` type `ChunkPos`.
- **Duplicate dependencies (20):** all come from inside Bevy (`hashbrown`
  ×3, `syn` 2 and 3, `windows-sys`, and font crates). None can be fixed from
  here. Note: `syn 3` is already compiled because of `bytemuck_derive`, so
  the `encase 0.12.1` pin does not save a `syn 3` compile. The pin is still
  required for its real reason: the `syn` type mismatch across the
  `bevy_encase_derive` proc-macro boundary.
- **Security:** no network, no file I/O, no parsing of untrusted input. The
  only external input is keyboard and mouse.
- **`bevy_render::slab_allocator` "Use-after-free" errors, logged during
  large chunk-loading bursts:** confirmed (against Bevy's own issue tracker,
  not guessed) to be a Bevy 0.19 logging quirk, not a real memory-safety
  issue or a bug in this project — `MeshAllocator` skips allocating a
  zero-vertex mesh (a fully air chunk, or a fully solid one with every face
  culled by its neighbours, both routine here) but still runs the copy step
  for it regardless, and that copy step is what logs this. Silenced in
  `utils::log`'s `SILENCED` filter rather than worked around in our own
  code, since there is nothing to work around. See `IMPROVEMENTS.md`,
  2026-09-28 (3).

---

## How to run this audit

Repeat these steps each time, then replace this file:

1. Read the standing decisions and reversals in `IMPROVEMENTS.md` first.
2. `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` must
   all be clean. Stop and fix first if not.
3. `cargo clippy --all-targets -- -W clippy::pedantic -W clippy::nursery`.
   Triage new warning *kinds* only; the dismissed kinds are listed above.
4. `cargo tree -d -e normal --depth 0`: note new duplicates, and check
   whether each one comes from us or from Bevy.
5. Grep `src/` for `unwrap(`, `expect(`, `unsafe`, `#[allow`, `TODO`,
   `format!`, `.clone()` and `collect` inside systems.
6. Read every source file's `//!` header and the `pub` docs against what the
   code now does. Stale docs have been the most common finding both times.
7. Check that `README.md`, `ARCHITECTURE.md`'s module tree, and `CLAUDE.md`'s
   "What exists today" match `find src -name '*.rs'`.
8. Check each carried-over item's trigger (for example, "when meshing
   exists") to see whether it has fired.
9. Record decisions in `IMPROVEMENTS.md`, then rewrite this file.
