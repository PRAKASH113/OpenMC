# Audit

**Audit #2 — 2026-09-26.** Replaces audit #1 (2026-09-19) completely; every
item from #1 was either done, recorded in [`IMPROVEMENTS.md`](IMPROVEMENTS.md),
or carried forward below with its current status.

**Updated 2026-09-27, not re-run in full.** Building `render/` (chunk
meshing and the chunk mesh lifecycle) resolved four items outright — 2.3
(`World` renamed to `LoadedChunks`), 3.2 (chunks now unload), 4.1 (spawn
height moved above ground), and 4.3 (`Chunk::block` gained a real caller, so
the dead-code question is moot) — and fixed one doc comment 2.5 flagged
(`RENDER_DISTANCE`'s "3×3 ring" wording). All five are removed below rather
than left marked done; see `IMPROVEMENTS.md`'s 2026-09-27 entry for what
changed and why.

**Updated again, same day.** Went through tier 1 one item at a time: 1.1 and
1.2 done (removed below); 1.3's file re-added, staged, still awaiting a
deliberate enable; 1.4, 1.5, and 1.6 explicitly deferred with reasons
recorded, rather than left as generic carry-overs — see `IMPROVEMENTS.md`.

**Updated a third time, same day.** 1.3 and 1.4 acted on: the Windows linker
enabled, unused Bevy features dropped, `noise` added — made without a build
to confirm any of it, at explicit request.

**Updated a fourth time, same day.** That build happened: dropping `audio`
broke `app::plugin`'s `.disable::<bevy::audio::AudioPlugin>()` line exactly
as its own comment predicted (the type no longer exists once the feature is
gone), surfaced by rust-analyzer and fixed. After that, `cargo check`,
`clippy`, `build`, `test` (27 pass), and a boot run were all run clean —
confirming the UI renders without `2d`, `noise` resolves and compiles, and
`rust-lld.exe` actually links successfully, not merely gets found. Item 1.7
(which existed to flag this exact verification as outstanding) is resolved
and removed. Everything else here is unchanged from audit #2 and has not
been re-checked against the current code.

This file lists improvements that are identified but **not yet done**,
ranked by the four tiers in `CLAUDE.md`. Nothing here has been implemented.
When an item is acted on or rejected, move it to `IMPROVEMENTS.md` (a change
log entry, and a standing-decision row if it sets a rule) and delete it here.
Before proposing anything, check the standing decisions table in
`IMPROVEMENTS.md`: an idea that contradicts a row there needs a conversation
about that row, not a quiet change.

**Scope scanned (2026-09-26):** all 35 Rust files (1,899 lines, largest
190), `Cargo.toml`, `Cargo.lock`, `.gitignore`, every file in `docs/`,
`README.md`, and the parent `CLAUDE.md`. Also run: `cargo clippy -- -D
warnings` (clean), `cargo test` (15 pass), a `clippy::pedantic` +
`clippy::nursery` pass, and `cargo tree -d`. Now 37 files (2,356 lines,
largest 246) after adding `render/`, but the scan itself has not been
repeated — treat file/line counts and the pedantic/nursery and `cargo tree`
findings below as of the 26th, everything else as still current. How to
repeat this scan is at the bottom of the file.

---

## Summary

| ID | Tier | Finding | Effort | Recommendation |
| --- | --- | --- | --- | --- |
| 1.5 | Perf | `opt-level = 0` for our crate | Tiny | Reconfirmed: measure once greedy meshing exists |
| 1.6 | Perf | Chunk generation runs synchronously; `mesh.rs` doesn't see across chunks | Medium | Deferred: alongside greedy meshing + async generation |
| 2.1 | Read | README controls table and module tree are wrong | Tiny | Do |
| 2.2 | Read | ARCHITECTURE "Startup configuration" describes deleted modules | Small | Do |
| 2.4 | Read | `unsafe_code` lint declared twice; comment contradicts `Cargo.toml` | Tiny | Decide which one to keep |
| 2.5 | Read | Stale or inaccurate doc comments in 7 places | Small | Do |
| 2.6 | Read | `CLAUDE.md` "What exists today" is missing `world/` and `render/` | Tiny | Do |
| 3.1 | Mod | `fly` is at the ~50-line limit, with sprint logic inline and untested | Small | Do |
| 3.3 | Mod | `world/` finds the player through the camera | — | Note only: revisit with `player/` |
| 4.2 | Other | Test gaps: look clamp, `Hold` sprint, plugin wiring | Small–Med | Do the first two |
| 4.4 | Other | Runtime checks still unconfirmed | — | Check by eye |
| 4.5 | Other | Release builds cannot log anywhere | Medium | Carry over: before a release |
| 4.6 | Other | Temporary scaffolding still to delete | — | Carry over: as replacements land |

---

## Tier 1 — Performance

### 1.5 `opt-level = 0` for our crate — reconfirmed

Standing decision, reconfirmed: stays `0`. Meshing exists now (`render/`),
but as a straightforward face-culling pass over one small chunk at a time,
not yet the kind of tight, hot numeric loop the original note meant.
**Greedy meshing is the real trigger** — profile a chunk mesh at `0` and `1`
once that lands, and decide from the numbers rather than guessing.

### 1.6 Chunk generation runs synchronously on the main thread — deferred

`generate` is called inside a system, so a chunk is built within the frame
that needs it. At `RENDER_DISTANCE = 2` that's up to 13 chunks, and moving
across a chunk boundary can generate several in one frame — a hitch, though
not yet a severe one at this size.

**Deliberately postponed**, alongside greedy meshing: multi-threaded
chunk generation (`AsyncComputeTaskPool`) is planned as a follow-up to that
work, not before it — `generate` is already a pure function, so moving it
off the main thread needs no change to the function itself when the time
comes.

**Same trigger, and now an active cost, not just a future one:**
`render::mesh` never looks past its own chunk's data — a block at a chunk
edge always gets its boundary face, even where a neighbouring chunk would
actually hide it. This was harmless when only one chunk was ever loaded; at
`RENDER_DISTANCE = 2`, with up to 13 chunks loaded and touching, it is now
real extra geometry drawn at every chunk seam. Still deferred alongside the
rest of this item — greedy meshing is expected to fold in cross-chunk
awareness as part of the same rework, so fixing it twice would be wasted
effort. Fixing it needs `mesh.rs` to read the *other* chunk's edge blocks
from `LoadedChunks`, which does not exist as a capability yet.

---

## Tier 2 — Readability and discoverability

### 2.1 README controls table and module tree are wrong

`README.md` is the first thing a stranger reads, and it currently says:

- `Left Ctrl` flies down. It is `Left Shift` now.
- `Left Shift` sprints. Sprint is now a double-tap of W/A/S/D, or holding
  `Left Ctrl` when `SPRINT_MODE` is `Hold`.
- The module tree has no `world/`, and does not list `config/camera.rs` or
  `config/world.rs`.
- "Terrain, chunks … are next". Chunk data now exists; rendering is next.

**Recommendation:** fix all four. Point the controls table at
`config/input.rs` as the source of truth, so it is easier to keep in sync.

### 2.2 ARCHITECTURE "Startup configuration" describes deleted modules

The section talks about `app::config`, `app::window::primary_window_plugin`
and `app::window::WindowControlPlugin`. None of these has existed since the
tier-2 and tier-3 restructures (they are `config::window`,
`window::setup::primary_window_plugin` and
`window::toggles::WindowControlPlugin`). Most of its content also repeats
the later "Configuration" and "Window" sections.

**Recommendation:** delete the section. Move its two unique points into
"Configuration" and "Window": why Bevy enums are used directly
(`PresentMode`'s fallback chain), and "constants describe startup; the live
`Window` component is what changes at runtime".

### 2.4 The `unsafe_code` lint is declared twice, and the comment is wrong

`Cargo.toml` still has an active `[lints.rust] unsafe_code = "deny"` table.
The comment directly under it says the lint is set in `main.rs` "rather
than" in that table. `main.rs` also has `#![deny(unsafe_code)]`. Git shows
the table was never removed when the lint moved. So the editor warning that
the move was meant to silence is presumably still there, and a reader cannot
tell which declaration is the real one.

**Recommendation — pick one:**

- **(a) Delete the table.** This finishes the original decision. The IDE
  warning goes away, and the comment becomes true.
- **(b) Keep the table, delete the `main.rs` attribute and fix the comment.**
  Choose this if the editor warning no longer appears (for example, after an
  extension update). `[lints]` is the form that also covers future
  `tests/`, benches, and workspace members.

Either way, update the matching notes in `main.rs`, `Cargo.toml` and
`CLAUDE.md` together.

### 2.5 Stale or inaccurate doc comments

| Where | Says | Should say |
| --- | --- | --- |
| `app/mod.rs` header | the window is built by "finished adapters … in `crate::utils`" | the window is `crate::window`; `utils` only holds `log` |
| `window/mod.rs` header | "`toggles` runs every frame" | runs only on the frame F10/F11 is pressed (run conditions) |
| `window/mod.rs` `FULLSCREEN_MODE` | `config::FULLSCREEN` | `config::window::FULLSCREEN` |
| `states/mod.rs` `GameState::Loading`, `states/loading/mod.rs` header | the world-generation design is in `docs/ARCHITECTURE.md` | it is in `docs/LOADING.md` |
| `config/window.rs` `PRESENT_MODE` | "…actually supports. so they work…", with lines over 100 characters | fix the sentence, re-wrap |
| `input/movement.rs` `MOVEMENT_KEYS` | "The keys a double-tap of any one of can engage sprint" | "Double-tapping any of these keys engages sprint" |
| `Cargo.toml` release profile | "realeases" | "releases" |

### 2.6 `CLAUDE.md` "What exists today" is missing `world/` and `render/`

The parent `CLAUDE.md` (outside this repo) lists every current module except
`world/` and `render/`. Its architecture section also still shows both only
as future targets. It is the first thing a new contributor reads.

**Recommendation:** add `world/` (chunk coordinates, storage, generation) and
`render/` (chunk meshing, materials, the chunk mesh lifecycle) to that list.

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

### 3.3 `world/` finds the player through the camera — note only

`load_chunks_around_player` queries `With<WorldCamera>`. That is correct
today, because the camera *is* the player, and the dependency direction
(`world/` → `camera/`) is legitimate. When `player/` arrives, switch to a
player marker so `world/` doesn't depend on how the view is rendered.
**No action now.**

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
descend, and F11 restoring the window size. Not yet confirmed by eye:
changing `FOV_DEGREES`, `SprintMode::Hold` with Left Ctrl, the F10
borderless toggle, and — new since `render/` — whether a chunk actually
appears on screen as a solid green surface rather than nothing, whether the
player now spawns above it instead of inside it, and whether every face
renders right-side-out rather than inside-out (the winding in `render/mesh`
was checked by hand with the cross-product for all six faces and by the
vertex-count tests, but never seen rendered).

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
