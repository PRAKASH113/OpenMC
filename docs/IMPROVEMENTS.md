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
| Layout | One top-level folder per state | Each state owns everything belonging to it, so work on one never touches another. |
| Layout | `main.rs` holds only `mod` declarations and crate attributes | Setup belongs in `app`. The only reason to reopen it is adding a module. |
| Layout | `app` submodules are private; only `GameState` is re-exported | One path to the type, and misuse of `app` internals fails to compile. |
| Config | `config.rs` is `pub const` values only — no structs | It is a settings surface meant to be read in seconds. Revisit only when values must load from disk at startup. |
| Config | Bevy enums used directly (`PresentMode`), never mirrored | A copy would lose the fallback semantics and need updating whenever Bevy's enum grows. "Let Bevy decide" is the *value* `AutoVsync`, not a separate mode. |
| Window | `BORDERLESS` is stored as the inverse of Bevy's `decorations` | Bevy models "draw the title bar"; the inversion happens once, at the boundary in `window.rs`. |
| Window | `present_mode` is applied in `window.rs` | It is a property of Bevy's `Window`, not of a render plugin. |
| Camera | One persistent camera, owned outside the states | State screens spawn only their own content and never contend over the view. |
| States | Each state's screen is self-contained; duplication is intentional | The four are meant to diverge completely; a shared abstraction would have to be torn out. |
| States | `Paused` is a sub-state of `InGame`, not a sibling | Makes "paused with no world loaded" unrepresentable instead of guarded at runtime. See [`INGAME_FOUNDATION.md`](INGAME_FOUNDATION.md). |
| Rendering | The UI camera does not clear; the world camera does | The UI draws after the world, so clearing there would erase it. States without a world camera must paint an opaque background. |
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

**Window resolution units documented.** Noted that `WindowResolution::new`
takes physical pixels, so on a display with OS scaling the window is smaller
than "1280 wide" suggests. *Perf: none. Reading: removes a surprise that
would otherwise be diagnosed by confusion.*

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
