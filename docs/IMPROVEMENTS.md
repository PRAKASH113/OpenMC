# Improvements Log

A running record of deliberate changes and the reasoning behind them.
[`ARCHITECTURE.md`](ARCHITECTURE.md) describes what the code *is*; this file
records *why it became that*, including decisions that were later reversed.

Read the **Standing decisions** table before proposing a change — if a new
idea contradicts a row there, that is a conversation to have, not something
to quietly undo.

---

## Standing decisions

| Area | Decision | Why |
| --- | --- | --- |
| Dependencies | `encase*` pinned to `0.12.1` in `Cargo.lock` | `0.12.2` moved to `syn 3` while `bevy_macro_utils` is on `syn 2`, breaking `bevy_encase_derive`. Do not `cargo update -p encase` past this until upstream fixes it. |
| Layout | `mod.rs` module style | Every module here has submodules, so the sibling-file style pairs each folder with a file and doubles the tree. Bevy is written this way too. |
| Layout | Every state lives under `states/`, one folder each; a sub-state's folder sits inside its parent's | The top level separates states from infrastructure, and the folders mirror the state hierarchy — `paused/` is in `ingame/` because `Paused` is a sub-state of `InGame`. |
| Layout | `main.rs` holds only `mod` declarations and crate attributes | Setup belongs in `app`. The only reason to reopen it is adding a module. |
| Layout | The state machine lives in `states/mod.rs`; each state folder is private to it | Someone looking for `GameState` finds it where the states are. `app/` keeps only `run()` and the composition root. |
| Layout | Every configurable value lives in `config/`, grouped by what it configures | One answer to "where do I change a setting?". A new category is a new file, not a decision. |
| Layout | `utils/` holds *finished* adapters between settings and the engine | The bar is completeness, not size — anything still growing a design belongs with its domain. Keeps `app/` to composition only. |
| Performance | Rare triggers are run conditions, not `if`s inside systems | The system is skipped outright when its trigger is absent — no query fetch, no body. |
| Performance | Per-frame systems exit on the common case before querying or doing maths | At `opt-level = 0`, inlined glam maths runs unoptimised in our crate. Skipping it beats speeding it up. |
| Performance | `AudioPlugin` and `GilrsPlugin` are disabled at runtime | No sound and no controllers exist. Both are leaf plugins. Re-enable Gilrs for controller support; delete the Audio line when the `audio` feature is dropped. |
| Input | `input/` holds controls that interpret input every frame; one-shot key actions live with what they change | Look, movement and cursor capture are continuous controls. Escape and F10/F11 are single actions whose logic belongs to their target, with the key as a run condition. |
| Camera | `camera/` owns camera entities only; `input/` steers them through `WorldCamera` and `LookAngles` | Dependencies run one way, `input/` -> `camera/`. A future `player/` owns physics; reading intent stays in `input/`. |
| Layout | One file per lifecycle when a module does both startup and runtime work | `window/` is `setup.rs` (once) plus `toggles.rs` (every frame). |
| Layout | Debug-only tooling lives in its own module, compiled out as a whole | `states/debug.rs` behind one `#[cfg(debug_assertions)]`, rather than a cfg on every item. |
| Layout | Engine plugin configuration lives inline in `AppPlugin::build`, not a separate adapter | Deciding which Bevy plugins run is a composition decision, the same kind as adding a domain plugin — not an adapter that turns config into one value. Tried as `utils::engine` and reverted. |
| Layout | `window/` is a top-level module, not inside `utils/` | Once split into two files by lifecycle it is a domain in its own right, a sibling of `camera/` and `input/` — not a small, complete utility. |
| Layout | `utils/` holds only complete, domain-less modules (currently `log.rs`) | The bar is completeness with no natural home elsewhere, not size — kept narrow on purpose so it does not collect an in-progress design by default. |
| Input | All key bindings live in `config/input.rs`, never inline in a system | One visible set, and a test can then prove no key is bound twice. Split from `config` by kind: `config` is engine/startup, `input` is player controls. |
| Config | `config.rs` is `pub const` values only — no structs | It is a settings surface meant to be read in seconds. Revisit only when values must load from disk at startup. |
| Config | Bevy enums used directly (`PresentMode`), never mirrored | A copy would lose the fallback semantics and need updating whenever Bevy's enum grows. "Let Bevy decide" is the *value* `AutoVsync`, not a separate mode. |
| Window | `BORDERLESS` is stored as the inverse of Bevy's `decorations` | Bevy models "draw the title bar"; the inversion happens once, at the boundary in `window.rs`. |
| Window | `present_mode` is applied in `window.rs` | It is a property of Bevy's `Window`, not of a render plugin. |
| Camera | One persistent camera, owned outside the states | State screens spawn only their own content and never contend over the view. |
| States | Each state's screen is self-contained; duplication is intentional | The four are meant to diverge completely; a shared abstraction would have to be torn out. |
| States | `Loading` is boot loading only | World generation is planned as `InGame`'s first sub-state and soft loading as a counted overlay, not a state. See [`LOADING.md`](LOADING.md). |
| States | `Paused` is a sub-state of `InGame`, not a sibling | Makes "paused with no world loaded" unrepresentable instead of guarded at runtime. See the States section of [`ARCHITECTURE.md`](ARCHITECTURE.md). |
| Rendering | The UI camera clears its own texture to transparent (`ClearColorConfig::Custom(Color::NONE)`) and is composited onto the window with premultiplied alpha | With `Msaa::Off` it renders into its own intermediate texture, not the world's, so it must clear or every frame's UI piles onto the last. See 2026-09-26. Never set it back to `ClearColorConfig::None`. |
| Logging | Log config lives in `app/log.rs`, not `main.rs` | `LogPlugin` is configured *on* `DefaultPlugins`, which happens in `AppPlugin`. Routing it through `main.rs` would mean plumbing settings down only to hand them back. |
| Logging | Filters build on `DEFAULT_FILTER` rather than replacing it | Bevy's own defaults survive; ours stay additive. |
| Debug tooling | Debug-only code is gated with `#[cfg(debug_assertions)]` | Keeps it provably out of release rather than relying on a comment. |

