//! `ControlMode` (Dev/Player) state, the player's physics components, the
//! collision resolver, and the crosshair — see `docs/player-physics.md`.

use bevy::prelude::*;

use crate::app::states::GameState;
use crate::config::player::{DEPTH, HEIGHT, WIDTH};

/// Which control scheme is active while `GameState::Playing`. `Dev` (the
/// default, and the only mode that existed before this) is the free-fly
/// camera with no gravity or collision, unchanged — every terrain/lighting/
/// performance round so far used it. `Player` adds gravity, the collision
/// box below, and a crosshair.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::Playing)]
pub enum ControlMode {
    #[default]
    Dev,
    Player,
}

/// Current velocity in blocks/second. Only meaningful in `ControlMode::Player`
/// — `Dev`'s free-fly movement writes directly to `Transform` and never reads
/// this.
#[derive(Component, Default)]
pub struct PlayerVelocity(pub Vec3);

/// Whether the collision box is currently resting on solid ground — only set
/// `true` when the *last* vertical movement was blocked while falling (not
/// merely "touching something"), since this is also what gates whether a
/// jump is allowed.
#[derive(Component, Default)]
pub struct Grounded(pub bool);

/// Marker for the center-screen crosshair, only present in `ControlMode::Player`.
#[derive(Component)]
struct Crosshair;

const CROSSHAIR_SIZE: f32 = 16.0;
const CROSSHAIR_THICKNESS: f32 = 2.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<ControlMode>()
            .add_systems(OnEnter(ControlMode::Player), (reset_physics, spawn_crosshair))
            .add_systems(OnExit(ControlMode::Player), despawn_crosshair);
    }
}

/// Zeroes velocity/grounded on entering `Player` mode so the first frame
/// after switching starts from a known state (e.g. not carrying over a
/// stale `Grounded(true)` from before Dev-mode flying moved the camera
/// somewhere that's no longer touching ground) rather than whatever was left
/// over from the last time Player mode was active.
fn reset_physics(mut query: Query<(&mut PlayerVelocity, &mut Grounded)>) {
    for (mut velocity, mut grounded) in &mut query {
        velocity.0 = Vec3::ZERO;
        grounded.0 = false;
    }
}

