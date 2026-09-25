# Audit

A review of everything built so far, against the four priority tiers in
`CLAUDE.md`. **Nothing here is implemented** — this is a list of candidates
for you to accept, reject, or defer.

[`IMPROVEMENTS.md`](IMPROVEMENTS.md) records decisions already *made*. This
file records ones still *open*. When something here is acted on, it should
move there.

Current size: 1,079 lines of Rust across 20 files. Largest file is
`camera/camera_3d.rs` at 189 lines. Nothing is close to the 800-line limit,
so none of these are urgent.

---

## Tier 1 — Performance

### 1.1 Unused Bevy features are being compiled — **staged, commented out**

Written into `Cargo.toml` as three commented steps, deliberately not enabled:
turning any of them on recompiles all of Bevy, so it should ride along with
the next dependency change instead of costing a rebuild on its own.

The `Camera2d` question below is now **resolved**: `ui_bevy_render` already
pulls in `bevy_core_pipeline`, which contains the Core2d render pass, and
`2d_api` is only `bevy_sprite`. This game never spawns a `Sprite`, so the `2d`
feature looks droppable — read off the feature graph, so still worth one
compile to confirm the UI draws.

Also found: the `3d` umbrella hardcodes `bevy_gltf` and `gltf_animation`, and
this game loads no models — its meshes are generated in code. Dropping that
means hand-listing the `3d` sub-features, which is recorded as a later step.

The original finding follows.

### 1.1a Original finding

**Build time. The biggest single win available right now.**

`Cargo.toml` uses `bevy = "0.19"` with default features, which are
`["2d", "3d", "ui", "audio"]`. Those expand further — `2d`, `3d` and `ui` all
pull in `picking`, and `audio` pulls in `bevy_audio` plus `vorbis`, an Ogg
decoder.

The game currently uses **no audio and no picking at all**. Every one of
those crates is compiled, linked, and carried in the binary for nothing.

Turning them off means `default-features = false` and re-adding what is
needed, which is fiddly to get right the first time.

**One caveat I could not resolve without testing:** the UI camera is
`Camera2d`, which may come from the `2d` feature rather than `ui`. If so,
`2d` cannot simply be dropped. Worth testing rather than assuming — the
`audio` removal is safe regardless.

### 1.2 No linker configuration — **staged, commented out**

`.cargo/config.toml` now exists with `rust-lld` blocks for Windows, Linux and
macOS, all commented. Same reason: enabling it forces a full rebuild. It also
carries the `cargo build --timings` recipe for measuring whether it actually
helped, so the change can be justified rather than assumed.

The original finding follows.

### 1.2a Original finding

**Build time.** Bevy links a very large binary, and on Windows the default
MSVC linker is the slowest part of every incremental rebuild. Switching to
`rust-lld` via `.cargo/config.toml` is a well-documented, low-risk change
that typically cuts several seconds off *every* edit-compile cycle. There is
currently no `.cargo/config.toml` at all.

Bevy's `dynamic_linking` feature is the other standard dev speedup — it
links Bevy as a shared library so it is not re-linked each time. It is
dev-only and must not ship in release.

### 1.3 `opt-level = 0` — **kept deliberately; code written to suit it**

Decision: stays at `0` for now, raised later. Until then, per-frame code is
written so unoptimised builds pay as little as possible — see the conventions
added to `CLAUDE.md` and item 1.4.

The key fact that shapes this: glam's maths functions are `#[inline]`, and an
inline function is compiled into the *calling* crate at the *caller's*
opt-level. So vector and quaternion maths inside our systems runs fully
unoptimised, even though Bevy and glam themselves are built at `opt-level 3`.
The cheapest defence is not running that maths on frames that do not need it.

Still true for later: chunk meshing and terrain generation are tight numeric
loops that commonly run 10–50x slower unoptimised, so expect to raise this to
`1` once they exist — and measure rather than guess. The `Cargo.toml` comment
that contradicted the setting has been fixed.

### 1.4 Per-frame systems that could be event-driven — **done**

Every system that runs each frame was reviewed, including Bevy's own plugins.