---

## Reversals

Decisions that were made and then undone. Listed so the discarded option is
not re-proposed as if it were new.

| Was | Now | Why it changed |
| --- | --- | --- |
| `u16` for window width/height | `u32` | The 8 bytes saved are meaningless for a struct built once, and `u16` invites an overflow footgun (`1280 * 720` does not fit) plus a cast at every Bevy boundary. |
| Sibling-file modules (`menu.rs` + `menu/`) | `mod.rs` style | Every module here has submodules, so the sibling style doubled the tree for no benefit. |
| All states nested under `states/` | One top-level folder per state | Each state should own its own folder rather than being a file in a shared one. |
| One top-level folder per state | Every state under `states/`, still one folder each | The top level had started mixing states with infrastructure. Grouping kept the folder-per-state property that was the original point. |
| UI camera `ClearColorConfig::None` ("draw over the world, don't erase it") | Clear to `Color::NONE`, premultiplied-alpha output | `None` is only correct when two cameras share an intermediate texture, which requires matching MSAA. Ours do not, so the UI texture was never cleared: a stuck, black-looking pause overlay and a frozen-looking view. See 2026-09-26. |
| `paused/` as a top-level sibling of `ingame/` | `states/ingame/paused/` | The folders now say what the state machine already said: `Paused` exists inside `InGame`. |
| `camera_2d` / `camera_3d` | `camera_ui` / `camera_world` | Named for what each camera shows rather than how it renders. |
| Engine plugin config in `utils/engine.rs` | Inline in `AppPlugin::build`, next to the domain plugin list | Disabling `AudioPlugin`/`GilrsPlugin` and swapping in our window/log plugins is deciding what the app is made of — composition, not an adapter that turns our config into one Bevy value like `window/` and `log.rs` do. |
| `window/` inside `utils/` | `window/` as a top-level module | A two-file lifecycle split (`setup.rs` + `toggles.rs`) is a domain, not a small complete utility — a sibling of `camera/` and `input/`. |
| Shared `states/screen.rs` helper | Per-state `screen.rs` | Self-contained states were preferred over DRY, since the screens are placeholders meant to diverge. |
| `Paused` as a `GameState` variant | `InGameState::Paused` sub-state | Enforcing "only pausable from in-game" structurally beats a runtime guard. Done once `InGame` owned real resources, as planned. |
| `ingame/screen.rs` placeholder | `ingame/scene.rs` | The flat colour was scaffolding; the 3D scene replaces it. |
| `GameConfig`/`WindowConfig`/`RenderConfig` structs | Plain `pub const` values | 68 lines of type machinery for 5 values. Nothing read the config after startup, so the types earned nothing. |