fn spawn_crosshair(mut commands: Commands) {
    commands.spawn((
        Crosshair,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            ..default()
        },
        children![
            (
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(CROSSHAIR_SIZE),
                    height: Val::Px(CROSSHAIR_THICKNESS),
                    left: Val::Px(-CROSSHAIR_SIZE / 2.0),
                    top: Val::Px(-CROSSHAIR_THICKNESS / 2.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(CROSSHAIR_THICKNESS),
                    height: Val::Px(CROSSHAIR_SIZE),
                    left: Val::Px(-CROSSHAIR_THICKNESS / 2.0),
                    top: Val::Px(-CROSSHAIR_SIZE / 2.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ),
        ],
    ));
}

fn despawn_crosshair(mut commands: Commands, crosshair: Query<Entity, With<Crosshair>>) {
    for entity in &crosshair {
        commands.entity(entity).despawn();
    }
}

/// The player's collision box size (see `config::player`), as a `Vec3` for
/// direct use with `resolve_movement`.
pub fn collision_size() -> Vec3 {
    Vec3::new(WIDTH, HEIGHT, DEPTH)
}

/// An AABB edge sitting exactly on a block boundary (e.g. `max.x == 5.0`)
/// must not be treated as overlapping block 5, which occupies `[5, 6)` — the
/// box is touching it, not inside it.
const BOUNDARY_EPSILON: f32 = 1e-4;

/// Every integer block coordinate a box spanning `[min, max)` on every axis
/// overlaps.
fn blocks_overlapping(min: Vec3, max: Vec3) -> impl Iterator<Item = IVec3> {
    let x0 = min.x.floor() as i32;
    let x1 = (max.x - BOUNDARY_EPSILON).floor() as i32;
    let y0 = min.y.floor() as i32;
    let y1 = (max.y - BOUNDARY_EPSILON).floor() as i32;
    let z0 = min.z.floor() as i32;
    let z1 = (max.z - BOUNDARY_EPSILON).floor() as i32;

    (x0..=x1).flat_map(move |x| (y0..=y1).flat_map(move |y| (z0..=z1).map(move |z| IVec3::new(x, y, z))))
}

/// Moves a `size`-shaped box (given by its lower corner, `min`) by `delta`,
/// resolved one axis at a time against `is_solid` so the box slides along a
/// wall/floor instead of stopping dead the moment *any* axis would collide.
/// Returns the resolved lower corner and which axes were blocked (index 0/1/2
/// = X/Y/Z) — the caller zeroes velocity on blocked axes and treats a
/// downward-blocked Y as "grounded."
///
/// Deliberately not a continuous/swept sweep: for each axis, tentatively
/// apply the *whole* delta, and if that overlaps a solid block, snap back to
/// the nearest block boundary in the direction of travel. This is exact
/// (not an approximation) for any `delta` magnitude, because blocks are
/// unit-aligned — the "nearest boundary" is just the closest colliding
/// block's own edge, computed directly rather than found by stepping.
pub fn resolve_movement(
    is_solid: &dyn Fn(IVec3) -> bool,
    min: Vec3,
    size: Vec3,
    delta: Vec3,
) -> (Vec3, [bool; 3]) {
    let mut min = min;
    let mut blocked = [false; 3];

    for axis in 0..3 {
        let d = delta[axis];
        if d == 0.0 {
            continue;
        }

        // Query the *entire swept path* this axis of movement passes
        // through, not just the box's final resting position — checking
        // only the endpoint would let a delta large enough to jump clean
        // past a thin obstacle (a lag spike, or simply a fast enough fall)
        // tunnel through it undetected, since the final position alone
        // might not overlap anything even though the path did.
        let mut sweep_min = min;
        let mut sweep_max = min + size;
        if d > 0.0 {
            sweep_max[axis] = min[axis] + d + size[axis];
        } else {
            sweep_min[axis] = min[axis] + d;
        }

        let hit_coords: Vec<i32> = blocks_overlapping(sweep_min, sweep_max)
            .filter(|block| is_solid(*block))
            .map(|block| block[axis])
            .collect();

        if hit_coords.is_empty() {
            min[axis] += d;
            continue;
        }

        blocked[axis] = true;
        min[axis] = if d > 0.0 {
            *hit_coords.iter().min().unwrap() as f32 - size[axis]
        } else {
            *hit_coords.iter().max().unwrap() as f32 + 1.0
        };
    }

    (min, blocked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// A tiny in-memory "world" for testing, independent of `ChunkManager`/
    /// Bevy entirely — exactly the kind of fake the `is_solid: &dyn Fn`
    /// parameter exists to make possible.
    fn world(solid: &'static [(i32, i32, i32)]) -> impl Fn(IVec3) -> bool {
        let solid: HashSet<IVec3> = solid.iter().map(|&(x, y, z)| IVec3::new(x, y, z)).collect();
        move |block| solid.contains(&block)
    }

    /// After resolution, the box must never actually overlap a solid block —
    /// the one invariant that matters regardless of exactly where it stops.
    fn assert_no_overlap(is_solid: &dyn Fn(IVec3) -> bool, min: Vec3, size: Vec3) {
        for block in blocks_overlapping(min, min + size) {
            assert!(
                !is_solid(block),
                "box at min={min:?} size={size:?} overlaps solid block {block:?}"
            );
        }
    }

    #[test]
    fn falls_freely_through_open_air() {
        let is_solid = world(&[]);
        let size = Vec3::new(0.6, 2.0, 0.6);
        let (min, blocked) = resolve_movement(&is_solid, Vec3::new(0.0, 10.0, 0.0), size, Vec3::new(0.0, -5.0, 0.0));
        assert_eq!(min, Vec3::new(0.0, 5.0, 0.0));
        assert_eq!(blocked, [false, false, false]);
    }

    #[test]
    fn lands_exactly_on_top_of_the_ground() {
        // A solid floor at y=4 (occupies world space y in [4, 5)).
        let is_solid = world(&[(0, 4, 0)]);
        let size = Vec3::new(0.6, 2.0, 0.6);
        let start = Vec3::new(0.0, 10.0, 0.0);
        let (min, blocked) = resolve_movement(&is_solid, start, size, Vec3::new(0.0, -20.0, 0.0));
        assert_eq!(min.y, 5.0, "box should rest exactly on top of the floor at y=5");
        assert_eq!(blocked, [false, true, false]);
        assert_no_overlap(&is_solid, min, size);
    }

    #[test]
    fn stops_at_a_wall_and_slides_along_it() {
        // A wall filling the column at x=1 for a couple of Y levels, floor at y=0.
        let is_solid = world(&[(1, 0, 0), (1, 1, 0), (0, -1, 0)]);
        let size = Vec3::new(0.6, 2.0, 0.6);
        let start = Vec3::new(0.5, 0.0, 0.0);
        // Try to move diagonally into the wall while also sliding along Z.
        let (min, blocked) = resolve_movement(&is_solid, start, size, Vec3::new(2.0, 0.0, 3.0));
        assert!(min.x < 1.0, "should stop before entering the wall's block, got x={}", min.x);
        assert_eq!(blocked[0], true, "X movement should have been blocked by the wall");
        assert_eq!(blocked[2], false, "Z movement was never obstructed and should go through");
        assert_eq!(min.z, 3.0);
        assert_no_overlap(&is_solid, min, size);
    }

    #[test]
    fn stopping_position_never_overlaps_solid_blocks_negative_direction() {
        let is_solid = world(&[(-2, 0, 0)]);
        let size = Vec3::new(0.6, 2.0, 0.6);
        let start = Vec3::new(0.0, 0.0, 0.0);
        let (min, blocked) = resolve_movement(&is_solid, start, size, Vec3::new(-5.0, 0.0, 0.0));
        assert_eq!(blocked[0], true);
        assert_eq!(min.x, -1.0, "box should stop with its left edge exactly at the wall's right edge");
        assert_no_overlap(&is_solid, min, size);
    }

    #[test]
    fn no_movement_on_an_axis_is_a_no_op_not_a_collision() {
        let is_solid = world(&[]);
        let size = Vec3::new(0.6, 2.0, 0.6);
        let start = Vec3::new(1.0, 1.0, 1.0);
        let (min, blocked) = resolve_movement(&is_solid, start, size, Vec3::ZERO);
        assert_eq!(min, start);
        assert_eq!(blocked, [false, false, false]);
    }

    #[test]
    fn large_single_frame_delta_still_stops_exactly_at_the_boundary() {
        // Simulates a lag-spike-sized fall — must not tunnel through the floor.
        let is_solid = world(&[(0, 0, 0)]);
        let size = Vec3::new(0.6, 2.0, 0.6);
        let start = Vec3::new(0.0, 500.0, 0.0);
        let (min, blocked) = resolve_movement(&is_solid, start, size, Vec3::new(0.0, -1000.0, 0.0));
        assert_eq!(min.y, 1.0);
        assert_eq!(blocked[1], true);
        assert_no_overlap(&is_solid, min, size);
    }
}
