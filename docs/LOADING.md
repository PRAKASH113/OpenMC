# Loading — Design Notes

What exists today, and what is planned but deliberately **not built yet**.
Nothing below the "Planned" heading has code; this is the design to follow
when the need arrives.

---

## Today: solid loading at boot

`GameState::Loading` is the game's starting state. It shows a full-screen
loading screen and nothing else runs. Its job is *boot* loading: whatever
the game needs before the menu can appear.

There is nothing to load yet, so `states/loading/mod.rs` moves straight on to
`GameState::Menu` on the first frame. When real boot assets exist, that exit
gains one extra run condition — `.and_then(boot_assets_loaded)` — and the
screen stays up until they arrive. The system itself does not change.

---

## Planned

Loading in this game is really three different jobs. They look similar on
screen, but they belong in three different places.

| Kind | What it is | Where it lives |
| --- | --- | --- |
| Boot loading | Startup assets, before the menu | `GameState::Loading` — exists |
| World generation | Building chunks around spawn | First sub-state of `InGame` |
| Soft loading | A brief overlay over the current screen | Not a state — a resource plus an overlay |

### World generation: a sub-state of `InGame`, not a trip back to `Loading`

When a world is entered, chunks near spawn must be generated before the
player can be dropped in. The natural shape is:

```rust
pub enum InGameState {
    #[default]
    Generating, // solid loading screen, player cannot move yet
    Playing,
    Paused,
}
```

Why inside `InGame` rather than going back to `GameState::Loading`:

- **The world must exist while it is being generated.** Its data, its chunk
  tasks, its entities all belong to `InGame`. If generation happened in
  `Loading`, the world would be created in one state and handed to another,
  and its lifetime would belong to neither.
- **Loading never finishes in a voxel game.** Chunks keep streaming as the
  player moves. `Generating` ends at a *threshold* — "enough chunks near
  spawn are ready" — not at a finish line, and streaming carries on during
  `Playing` without any state change.
- **Pausing stays correct.** The pause toggle maps `Playing <-> Paused` only,
  so it simply does nothing during `Generating`.

`Generating` can reuse the same solid loading screen as boot loading. The
screen is the shared part; the states differ in what they wait for.

### Soft loading: not a state at all

A split-second overlay that sits on top of whatever the player is looking at
— the menu, the game — blocks input, and shows that something is working.

It must **not** be a `GameState` variant. Entering a state exits the current
one, which would despawn the very menu the overlay is meant to cover; leaving
would respawn it. The result would be a flicker plus a wasted teardown and
rebuild, for something that should be invisible.

The planned shape:

- A resource counting loads in progress, e.g. `SoftLoading(u32)`. A counter
  rather than an on/off flag, because two overlapping loads must keep the
  overlay up until *both* finish — a flag would drop it after the first.
- An overlay spawned when the count rises above zero and despawned when it
  returns to zero, driven by a `resource_changed` run condition so it costs
  nothing while idle.
- Input-handling systems gain a `run_if(no_soft_loading)` condition, so input
  is ignored while the overlay is up. A full-screen overlay node also blocks
  mouse clicks from reaching UI underneath.

Performance: while idle this is one resource read inside run conditions that
already exist, and the overlay entities only exist while something is
loading.

---

## Worth evaluating when real assets arrive

`bevy_asset_loader` is a widely used crate that manages exactly the boot
case: declare asset collections, name the loading state and the state to
continue to, and it transitions when everything is loaded. Check that it
supports the Bevy version in use before adopting it, and add it alongside
another dependency change, since any new dependency is a rebuild.