---

## Change log

### 2026-09-19

**Dependency fix.** Pinned `encase`/`encase_derive`/`encase_derive_impl` to
`0.12.1`. `0.12.2` bumped `syn` to 3 in a patch release, conflicting with
`bevy_macro_utils` on `syn 2` and breaking the `bevy_encase_derive` proc
macro. *Perf: none. Modularity: none — purely a build fix.*

**`main.rs` reduced to an entry point.** Module declarations, the
`windows_subsystem` attribute, and a call to `app::run`. The attribute is
gated on `not(debug_assertions)` so dev builds keep their console for logs.
*Perf: none. Modularity: setup stays in one place instead of leaking into the
entry point.*

**Plugin-per-domain composition.** Every domain is a Bevy `Plugin`, all
registered in `app::plugin::AppPlugin`. *Perf: none — `Plugin::build` runs
once at startup and a system registered via a plugin runs exactly as fast as
one registered inline. Modularity: the whole game reads as one list.*

**State machine.** `GameState` with `Loading`/`Menu`/`InGame`/`Paused`, one
module per state. *Perf: none. Modularity: adding behaviour to a state
touches only that state's folder.*

**`config.rs` reduced to constants.** Removed three structs and two `Default`
impls; 68 lines became 27. *Perf: negligibly positive — one startup `String`
allocation saved, zero effect on frame time. Modularity: better now (three
fewer types, `window.rs` lost a parameter), with one real cost — constants
cannot be loaded from a file, so a `settings.toml` would need a struct back.
Narrower than it looks, since a settings menu would mutate the live `Window`
component rather than these startup values.*

**Log filtering.** Silenced `wgpu_hal::vulkan::instance`, which emitted five
errors per startup about Steam and Epic overlay layers that are not
installed. *Perf: none. Modularity: none. Tradeoff: genuine Vulkan instance
errors go quiet too — acceptable because a real failure stops the app with a
clearer message, but comment it out first when diagnosing a GPU problem.*

**State transition logging.** `log_state_change` reports every transition,
skipping identity transitions. *Perf: negligible — an empty `MessageReader`
is a cursor check.*

**`app` submodule visibility tightened.** `config`, `log`, `plugin`,
`states`, and `window` are now private; only `GameState` is re-exported.
*Perf: exactly zero — visibility is erased before codegen. Modularity: `app`'s
coupling surface dropped from five modules to one type plus `run()`, and
reaching into its internals is now a compile error instead of an unnoticed
dependency.*

**Debug keybinds gated.** `debug_jump_to_state` and its key table are
`#[cfg(debug_assertions)]`, and the keys moved into a `DEBUG_JUMPS` table.
*Perf: the system no longer exists in release, so it costs nothing there
rather than nearly nothing. Modularity: adding a state to the shortcuts is
one table row. Verified `cargo clippy --release` is clean, so the exclusion
leaves no dead code.*

