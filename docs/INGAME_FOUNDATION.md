# In-Game Foundation

The first playable slice: a 3D scene, a camera you can fly, and a pause that
is structurally impossible to reach from the wrong place.

This document covers only that change set. [`ARCHITECTURE.md`](ARCHITECTURE.md)
describes the system as a whole; [`IMPROVEMENTS.md`](IMPROVEMENTS.md) is the
running decision log.

---

## Controls

| Input | Action |
| --- | --- |
| `W` `A` `S` `D` | Move relative to where the camera is facing |
| `Space` / `Left Ctrl` | Move up / down |
| `Left Shift` | Sprint (3x speed) |
| Mouse | Look |
| `Escape` | Pause / resume |
| `1` `2` `3` | Debug: jump to Loading / Menu / InGame (debug builds only) |

There is deliberately no debug shortcut to the pause state — see below.

---

## The state change

`Paused` is no longer a variant of `GameState`. It is now a variant of a new
`InGameState`, declared as a Bevy **sub-state**:

```rust
pub enum GameState { Loading, Menu, InGame }

#[derive(SubStates, ...)]
#[source(GameState = GameState::InGame)]
pub enum InGameState { Playing, Paused }
```

Bevy creates `InGameState` on entering `GameState::InGame` and removes it on
leaving. The requirement "nobody can go from menu or loading straight to
paused" is therefore satisfied **structurally**: outside a loaded world the
state does not exist, so there is no transition to write incorrectly and no
runtime guard that could be bypassed or forgotten.

This is the restructure discussed earlier and deliberately deferred. It was
done now because `InGame` started owning real resources — a scene and a
camera — which is exactly the point at which "paused with no world loaded"
stops being a curiosity and becomes a bug.

The debug jump table dropped from four keys to three as a direct consequence.
Pausing is reachable only by pausing.

---

## Module changes

```text
src/
├── app/states.rs      GameState (3 variants) + InGameState sub-state
├── camera/
│   ├── camera_2d.rs   UI camera — no longer clears
│   └── camera_3d.rs   NEW — world camera, look, fly, cursor grab
├── ingame/
│   ├── mod.rs         Registers the scene and the pause toggle
│   ├── scene.rs       NEW — six cubes and a light
│   └── pause.rs       NEW — Escape toggle
└── paused/
    ├── mod.rs         Now keyed off InGameState::Paused
    └── screen.rs      Now a translucent overlay
```

`ingame/screen.rs` — the flat green placeholder — was deleted. The 3D scene
replaces it.

---

## Rendering: the two-camera contract

Two cameras now draw to the same window, which introduces two things that
would otherwise fail as confusing visual bugs rather than errors.

**Draw order.** The world camera is `order: 0`, the UI camera `order: 1`.
Higher orders draw later, so the interface lands on top of the world. Neither
camera knows about the other; they only agree on the ordering.

**Clearing.** The UI camera now sets `ClearColorConfig::None`. It draws
*after* the world camera, so if it cleared, it would erase the world it is
meant to sit on top of. The world camera does the clearing instead.

That has a consequence worth knowing: in `Loading` and `Menu` there is no
world camera, so **nothing clears the screen**. This is safe only because
both states paint an opaque full-screen background. A future state that does
not must either clear itself or bring its own camera. The constraint is
recorded in `camera_2d.rs` next to the setting.

**MSAA** stays split per camera: `Off` on the UI camera (axis-aligned quads
and atlas text gain nothing from it) and `Sample4` on the world camera, where
cube edges genuinely alias. This is why MSAA was left a per-camera component
rather than global config.

---

## Camera control notes

**Look angles are stored, not derived.** `LookAngles { yaw, pitch }` is a
component rather than being read back from the transform each frame.
Recovering Euler angles from a quaternion is lossy, accumulates drift, and
makes clamping pitch awkward. Pitch is clamped just short of vertical so the
view can never flip over.

**The rotation rebuild is skipped on frames with no mouse input**, which is
most of them when standing still.

**Zero-length movement vectors are guarded.** `try_normalize` returns `None`
rather than the `NaN` that normalising a zero vector would produce — and a
`NaN` in a transform is permanent, poisoning the camera for the rest of the
session.

**Movement follows the camera's facing**, including pitch, which is free
flight rather than ground movement. Gravity and collision arrive with the
voxel world.

**The cursor is locked and hidden on entering `Playing`** and released on
leaving it, so a paused player gets their desktop back.

---

## What pausing actually stops

The look and fly systems are gated with
`run_if(in_state(InGameState::Playing))`, so pausing genuinely freezes the
camera rather than drawing an overlay over a view that still moves.

The Escape toggle itself is gated on `GameState::InGame` instead — running in
both sub-states, because the same key has to resume as well as pause. Gating
it on `Playing` would have made pausing one-way, which is the obvious mistake
here and is noted in the source.

The pause screen became a **translucent** overlay rather than an opaque one.
That is not decoration: seeing the world still sitting behind the overlay is
what visually distinguishes "paused" from "returned to menu", and it matches
what the sub-state means — the world is still loaded, its systems have just
stopped.

---

## Scene notes

Six cubes in two rows of three, offset in depth so the arrangement reads as
solid rather than as a flat wall from the starting camera angle. Each gets
its own colour so individual cubes stay identifiable while flying around.

One `Cuboid` mesh handle is shared by all six — identical geometry has no
reason to be uploaded to the GPU six times. The materials differ per cube, so
those are separate.

The whole scene carries one `SceneEntity` marker, so leaving the state clears
it with a single query rather than by tracking each entity.

None of this is built to grow: it exists so there is something to fly around
and judge the camera against, and the voxel world replaces it wholesale.

---

## Verified / not verified

`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` both
pass. Two compile errors were fixed along the way: `DirectionalLight` renamed
`shadows_enabled` to `shadow_maps_enabled` in this Bevy version, and
`SceneEntity` needed `pub(crate)` to stop a private type leaking into a
public system signature.

**Nothing here has been run.** The scene, the camera feel, the mouse
sensitivity, and the clear-colour behaviour in `Menu`/`Loading` are all
unverified on screen. Sensitivity and movement speed in particular are
guesses that will want tuning against how they actually feel.