**Converted to run conditions** — the system is skipped outright on every
frame without its trigger, so there is no query fetch and no body:

- `toggle_borderless` / `toggle_fullscreen` — `run_if(input_just_pressed(..))`
- `pause::toggle` — `in_state(InGame).and_then(input_just_pressed(PAUSE))`.
  `and_then` short-circuits, so the key is only checked inside a world.
  (`.and()` is deprecated in this Bevy version.)

**Reordered so the common case exits first:**

- `fly` reads the six movement keys *before* querying the camera or doing any
  maths. Standing still now costs six key lookups; previously it also did a
  camera query and two quaternion rotations (`forward()`, `right()`) every
  frame. The key-to-axis logic moved into a plain `axis()` function with unit
  tests, including that opposing keys cancel.
- `look` sums the frame's mouse deltas and returns before the query if the
  sum is zero. It also now does one angle update and one quaternion rebuild
  per frame, however many motion events arrived. Side effect: pitch is clamped
  once on the summed movement rather than per event, which is if anything
  more faithful to what the mouse actually did.

**Two engine plugins disabled** at runtime in `AppPlugin` — no Bevy recompile:

- `AudioPlugin` — no sound exists, but it opened an output device and kept an
  audio thread alive all session.
- `GilrsPlugin` — no controller support exists, but it polled the OS for
  gamepad events every frame.

Both were confirmed to be leaf plugins (no Bevy crate depends on
`bevy_audio` or `bevy_gilrs`), and a 15-second run showed a clean startup.

**Considered and deliberately left:**

- `log_state_change` — an `on_message` run condition would perform the same
  empty-queue check the system already does, so it would save nothing.
- `debug_jump_to_state` — three key lookups, debug builds only. Chaining three
  run conditions would cost more than it saves.
- Picking, animation, gizmos, scene and sprite plugins also run per-frame
  systems over empty queries. They were not disabled: `bevy_ui` depends on
  `bevy_sprite`, glTF loading depends on the animation and scene plugins, and
  menu buttons will soon need picking. Disabling them is a feature-level job
  (see 1.1), not a runtime one.

---

## Tier 2 — Readability and discoverability

### 2.1 The folder structure disagrees with the state machine

**The most substantive finding in this file.**

`paused/` sits at the top level of `src/`, as a sibling of `ingame/`. But
`Paused` is a *sub-state* of `InGame` — that was the whole point of the
sub-state change. So the code says "paused lives inside in-game" while the
folders say "paused is a peer of in-game."

Someone new reading the folder tree would reasonably conclude they are
independent states. Moving it to `ingame/paused/` would make the layout
mirror the state hierarchy it already implements.

### 2.2 Camera modules are named by technology, not purpose — **half done**

`camera_2d.rs` is now `camera_ui.rs` (plugin `UiCameraPlugin`), because it
was never a 2D game camera — it only draws the UI, and calling it "2d" made
it look like it needed the `2d` engine feature.

`camera_3d.rs` keeps its name for now. It does render in 3D, so the name is
not wrong, but it now sits beside a purpose-named sibling. `camera_world.rs`
would make the pair consistent. Left as a question, since "3d" is accurate
and matches how the cameras were originally described.

### 2.3 Settings live in two unrelated places — **done**

Resolved: `config/` now holds `window.rs` and `input.rs`, and a new category
is a new file. See `IMPROVEMENTS.md`.

### 2.4 Top-level `src/` mixes states with infrastructure

`src/` holds `app/`, `camera/`, `config/`, `utils/` (infrastructure)
alongside `loading/`, `menu/`, `ingame/`, `paused/` (states), with nothing
marking which is which. At eight folders it is still readable. With `world/`,
`render/`, and `player/` added it becomes ten-plus, and the distinction
blurs.

You explicitly rejected nesting the states under `states/`, so this is **not**
a re-proposal of that. It is a flag that the thing that made you reject it —
wanting each state to own a real folder — is compatible with grouping later
if the top level gets crowded. Worth watching rather than acting on.

---

## Tier 3 — Modularity

### 3.1 `camera_3d.rs` contains a player controller