**Plugin registration grouped and ordered.** `AppPlugin` now registers in
three labelled steps — engine, shared infrastructure, states — instead of one
flat six-tuple, and records why `DefaultPlugins` must come first.
*Perf: none. Modularity/reading: the six plugins were not peers, and now do
not read as if they were. The ordering note prevents a real failure mode —
registering `GameStatePlugin` first produces no compile error, only a runtime
warning and a state machine that never transitions.*

**Window resolution units documented — later found to be wrong, see
2026-09-26.** Noted that `WindowResolution::new` takes physical pixels, so on
a display with OS scaling the window is smaller than "1280 wide" suggests.
This was an assumption from the constructor's parameter names
(`physical_width`/`physical_height`), not verified against how `bevy_winit`
actually uses the value — it turned out to be backwards for the one code path
that matters at startup. Left here uncorrected in place, rather than edited
away, precisely because acting on an unverified doc comment is what caused
the bug it led to.

**MSAA disabled on the UI camera.** Bevy defaults `Msaa` to `Sample4`, which
allocates a 4x multisampled render target and resolves it every frame. The UI
is axis-aligned quads and font-atlas text, where multisampling changes
nothing visible. *Perf: removes a 4x render target and its per-frame resolve
for zero visual cost — the largest single win found so far. Modularity: `Msaa`
is a per-camera component, so the 3D camera picks its own level
independently.*

**Explicit camera draw order.** The UI camera is now `order: 1` instead of
the default `0`. *Perf: none. Bug prevention: when the 3D camera arrives at a
lower order, the UI sits on top by construction. Two cameras sharing order
`0` on one target leaves draw order unspecified, which fails as a confusing
visual bug rather than an error.*

**`camera_2d` made private.** Nothing outside `camera` uses it. *Perf: zero.
Modularity: matches the `app` pattern — the folder exposes `CameraPlugin` and
nothing else.* A `pub(crate) use camera_2d::UiCamera;` re-export was tried and
reverted: nothing imports it yet, so it was just an unused import. Add it when
a real consumer appears.

**Key bindings centralised in `input/`.** Eleven bindings that were inline
`KeyCode` literals across `camera_3d.rs`, `pause.rs`, and `states.rs` are now
named constants in one module, with a test asserting no key is bound to two
actions. `LOOK_SENSITIVITY`, `MOVE_SPEED`, and `SPRINT_MULTIPLIER` moved there
too, as player-tunable settings. *Perf: none — constants inline identically.
Modularity/reading: a key clash was previously only findable by grepping three
files; it is now a failing test. Camera draw order and the pitch clamp
deliberately stayed in `camera_3d` — the first is a rendering detail, the
second a safety limit, and neither is a preference.*

**Runtime window toggles.** Added `FULLSCREEN` alongside `BORDERLESS` in
config (both start `false`), bound `F10`/`F11` in `input/`, and added
`WindowControlPlugin` in `app/window.rs` to mutate the live `Window`
component. *Perf: two systems doing a single `just_pressed` check per frame —
negligible, and they early-return before touching the window query. Nothing
re-creates the window; Bevy applies the component change. Modularity: window
vocabulary stayed in `app/window.rs`, keys stayed in `input/`, config holds
only starting values — each of the three files gained the part that belongs
to it.*

**Settings unified into `config/`, adapters split into `utils/`.**
`app/config.rs` became `config/window.rs`, `input/mod.rs` became
`config/input.rs`, and `app/log.rs` and `app/window.rs` moved to `utils/`.
`app/` now holds only `plugin.rs` and `states.rs`. `input/` is kept as an
empty signpost module reserved for future input behaviour. *Perf: none — pure
module reorganisation, identical codegen. Readability: settings had two homes
and now have one; `app/` dropped from six files to three, leaving only the
parts that shape how the game is assembled.*

