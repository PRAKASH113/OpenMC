# Audit

**Audit #3 — 2026-09-30.** Replaces audit #2 (2026-09-26, updated through
2026-09-30) completely. Every item from #2 was either done, recorded in
[`IMPROVEMENTS.md`](IMPROVEMENTS.md), or carried forward below under a new
ID. The mapping is in "Audit #2's items" at the end of this file. The full
history of what #2 found and what was done about it lives in
`IMPROVEMENTS.md`'s change log, 2026-09-26 through 2026-09-30 (3).

This file lists improvements identified by the audit, whether or not they've
been acted on yet — see each item's status. During this audit's life, a
resolved item is marked `[DONE]` or `[REJECTED]` in place with a short note,
not deleted, and gets a change-log entry (plus a standing-decision row if it
sets a rule) in `IMPROVEMENTS.md`. The next full audit wipes this file and
starts again from the checklist at the bottom. Before proposing anything,
check the standing decisions table in `IMPROVEMENTS.md`: an idea that
contradicts a row there needs a conversation about that row, not a quiet
change. Three items below (1.1, 1.3, 3.2) do touch standing rows, and say so.

**Scope scanned:** all 46 Rust files (4,977 lines; largest
`input/movement.rs` at 528, more than half of it tests), `Cargo.toml`,
`Cargo.lock`, `.gitignore`, every file in `docs/`, `README.md`, and the
parent `CLAUDE.md`. Also run: `cargo fmt --check`, `cargo clippy -- -D
warnings`, `cargo test` (71 pass), a `clippy::pedantic` + `clippy::nursery`
pass, `cargo tree -d`, and a grep sweep for `unwrap`/`expect`/`unsafe`/
`#[allow`/`TODO`/`format!`/`.clone()`/`collect`.

**The baseline wasn't clean on arrival.** `cargo check` failed on the four
uncommitted files adding the menu's Play button and the pause screen's Exit
button. Their marker components (`PlayButton`, `ExitButton`) were private
but named in the signature of a `pub(crate)` system the parent module
registers, which is a hard error. Their query types also tripped clippy's
`type_complexity`. Fixed before scanning, with no change in behaviour: the
markers are now `pub(crate)`, matching the screen markers beside them, and
the query filters are named type aliases (`PlayButtonChanged`,
`ExitButtonChanged`). The rest of that work is still yours and uncommitted.
This audit treats it as part of the codebase, which is why 2.1 exists.

---

## Summary

| ID | Tier | Finding | Effort | Recommendation |
| --- | --- | --- | --- | --- |
| 1.1 | Perf | Chunk loading rescans the whole render area on every frame the player moves or turns | Small | Do: skip unless the player's chunk changed |
| 1.2 | Perf | `opt-level = 0` never measured, though its trigger (greedy meshing) fired | Small | Do: measure `chunk_mesh` at 0 and 1 |
| 1.3 | Perf | Meshing still runs synchronously, now in far bigger bursts at `RENDER_DISTANCE = 6` | Small–Med | Decide after 1.2's numbers |
| 1.4 | Perf | The mesher allocates buffers and a `Mesher` fresh for every chunk | Tiny–Small | Do the `with_capacity` part; reuse after 1.2 |
| 2.1 | Read | Docs still describe the game before the Play/Exit buttons | Tiny | Do, when the buttons are committed |
| 2.2 | Read | Stale or wrong doc comments in 9 places | Small | Do |
| 2.3 | Read | Three docs still say `AudioPlugin` is disabled at runtime | Tiny | Do |
| 2.4 | Read | `Cargo.toml`'s "Step 3" says the game loads no models | Tiny | Do |
| 2.5 | Read | `CLAUDE.md`'s layout rule still says `input/ -> camera/`; the real graph is `input/ -> player/ <- camera/` | Tiny | Do |
| 3.1 | Mod | `apply_physics` is ~65 lines, with gravity/jump/landing rules inline and untested | Small | Do |
| 3.2 | Mod | The Play and Exit buttons are duplicated verbatim | — | Note only: revisit at the third button |
| 4.1 | Other | No plugin-wiring integration test, with far more wiring to protect now | Small–Med | Do a first one |
| 4.2 | Other | Release builds cannot log anywhere | Medium | Carry over: before a release |
| 4.3 | Other | Debug jump keys and placeholder screens — trigger partly fired | — | Decide whether to delete `states/debug.rs` now |
| 4.4 | Other | Third-person camera has no collision | Small–Med | Deferred: until terrain is more detailed |
| 4.5 | Other | Player spawns at a fixed position, not on the surface | Small | Deferred: until terrain is more interesting |
| 4.6 | Other | No floor below the world: Creative can fall forever | Small | Note only: until it matters |
| 4.7 | Other | The sea-level debug grid allocates a `HashSet` every frame it's shown | Tiny | Do, whenever `render::debug` is next touched |