At 189 lines it is the largest file, and it holds two different categories:
camera *setup* (spawn, order, MSAA, despawn) and player *input handling*
(`look`, `fly`, cursor grab).

`CLAUDE.md`'s own target layout assigns "controller, interaction" to a
`player/` module. Flying a camera with WASD is player control that happens to
move a camera. Splitting it would leave `camera/` owning cameras and give
`player/` its first real content.

This is also the natural moment to do it, because the controller is about to
grow gravity and collision, which are definitely not camera concerns.

### 3.2 `utils/window.rs` does two lifecycles

It holds `primary_window_plugin()` (runs once, at startup) and
`WindowControlPlugin` (runs every frame, forever). Those are different kinds
of thing sharing a file because they share a noun.

At about 100 lines it is not a problem yet. If it grows, splitting startup
from runtime controls is the move — and a runtime-controls module that keeps
growing arguably stops meeting the `utils/` bar of being *finished*.

### 3.3 `input/` is a different kind of module — **resolved**

The constants moved to `config/input.rs` and the `input/` folder was removed.
It returns as a real module when input behaviour (rebinding, gamepad) exists.

---

## Tier 4 — Everything else

### 4.1 Commit the restructure — **partly resolved**

There is now one commit (`chore: initialize openmc_b repository`), so history
exists. But everything since — the `config/`/`utils/` split, the camera
rename, the audit, the lint move — is uncommitted, and several files show as
deleted relative to that commit. The working tree has drifted a long way from
the only snapshot.

### 4.2 Runtime verification — **partly done**

A 15-second run confirmed: clean startup with no panics, errors or warnings;
the Vulkan loader noise is filtered; audio and gilrs are gone; and the path
`Loading -> Menu -> InGame -> Playing` works, including the 3D scene spawning
and the sub-state logging.

Still unconfirmed by eye: camera feel, mouse sensitivity (`0.002`), move speed
(`12.0`), both window toggles, the translucent pause overlay, and whether the
UI camera's non-clearing causes artefacts in `Menu`/`Loading`.

### 4.3 `Loading` never advances

Nothing transitions `Loading → Menu`. In a debug build you escape with the
`2` key; in a **release** build there are no debug keys, so the game would
sit on the loading screen permanently. This is the first real gameplay gap.

### 4.4 Almost no tests

One test exists (key-binding uniqueness). `cargo test` was measured at 11
seconds warm, so tests are cheap to run here. Candidates that are pure logic
and would not need Bevy: the pitch clamp, the movement direction composition,
and the borderless/decorations inversion.

### 4.5 A stale comment in `Cargo.toml` — **done**

The dev-profile comment claimed opt-level 1 while the setting was 0, and the
Bevy comment referred to "everything you listed". Both rewritten, and the
opt-level note from 1.3 folded in so the warning lives next to the setting it
concerns.

### 4.6 The `[lints]` schema warning — **done, on the second attempt**

"Even Better TOML" flagged `[lints.rust] unsafe_code = "deny"` as invalid.
Cargo disagrees: `cargo verify-project` succeeds and a deliberate `unsafe`
block is rejected.

The first diagnosis was wrong. It assumed the extension had an outdated
schema and added a `#:schema` directive pointing at schemastore — which did
nothing, because the extension was already using that exact schema. Reading
its cached copy showed the real cause: `cargo.json` defines `lints.rust` as a
cross-file `$ref` to `cargo-lints-rust.json`, and that second file *does*
list `unsafe_code` with `"deny"` as a valid level. So the TOML is valid under
the schema; the extension (0.21.2) fails to evaluate across the reference.

No manifest edit can satisfy it while the table exists, and an editor-settings
workaround would only fix one person's editor. The lint moved to
`#![deny(unsafe_code)]` in `main.rs`, which is equivalent for a single crate
and shows no error to anyone who clones the repo. The trade-off: integration
tests, benches, and workspace members are separate crates that only `[lints]`
covers, so it should move back when any of those appear. That note lives in
`main.rs`, `Cargo.toml`, and `CLAUDE.md`.