Recorded because it was discussed and decided rather than assumed: a `utils/`
folder is normally an anti-pattern, because "miscellaneous" has no membership
test and becomes a junk drawer. It was adopted here with an explicit bar —
**finished adapters only, completeness rather than size** — written into
`utils/mod.rs` so the criterion travels with the folder. Revisit if anything
lands there that is still growing a design.

**`camera_2d` renamed to `camera_ui`.** The module and its plugin
(`Camera2dPlugin` -> `UiCameraPlugin`) now say what the camera is *for*. It
only ever drew the UI, and "2d" suggested it depended on the `2d` engine
feature, which it does not. The module doc now explains why a UI camera is
built from Bevy's `Camera2d` component, since that is exactly the confusion
the old name caused. Moved with `git mv` so history follows the file. *Perf:
none. Readability: the name matches the `UiCamera` marker it spawns.*

**`unsafe_code` deny moved from `Cargo.toml` to `main.rs`.** The `[lints]`
table is valid Cargo, but the "Even Better TOML" extension shows a false error
for it that no manifest edit can clear. As a crate attribute it is equivalent
for this single crate. *Perf: none. Readability: no false error for anyone
opening the manifest. Cost: `[lints]` would also cover future `tests/`,
benches, and workspace members, which the attribute does not — so it moves
back when those exist.* See `AUDIT.md` 4.6 for the full diagnosis, including
the first attempt that did not work.

**Stale documentation corrected.** `ARCHITECTURE.md` still described `Paused`
as a flat sibling of `InGame`, debug keys `1`-`4`, every state as a flat
colour, the 3D camera as not yet existing, an `input/` folder, and an
`ingame/screen.rs` that was deleted. All rewritten against the code, and a
link to the removed `INGAME_FOUNDATION.md` was repointed. *Readability: a doc
that contradicts the code is worse than no doc.*

**Per-frame work pass (audit 1.4).** Key-triggered systems became run
conditions; `fly` and `look` exit before querying or doing maths when there is
no input; `AudioPlugin` and `GilrsPlugin` are disabled. *Perf: runtime — on a
typical idle frame, three systems no longer execute at all, and the camera
controls no longer query or do quaternion maths when the player is still.
Startup — no audio device is opened and no gamepad backend is initialised.
Build — none; all runtime changes, no feature edits. Modularity: `fly`'s
key-reading moved into a pure `axis()` function, which is now unit-tested
without Bevy. Verified with a 15-second run: clean startup, states and
sub-states transition, scene spawns.*

**Cargo.toml Step 2 claim corrected.** The comment said dropping the `2d`
feature removes `bevy_sprite`. It does not: `bevy_ui` depends on `bevy_sprite`
directly, so only `bevy_sprite_render` goes. Step 2 is still worth doing
alongside Step 1, but it is a small win, and the comment now says so.

**Tier 2 restructure: states grouped, camera renamed.** Every state moved
under `states/`, with the state machine in `states/mod.rs` (formerly
`app/states.rs`) and `paused/` inside `ingame/`. `camera_3d.rs` became
`camera_world.rs` (plugin `WorldCameraPlugin`). Registration now nests the
same way as the folders: `AppPlugin` adds `GameStatePlugin`, which adds the
top-level states, and `InGamePlugin` adds `PausedPlugin`. All moves used
`git mv`. *Perf: none — module reorganisation only. Readability: the top
level is now five infrastructure folders plus `states/`; the state hierarchy
is visible in the tree; both cameras are named for what they show; adding a
state touches only its parent's `mod.rs`.*

**`Loading` now exits to `Menu`.** A `finish` system gated on
`in_state(Loading)` moves on once boot loading is done — today on the first
frame. Before this, a release build (no debug keys) sat on the loading screen
forever. The wider loading design is written up in `LOADING.md` rather than
built.

**`cargo test --release` fixed.** The key-uniqueness test named the three
debug keys unconditionally, but they only exist in debug builds, so the test
failed to compile in release. The debug entries are now added under
`#[cfg(debug_assertions)]`. The bug predated this change; it surfaced when
the release lint was run with `--all-targets`.