---

## Tier 1 — Performance

### 1.1 Chunk loading rescans the whole render area on every frame the player moves or turns

`world::load_chunks_around_player` is gated on `Changed<Transform>` for the
player. Its standing-decision row accepts that as "coarser than tracking the
specific derived value, … reached for before a hand-rolled check". That
trade-off was made at `RENDER_DISTANCE = 2`. It's now `6`, and the system
runs on *every* frame the player moves, and on every frame they turn, since
`input::look` rotates the same `Transform`. Each run does three things:

- `retain` over every loaded chunk (≈ 226 at radius 6 with 2 vertical layers,
  up from ≈ 26).
- `retain` over every pending task.
- A `(2r+1)² × layers` candidate scan: 13 × 13 × 2 = 338 positions, each
  checked with `in_render_distance` plus up to two `HashMap` lookups. That's
  up from 50.

That's roughly 560 range checks and 450 hash lookups per frame at
`opt-level = 0`, spent recomputing an answer that only changes when the
player crosses a chunk boundary, about once every 32 blocks.

**Recommendation:** early-exit when the player's `ChunkPos` equals the
centre of the last run. Keep that centre in a small resource rather than a
`Local`, so `clear_loaded_chunks` can reset it on leaving the world.
Otherwise re-entering in the same chunk would skip the load entirely. Keep
`Changed<Transform>` too: it still skips the system outright when the player
is idle. The chunk check is what skips it while moving within one chunk.
**This amends the standing row on `Changed<Transform>`** (Performance), so
record the change there when it lands. A single test covers the one new
decision ("same centre → no work, reset → work again") if it's pulled into a
pure function.

### 1.2 `opt-level = 0` never measured, though its trigger fired

Carried from audit #2's 1.5, which said: "Greedy meshing is the real trigger
— profile a chunk mesh at `0` and `1` once that lands." Greedy meshing
landed on 2026-09-27 and the measurement was never taken. It matters more
now. `mesh::padded_voxels` copies 32,768 blocks one at a time through
`Chunk::block` (bounds-checked, `assert!`) and `pad_linearize`, and that's
*our* crate's code, so it runs unoptimised along with the glam maths it
inlines. `binary_greedy_meshing` itself is a dependency and runs at
`opt-level = 3`.

**Recommendation:** time `chunk_mesh` on a typical overlapping-the-band
chunk at `opt-level = 0` and `1`: a `std::time::Instant` around one call in
a throwaway test, or `bevy/trace_tracy` over a world load. The standing
decision already names meshing as the trigger to raise it, so if the numbers
justify `1`, that's the row's own condition being met rather than a reversal.
Measure this first: 1.3 and 1.4 both depend on it.

### 1.3 Meshing still runs synchronously, now in far bigger bursts

Carried from audit #2's 1.6a. `render::spawn_chunk_meshes` meshes every
chunk whose `ChunkLoaded` arrives that frame, plus its already-spawned
neighbours, all on the main thread. At `RENDER_DISTANCE = 6` that means:

- **Entering a world:** about 226 generation tasks, most of which finish in
  a handful of frames, so over a hundred meshes can land in one frame.
- **Crossing a chunk boundary:** about 13 new columns × 2 layers, plus
  re-meshes of their already-loaded neighbours, so dozens of meshes in one
  frame.

At radius 2 this was a few meshes. Whether it's a visible hitch now depends
on 1.2's per-chunk number, which is why this waits for it.

**Recommendation:** once 1.2 gives a per-chunk cost, pick one:

- **(a) Raise `opt-level`**, if that alone makes the burst cheap enough.
- **(b) Move `chunk_mesh` onto `AsyncComputeTaskPool`**, the same way
  generation did. Spawning or updating the entity stays in a polling system
  on the main thread, since `Commands` and component mutation can't happen
  in a task.
- **(c) Cap meshes per frame** and queue the rest. The smallest change,
  though it trades hitches for pop-in.

(b) touches the standing row "Loading a chunk also re-meshes any of its
already-spawned face neighbours" only in *where* the re-mesh runs, not
whether it does.

### 1.4 The mesher allocates buffers and a `Mesher` fresh for every chunk

`mesh::chunk_mesh` does four allocations of its own:

- It builds `positions`, `normals` and `uvs` with `Vec::new()` and grows
  them one push at a time. The final size is known up front: four vertices
  per quad, summed over `mesher.quads`. `CLAUDE.md` asks for
  `Vec::with_capacity` whenever a size is known.
- It allocates a new `bgm::Mesher`, which owns large internal buffers.
- It allocates a new all-zero transparency mask.
- It allocates a new padded voxel buffer.

All of this repeats for every chunk and every neighbour re-mesh.

**Recommendation:** the `with_capacity` change is three lines and free, so
do it. Reusing the `Mesher` and buffers (a `Local` or resource, cleared per
chunk) is a bigger change that only pays off if 1.2 shows meshing is worth
optimising. It also interacts with 1.3(b), since reused buffers can't cross
a task boundary as-is. Decide it alongside 1.3.

---

## Tier 2 — Readability and discoverability

### 2.1 Docs still describe the game before the Play/Exit buttons

Once the uncommitted menu and pause buttons land, these become wrong:

| Where | Says | Should say |
| --- | --- | --- |
| `ARCHITECTURE.md`, States | "Only three places in the crate change state … Nothing transitions `Menu -> InGame` except the debug key yet" | five places — add `menu::screen::handle_play_button` (`Menu -> InGame`) and `ingame::paused::screen::handle_exit_button` (`Paused -> Menu`) |
| `ARCHITECTURE.md`, Screens | `menu/` renders "a full-screen opaque colour with the state's name"; `paused/` only an overlay | the menu has a Play button; the pause overlay has an Exit button |
| `ARCHITECTURE.md`, Player (`game_mode.rs` paragraph) | the jump keys are left alone "since `3` is still the only way into a world at all" | `3` is a shortcut now; the Play button is the real way in |
| `config/input.rs`, debug section comment | the state jumps "stand in for a menu that doesn't exist yet, and `3` is the only way into a world at all" | same correction |
| `states/debug.rs`, module doc | "Delete this module once real transitions exist" | see 4.3 — they now partly do |

**Recommendation:** fix all five in the same commit as the buttons.

### 2.2 Stale or wrong doc comments

