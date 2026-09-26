# Audit

A review of everything built so far, against the four priority tiers in
`CLAUDE.md`. **Nothing here is implemented** — this is a list of candidates
for you to accept, reject, or defer.

[`IMPROVEMENTS.md`](IMPROVEMENTS.md) records decisions already *made*. This
file records ones still *open*. When something here is acted on, it should
move there.

Current size: 1,366 lines of Rust across 30 files. Largest file is
`config/input.rs` at 127 lines (including its test); no source file exceeds
it. Nothing is close to the 800-line limit, so none of these are urgent.

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
- `states::debug::jump_to_state` — three key lookups, debug builds only. Chaining three
  run conditions would cost more than it saves.
- Picking, animation, gizmos, scene and sprite plugins also run per-frame
  systems over empty queries. They were not disabled: `bevy_ui` depends on
  `bevy_sprite`, glTF loading depends on the animation and scene plugins, and
  menu buttons will soon need picking. Disabling them is a feature-level job
  (see 1.1), not a runtime one.

---

## Tier 2 — Readability and discoverability

All four tier 2 items are resolved. Details are in `IMPROVEMENTS.md`.

### 2.1 The folder structure disagreed with the state machine — **done**

`paused/` now lives at `states/ingame/paused/`, inside its parent, so the
folder layout mirrors the state hierarchy it implements.

### 2.2 Camera modules named by technology, not purpose — **done**

`camera_2d.rs` became `camera_ui.rs` and `camera_3d.rs` became
`camera_world.rs` (plugins `UiCameraPlugin` and `WorldCameraPlugin`). Both are
now named for what they show.

### 2.3 Settings lived in two unrelated places — **done**

`config/` holds `window.rs` and `input.rs`; a new category is a new file.

### 2.4 Top-level `src/` mixed states with infrastructure — **done**

Every state now lives under `states/`, each in its own folder, together with
the state machine in `states/mod.rs`. The top level is five folders of
infrastructure plus `states/`, and a new state never adds a top-level folder.
This keeps the one-folder-per-state property that was asked for originally;
what changed is that the folders gained a common parent.

---

## Tier 3 — Modularity

All tier 3 items are resolved. Details are in `IMPROVEMENTS.md`.

### 3.1 `camera_world.rs` contained a player controller — **done**

Split: `camera/camera_world.rs` keeps the camera entity (marker, draw order,
MSAA, start position, `LookAngles`, spawn/despawn), and the controls moved to
a new `input/` folder — `look.rs`, `movement.rs`, `cursor.rs`. `input/`
depends on `camera/`, never the reverse. A future `player/` module will own
physics and block interaction; reading intent from the keys stays in
`input/`.

### 3.2 `utils/window.rs` did two lifecycles — **done**

Now `window/` — a top-level module, not inside `utils/` — with `setup.rs`
(runs once at startup) and `toggles.rs` (runs for the life of the game). The
shared fullscreen mode sits in `window/mod.rs`, and re-exports kept every
call site unchanged.

A sweep for the same problem elsewhere found one more, fixed the same way:
`states/mod.rs` held the state machine *and* the debug jump keys. The keys
moved to `states/debug.rs`, compiled out of release as a whole module.

The sweep also produced a second change — extracting `utils/engine.rs` for
Bevy's own plugin configuration, on the theory that it was "adapting settings
to the engine" like `window/` and `log.rs`. **This was tried and reverted.**
Deciding which engine plugins run (audio on or off, gamepad on or off) is a
composition decision — the same kind of decision as adding a domain plugin —
not an adapter producing one value from config. It moved back into
`AppPlugin::build`, inline, next to the domain plugin list it belongs beside.
`window/` also moved out of `utils/` entirely once it was a two-file lifecycle
split rather than a single small adapter — at that size it is a domain in its
own right, a sibling of `camera/` and `input/`, not a utility. `utils/` is
back to holding only `log.rs`. See `IMPROVEMENTS.md` for the full reasoning
on both reversals.

### 3.3 `input/` module — **done**

Recreated as a real module for the player controls from 3.1.

---

## Tier 4 — Everything else

### 4.1 Version control — **resolved**

Work is committed regularly now, so this is no longer tracked here.

### 4.2 Runtime verification — **partly done**

A 15-second run confirmed: clean startup with no panics, errors or warnings;
the Vulkan loader noise is filtered; audio and gilrs are gone; and the path
`Loading -> Menu -> InGame -> Playing` works, including the 3D scene spawning
and the sub-state logging.

Still unconfirmed by eye: camera feel, mouse sensitivity (`0.002`), move speed
(`12.0`), both window toggles, the translucent pause overlay, and whether the
UI camera's non-clearing causes artefacts in `Menu`/`Loading`.

### 4.3 `Loading` never advanced — **done**

`states/loading` now has a `finish` system that moves to `Menu` once boot
loading is done — today on the first frame, since there is nothing to load.
Release builds no longer sit on the loading screen forever. The fuller
loading design (world generation, soft loading) is in `LOADING.md`.

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