**Tier 3: controls split from the camera.** `camera/camera_world.rs` held both
the camera entity and the player controls. The controls moved to a new
`input/` folder — `look.rs`, `movement.rs` (with `axis()` and its tests), and
`cursor.rs` — registered by `GameInputPlugin` and gated on `Playing`. The
camera keeps its marker, draw order, MSAA, start position, `LookAngles`, and
spawn/despawn; it re-exports `WorldCamera` and `LookAngles` for `input/` to
use. `input/` depends on `camera/`, never the reverse. The pitch clamp moved
with `look`, the control that enforces it. Inside `input/`, `config::input`
is imported as `controls` so a bare `input::` never reads as the module
itself. *Perf: none — the same systems with the same run conditions,
registered from a different plugin. Modularity: `camera_world.rs` went from
237 lines of two concerns to 79 of one; each control is its own file.*

**Tier 3: window split by lifecycle.** `utils/window.rs` became
`utils/window/` with `setup.rs` (once, at startup) and `toggles.rs` (every
frame, for the life of the game). The fullscreen mode both use is in
`window/mod.rs`, and re-exports kept every call site unchanged. *Perf: none.
Modularity: each file has one lifecycle.*

**Two more files doing two jobs, found by sweeping the codebase.**
`states/mod.rs` held the state machine and the debug jump keys; the keys
moved to `states/debug.rs`, compiled out of release as one module instead of
through a cfg on each item. `app/plugin.rs` held the composition root and the
engine plugin configuration; the configuration moved to `utils/engine.rs`,
beside the window and log adapters it uses, so `AppPlugin` only assembles the
game. *Perf: none. Modularity: every file now has one job.* — **Partly
reverted, see below.**

**Correction: `utils/engine.rs` folded back into `AppPlugin`, `window/`
promoted out of `utils/`.** Challenged on the reasoning above: deciding which
Bevy plugins run — swapping in our window/log plugins, disabling
`AudioPlugin`/`GilrsPlugin` — is a composition decision, indistinguishable in
kind from adding a domain plugin, not an adapter that turns our config into
one Bevy value the way `window/` and `log.rs` do. It moved back inline into
`AppPlugin::build`, next to the domain plugin tuple it belongs beside. At the
same time, `window/` moved out of `utils/` to the top level: a two-file split
by lifecycle is a domain in its own right (a sibling of `camera/` and
`input/`), not a small complete utility. `utils/` is back to holding only
`log.rs`. See the reversals table above. *Perf: none — both changes are
reorganisation only. Readability: one file lists everything the app is made
of, engine plugins included, matching what `AppPlugin`'s own doc comment
already claimed; `utils/` stays narrow enough that "does this belong in
utils?" keeps having a clear answer.*

---

### 2026-09-26

**Fixed: window returned smaller than it started after leaving fullscreen.**
Reported as: open at 1280x720, which the OS scales normally; press F11 twice
(fullscreen, then back) and the window comes back noticeably smaller, as if
OS scaling stopped applying.

Root cause, confirmed against `bevy_winit` 0.19.1 source rather than assumed:
the *same* `WIDTH`/`HEIGHT` numbers are handled by two different code paths
depending on when they're set.

- **At window creation** (`window::setup`), Bevy hands winit a `LogicalSize`
  built from `window.width()`/`height()` — and since a freshly-constructed
  `WindowResolution` defaults `scale_factor` to `1.0`, those logical values
  are numerically equal to `WIDTH`/`HEIGHT`. Winit then converts *logical to
  physical* using the monitor's real DPI. `1280` logical becomes `1920`
  physical at 150% scaling — the window looks the same size as it would
  anywhere else.