| Where | Says | Should say |
| --- | --- | --- |
| `render/mod.rs`, `spawn_chunk_meshes` | same-frame mesh churn "is what was observed tripping Bevy's own mesh slab allocator" | disproven on 2026-09-28 (3): the real cause is zero-vertex meshes, silenced in `utils::log`. The dedup is still worth keeping for the work it saves, but not for that reason |
| `render/mesh.rs`, module doc | "(see `docs/AUDIT.md`, 1.6)" | that item no longer exists; drop the pointer or point at `IMPROVEMENTS.md`, 2026-09-27 |
| `render/material.rs`, `ChunkMaterial` | "the same reasoning `scene.rs`'s placeholder cubes used" | `scene.rs` was deleted on 2026-09-27; state the reasoning directly |
| `world/block.rs`, module doc | block variety "waits for `render/`" | `render/` exists; variety now waits on a texture atlas |
| `world/mod.rs`, `LoadedChunks::insert` | "not just used internally by `load_chunks_around_player`" | its production caller is `apply_generated_chunks` |
| `camera/camera_ui.rs`, `UiCamera` | "this is what tells them apart in a query" | no query reads it yet; say it's there for when one does (a HUD) |
| `config/debug.rs`, `TESTING_TOOLS_ENABLED` | "chunk locking (`world::debug`), and whatever joins it later" | five features now, across `world::debug`, `render::debug`, and `player::debug` |
| `input/mod.rs`, module doc, line 5 | a line over 100 characters | re-wrap to 80; rustfmt doesn't wrap comments |
| `player/mod.rs`, `SPAWN_POSITION` | "see `docs/AUDIT.md` 4.8" | 4.5 in this audit |

**Recommendation:** fix all nine in one pass. None of them change code.

### 2.3 Three docs still say `AudioPlugin` is disabled at runtime

The `audio` Cargo feature was dropped on 2026-09-27, which removed
`AudioPlugin` from existence. `app/plugin.rs`'s own comment already says so.
Three places didn't catch up:

- `ARCHITECTURE.md`, Composition ("the unused ones (`AudioPlugin`,
  `GilrsPlugin`) disabled").