- **On a live window** (`window::toggles`, restoring from fullscreen), Bevy
  compares `resolution.physical_width()`/`physical_height()` directly and
  calls `winit_window.request_inner_size()` with them as **exact physical
  pixels** — no DPI conversion at all.

`toggle_fullscreen` was doing `window.resolution = WindowResolution::new(WIDTH, HEIGHT)`
— constructing a *fresh* `WindowResolution`, which resets `scale_factor` to
its `1.0` default and feeds `1280`/`720` into the physical-pixels path. At
150% scaling that is visually two-thirds the size the window opened at.

Fix: `window.resolution.set(WIDTH as f32, HEIGHT as f32)` instead. `.set()`
mutates the *existing* `WindowResolution` in place — so it keeps whatever
`scale_factor` Bevy has been tracking live from the OS — and internally
multiplies by that scale factor before writing the physical fields. Same
logical-to-physical conversion as window creation, just computed by us
instead of by winit. *Perf: none, a startup/toggle-time fix only.
Correctness: the window now returns to the same visual size on any display
scaling, not just 100%.*

**Corrected the doc comments this bug traces back to.** `config::window::WIDTH`/`HEIGHT`
and `window::setup`'s resolution comment both asserted "physical pixels,"
reasoning from `WindowResolution::new`'s parameter names
(`physical_width`/`physical_height`) rather than from how `bevy_winit`
actually consumes the value. That assumption was never verified against the
winit integration before being written down — this is the second time in
this project an unverified comment about engine internals turned out
backwards (the first was the `[lints]` schema diagnosis). Both comments now
state which code path applies and why, with a cross-reference between
`config::window`, `window::setup`, and `window::toggles` so the two lifetimes
of the same constants aren't read in isolation again.

**Fixed: the pause overlay read as a completely different screen.** Reported
as: pausing should feel like an overlay on the game, but it looked like a
full screen swap instead — the same complaint the overlay was specifically
designed to avoid.

Before assuming the fix was cosmetic, the camera architecture was checked
against Bevy's actual UI-camera resolution (`DefaultUiCamera::get` in
`bevy_ui`), since `InGame`/`Paused` is the first state where the UI camera
and the world camera are ever alive at once — every earlier state only ever
had one camera, so this path had never been exercised. Confirmed correct:
with neither camera marked `IsDefaultUiCamera`, Bevy's fallback picks the
camera with the highest `order` targeting the primary window, which is the
UI camera (`order: 1` vs. the world camera's `0`) by construction. UI was
never misrouted to the wrong camera.

The actual cause was the overlay colour: `Color::srgba(0.05, 0.03, 0.08, 0.75)`
is 75% opacity of near-black over a lit 3D scene. At that strength a
near-black tint reads as solid, not dimmed — technically translucent, but
visually indistinguishable from the opaque Loading/Menu screens it was meant
to be unlike. Lowered to `0.45`. *Perf: none, a colour constant only.
Correctness: ruled out the more serious possible cause (wrong camera
targeting) by reading Bevy's source rather than assuming the visible-camera
architecture just worked because it compiled — worth remembering the next
time two cameras coexist for the first time in a new state.*

**Not visually confirmed** — this environment cannot render the game.
0.45 is a reasoned starting point (a common overlay strength), not a
measured one; it may still want tuning once seen.

**Fixed, for real this time: the pause overlay that never cleared, looked
black at any alpha, and the view that looked frozen on first entering the
game.** Reported as three separate bugs: controls do not engage on the first
`Menu -> InGame` (pressing `2` then `3` "fixed" it); the pause overlay looks
pitch black instead of translucent; after resuming, the overlay stays on
screen forever while the world visibly moves behind it.

They were one bug. Root cause, read from `bevy_render` and
`bevy_core_pipeline` 0.19.1 source:

- `prepare_view_targets` gives each camera an intermediate "main texture"
  keyed on `(target, usage, format, msaa)`. The UI camera is `Msaa::Off` and
  the world camera is `Sample4`, so **they do not share one**. The UI camera
  draws into a texture the world is never in.
- That texture's colour attachment only clears on the first use per frame
  *if it has a clear colour*. `ClearColorConfig::None` gives it none, so its
  load op is always `Load`: **it was never cleared**. `TextureCache` hands
  out the same GPU texture frame after frame, so each frame's UI landed on
  top of every earlier frame's.
- The `upscaling` node then blends that texture onto the window with alpha
  blending, because it is the second camera on the window.

Every symptom follows. A 0.3-alpha overlay drawn over its own previous
frame 10 times is 97% opaque, so any alpha looked black. After despawn,
nothing overwrites the texture, so the overlay and its text stay composited
over the live world indefinitely. That is also why minimising or resizing the
window did not help: the world *was* re-rendering. And because the world
camera's own main textures have the identical descriptor (same label,
size, format, sample count 1), the two cameras draw from one `TextureCache`
pool. The UI camera can be handed a texture that last held an opaque world
frame and blend it over the live one, which looks exactly like frozen
controls while input is in fact working (the diagnostic logs showed look
and movement firing from the first frame). Leaving and re-entering the game
reshuffles which texture each camera gets, which is why `2` then `3`
appeared to fix it.

Fix, in `camera::camera_ui`:

- `clear_color: ClearColorConfig::Custom(Color::NONE)` — clear the UI's own
  texture to transparent every frame.
- `output_mode: CameraOutputMode::Write { blend_state:
  Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING), clear_color: Default }`.
  The UI pipeline alpha-blends onto a transparent texture, which leaves
  premultiplied colour, so the default straight-alpha composite would
  multiply by alpha twice and darken text edges and the overlay. The window
  clear only applies to the first camera writing each frame: the world
  camera in-game, the UI camera in menus.

Kept `Msaa::Off` on the UI camera. The alternative fix, matching MSAA so
both cameras share one texture and `None` works as intended, would bring
back the 4x UI render target that entry removed. The overlay alpha went back
up to 0.5, since the earlier "too dark" reports were accumulation, not alpha.

*Perf: one transparent clear of the UI texture per frame, a single
full-screen fill, far cheaper than the 4x MSAA target the alternative
fix needed. Correctness: fixes all three reported bugs. It also drops the
old rule that states without a world camera must paint an opaque
background; the UI camera no longer relies on it.*

**The wrong turns, recorded so they are not retried.** Each was checked and
ruled out before the real cause was found:

- **Overlay alpha too strong** (0.75 → 0.45 → 0.3). Never the cause; see
  above. The earlier entry today blaming the colour is wrong.
- **OS focus loss dropping the cursor grab**, answered with a
  `regrab_on_focus_regained` system and a `Window::focused = true` request
  after the F10/F11 toggles. Logs showed no `WindowFocused` events at all
  during the failing transition, and input arriving normally. Both removed.
- **Escape double-firing** and flipping straight back to `Paused`, answered
  with a 0.25 s debounce. Logs showed exactly one toggle per press. Removed.
- **Non-recursive despawn** and **sub-state `OnExit` not firing**, both
  disproved from Bevy source. A diagnostic system confirmed the
  `PausedScreen` entity count went `1 -> 0` on every resume.

The lesson: once logs prove the ECS state is right and the picture is still
wrong, stop changing game logic and read the render path. Here that meant
`prepare_view_targets` and the `upscaling` node, not `bevy_ui`.

---

## Known open items

- **The repository has no commits.** Everything, including `Cargo.toml`, is
  untracked. There is no safety net under any of this.
- **Nothing has been visually verified.** The placeholder screens and window
  settings type-check but have not been confirmed on screen.
- **Release builds have no console** (`windows_subsystem = "windows"`), so
  logs go nowhere there. Diagnosing a release build needs a file layer via
  `LogPlugin::custom_layer`.
- **Temporary scaffolding to delete**: the placeholder state screens and
  `debug_jump_to_state`.