- `CLAUDE.md`, Performance ("Currently disabled: `AudioPlugin`,
  `GilrsPlugin`").
- `IMPROVEMENTS.md`'s standing row on it. That row is corrected in this
  audit's own pass, so only the first two remain.

**Recommendation:** say "`GilrsPlugin` disabled at runtime; audio dropped at
the Cargo feature level" in both.

### 2.4 `Cargo.toml`'s "Step 3" says the game loads no models

The "Step 3 (bigger, riskier): 3d without glTF" comment reasons that "This
game loads no models — its meshes are generated in code — so that is a
large unused dependency." Since 2026-09-29 the player is
`assets/models/player.glb`, loaded through `bevy_gltf`. Dropping glTF is no
longer an open option at all.

**Recommendation:** replace the Step 3 block with a two-line note that glTF
is required for the player model, so a future trim doesn't try it. The
standing row on Bevy features is updated in this audit's pass.

### 2.5 `CLAUDE.md`'s layout rule still says `input/ -> camera/`

`CLAUDE.md`'s layout rules say: "`camera/` owns camera entities; `input/`
owns what input does to them. Dependencies run `input/ -> camera/`, never
the reverse." Since 2026-09-29 `input/` doesn't import `camera/` at all. It
writes the player's `LookAngles` and `MovementIntent`, and `camera::follow`
reads the player to place itself, so the real graph is
`input/ -> player/ <- camera/`. The rule's intent still holds: input never
owns entities, and nothing reads input but `input/`. Only its wording is
out of date. `render/mod.rs`'s module doc makes the same stale comparison
("the same direction `input/` depends on `camera/`"). Two rows in
`IMPROVEMENTS.md` made it too, and are corrected in this audit's pass.

**Recommendation:** reword the rule to "`input/` owns what input does to the
player; `camera/` owns the cameras and follows the player. Dependencies run
`input/ -> player/ <- camera/`; `player/` knows neither." Fix `render/mod.rs`'s
comparison in the same pass.

---

## Tier 3 — Modularity

### 3.1 `apply_physics` is ~65 lines, with gravity/jump/landing rules inline and untested

`player::physics::apply_physics` runs well past `CLAUDE.md`'s ~50-line
signal, and the part over the line is exactly the part with rules in it:

- Flying zeroes vertical velocity.
- A jump only counts from the ground.
- Gravity is capped at `TERMINAL_VELOCITY`.
- Landing sets `grounded`, and a ceiling hit doesn't.
- "Stuck" clears both.

The collision maths (`move_and_collide` and friends) is well tested; none of
those rules are, since they only exist inside the system. It's the same
shape audit #2's 3.1 found in `fly`.

**Recommendation:** pull two pure functions out, keeping the system as the
thin ECS shell:

- **`vertical_step(motion, intent, flying, dt) -> Option<Vec3>`** returns the
  frame's delta, or `None` for the idle early-exit.
- **`settle(motion, moved, delta)`** applies the collision result to
  `Motion`.

Test each rule above directly. No behaviour change.

### 3.2 The Play and Exit buttons are duplicated verbatim — note only

`menu/screen.rs` and `ingame/paused/screen.rs` repeat the same pieces:

- The same button `Node`.
- The same three-colour idle/hover/pressed scheme, different only in hue.
- The same `Changed<Interaction>` system shape, different only in the state
  it sets.

The standing decision "each state's screen is self-contained; duplication is
intentional" covers this, and two copies is not a pattern yet.

**No action now.** When a third button arrives (Resume, Settings, Quit), a
shared button builder is the natural extraction. `CLAUDE.md`'s target layout
already names a `ui/` module for menus. That's a conversation about the
standing row then, not a quiet change.

---

## Tier 4 — Everything else

### 4.1 No plugin-wiring integration test

Carried from audit #2's 4.2, whose first two gaps (look clamp, `Hold` sprint)
are done. `CLAUDE.md` asks for "a thin integration test that builds an
`App` … and calls `App::update()`", and none exists. Audit #2 deferred it
until there was more wiring to protect. There's a lot more now:

- The input chain ordered `.before(PlayerPhysics)`.
- The camera follow in `PostUpdate` before transform propagation.
- `in_creative`-gated hotkeys.
- Two state transitions driven by UI buttons.

The baseline breakage this audit started with was a compile error, which
tests wouldn't have caught. A missing `.before()` or a wrong run condition,
though, is exactly what only a wiring test catches.

**Recommendation:** start with the one audit #2 described: an `App` with
`MinimalPlugins`, `StatesPlugin` and `GameStatePlugin`, updated twice,
asserting `GameState::Menu`. Extend from there. The state screens spawn UI,
so what a headless `App` can include still has to be worked out.

### 4.2 Release builds cannot log anywhere — carried over

Carried from audit #2's 4.5, unchanged. Release builds detach the console
(`windows_subsystem = "windows"`), and there's no file log, so a player's
crash leaves no trace. Add a file layer via `LogPlugin::custom_layer` before
anything is handed to other people.

### 4.3 Debug jump keys and placeholder screens — trigger partly fired

Carried from audit #2's 4.6. Its trigger for deleting `states/debug.rs` was
"once real menu transitions exist". Two now do: Play (`Menu -> InGame`) and
Exit (`Paused -> Menu`). Keys `2` and `3` are now shortcuts for things the
UI can do, and key `1` (`Loading`) never had a real path at all.
`loading/`'s screen is still a placeholder; `menu/` is part-way to real.

**Recommendation:** your call whether the shortcuts are still worth keeping
in debug builds, where they're faster than clicking. Either way, 2.1's doc
fix for `states/debug.rs` applies.

### 4.4 Third-person camera has no collision — deferred

Carried from audit #2's 4.7, unchanged. `camera::follow` places the camera
a fixed distance behind the player with no terrain check, so backing into a
hill puts it inside blocks. The fix is a voxel raycast from the pivot, which
block interaction will need too. Deferred on 2026-09-30 until terrain
generation does more than a smooth heightmap. The terrain is unchanged, so
it stays deferred.

### 4.5 Player spawns at a fixed position — deferred

Carried from audit #2's 4.8, unchanged. `player::SPAWN_POSITION` is
`(8, 15, 16)`, chosen only to clear the ±10-block terrain band. With gravity
the player now falls to the ground instead of floating, which takes most of
the sting out of it. It stays deferred alongside 4.4.

### 4.6 No floor below the world — note only

`LoadedChunks::is_solid` treats everything outside the world's vertical
extent as air, by design (`IMPROVEMENTS.md`, Player), so the space above the
world stays flyable. The other side of that is below the bottom (currently
y < −32). In Creative, a player who flies or noclips down there and then
lands falls forever. Survival can't get there, since the bottom 22 blocks
are guaranteed solid.

**No action now.** When it matters (raising `CHUNKS_BELOW_SEA_LEVEL`,
adding caves, or reaching Survival), a "below the world → respawn" check is
a few lines in `player::physics`.

### 4.7 The sea-level debug grid allocates a `HashSet` every frame it's shown

`render::debug::draw_sea_level_marker` builds a fresh `HashSet` of loaded
columns every frame while `F6` is on: about 113 entries at radius 6. It's
debug-only and off by default, but it's still the one per-frame allocation
in the codebase, which `CLAUDE.md` rules out.

**Recommendation:** a `Local<HashSet<(i32, i32)>>`, cleared at the top of
each run. Do it the next time `render::debug` is touched anyway.

---

## Audit #2's items

What happened to each item audit #2 still had listed, so a later audit can
trace any of them:

| Audit #2 | Status | Now |
| --- | --- | --- |
| 1.5 `opt-level = 0` | Trigger fired, not measured | 1.2 |
| 1.6a Synchronous meshing | Open, more pressing | 1.3 |
| 3.1 `fly` at the line limit | `[DONE]` 2026-09-30 (3) — `update_sprint` | — |
| 4.2 Test gaps | Look clamp and `Hold` sprint `[DONE]`; plugin wiring open | 4.1 |
| 4.4 Runtime checks | `[DONE]` 2026-09-30 (3), confirmed by play | — |
| 4.5 Release logging | Open | 4.2 |
| 4.6 Temporary scaffolding | Trigger partly fired | 4.3 |
| 4.7 Camera collision | Deferred | 4.4 |
| 4.8 Fixed spawn | Deferred | 4.5 |

Everything else audit #2 raised (2.1–2.6, 3.2, 3.3, 4.1, 4.3, and all of
tier 1 but 1.5/1.6a) was resolved before this audit; see `IMPROVEMENTS.md`.

---

## Checked and found nothing to do

Recorded so the next audit doesn't spend time re-checking these without
reason:

- **`unwrap`/`expect`:** none outside tests.
- **`unsafe`:** none. `#![deny(unsafe_code)]` in `main.rs` is the single
  declaration since 2026-09-30 (2).
- **`#[allow]`:** one: `#[allow(dead_code)]` on `config::input::SprintMode`,
  a standing decision (one variant is always unconstructed).
- **Per-frame allocations in gameplay paths:** none. The `collect`s in
  `apply_generated_chunks` and `spawn_chunk_meshes` produce empty
  collections on idle frames, which don't allocate. The only exception is
  the debug-only 4.7.
- **File and function size:** largest file `input/movement.rs` at 528 lines,
  about 250 of them code and the rest tests. No function over the line
  except `apply_physics` (3.1).
- **Pedantic and nursery lints (134 warnings, 20 kinds):** none worth acting
  on beyond what's above.
  - *Dismissed in audit #2, still dismissed:* redundant `pub(crate)` inside
    private modules; `u32 → f32`/`f32 → i32` casts on `CHUNK_SIZE` and
    positions (exact at these magnitudes); strict float comparisons against
    exact zero input; "passed by value" on Bevy `Res`/`Query` params and the
    `Copy` type `ChunkPos`.
  - *New kinds, triaged this audit, all style or provably safe:*
    - The remaining cast lints (`cast_lossless`, `cast_possible_wrap`,
      `cast_possible_truncation`, `cast_sign_loss`). Every one is on a
      small, non-negative, or already-clamped value: chunk counts, clamped
      heights, `rem_euclid` results.
    - `missing_const_for_fn`, `use_self`, `suboptimal_flops` (`mul_add`),
      `single_match_else`, `equatable_if_let`, `option_if_let_else`,
      `map_unwrap_or`, `redundant_closure`.
    - `trivially_copy_pass_by_ref`, on `&LookAngles` in
      `camera::follow::third_person_transform`. It's an 8-byte `Copy` type,
      so passing by value would be marginally tidier, but the effect is nil.
- **Duplicate dependencies (11 crates duplicated):** all inside Bevy's own
  tree except one. That one is `noise 0.9.0`'s `rand 0.8` / `rand_core 0.6` /
  `rand_xorshift`, alongside Bevy's `rand 0.10`. `0.9.0` is the latest
  `noise`, and the three are small crates compiled once and cached, so it's
  accepted rather than worked around (standing row added). The other new
  duplicate since audit #2, `miniz_oxide 0.8/0.9`, is two versions inside
  Bevy's `png` stack. The `encase 0.12.1` pin is still required for the
  `syn` type mismatch, not build time.
- **Security:** no network, no file I/O beyond Bevy loading the bundled
  `assets/` folder, no parsing of untrusted input. The only external input
  is keyboard and mouse.
- **`bevy_render::slab_allocator` "Use-after-free" errors:** a confirmed
  Bevy 0.19 logging quirk for zero-vertex meshes, not a memory-safety issue.
  Silenced in `utils::log`. See `IMPROVEMENTS.md`, 2026-09-28 (3). The one
  place that still blames it on mesh churn is 2.2.
- **Docs cross-check:** `ARCHITECTURE.md`'s module tree, `README.md`'s tree,
  and `CLAUDE.md`'s "What exists today" all match the 46 files on disk.
  `docs/CONFIG.md`'s value reference matches every current `config/` value,
  including `RENDER_DISTANCE = 6`, `FOV_DEGREES = 60`, and
  `INITIAL_GAME_MODE = Creative`.

---

## How to run this audit

Repeat these steps each time, then **wipe this file and rewrite it**:

1. Read the standing decisions and reversals in `IMPROVEMENTS.md` first.
2. `git status`: note any uncommitted work before judging the baseline, so
   an in-progress edit isn't mistaken for committed code. This audit found
   exactly that.
3. `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` must
   all be clean. Stop and fix first if not, touching only what's needed to
   compile, and say what was changed.
4. `cargo clippy --all-targets -- -W clippy::pedantic -W clippy::nursery`.
   Triage new warning *kinds* only; the dismissed kinds are listed above.
5. `cargo tree -d -e normal --depth 0`: note new duplicates, and check
   whether each one comes from us (`cargo tree -i <crate>@<version>`) or
   from Bevy.
6. Grep `src/` for `unwrap(`, `expect(`, `unsafe`, `#[allow`, `TODO`,
   `format!`, `.clone()` and `collect` inside systems.
7. Read every source file's `//!` header and the `pub` docs against what the
   code now does. Stale docs have been the most common finding every time.
8. Check that `README.md`, `ARCHITECTURE.md`'s module tree, `CLAUDE.md`'s
   "What exists today", and `docs/CONFIG.md`'s value reference match the
   code.
9. Check each carried-over item's trigger to see whether it has fired. Also
   **re-weigh any item or standing decision whose cost scales with a config
   value that has changed since** (this audit: `RENDER_DISTANCE` 2 → 6,
   behind 1.1 and 1.3).
10. Record decisions in `IMPROVEMENTS.md`: refresh any standing row this
    audit found stale, add rows for decisions made since the last audit,
    and write a change-log entry for the audit itself. Then rewrite this
    file. During its life, mark resolved items `[DONE]` or `[REJECTED]` in
    place, with a short note, rather than deleting them.
