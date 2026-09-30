//! Gravity, jumping, and collision with the terrain.
//!
//! The player is an axis-aligned box ([`config::HITBOX_WIDTH`] wide,
//! [`config::HITBOX_HEIGHT`] tall, standing on its `Transform`'s position).
//! Each frame it moves by its velocity one axis at a time, and any axis that
//! would push it into a solid block stops flush against that block instead.
//! Per-axis is what lets the player slide along a wall rather than stick to
//! it: the blocked axis stops, the others carry on.
//!
//! The collision maths is plain functions over an "is this block solid"
//! callback, so it's unit-tested below without a world or an `App`.

use bevy::prelude::*;

use super::{Flying, MovementIntent, Player};
use crate::config::player as config;
use crate::world::LoadedChunks;

/// The player's physical state carried between frames.
#[derive(Component, Default)]
pub(crate) struct Motion {
    /// Vertical velocity on foot, blocks per second, positive up. Kept at
    /// zero while flying, so landing starts from rest.
    vertical_velocity: f32,
    /// Whether the player is standing on something solid.
    grounded: bool,
    /// Whether the *last* move was stopped by a solid block on `x` or `z`.
    /// Deliberately never `y` — landing blocks that axis every frame while
    /// walking on flat ground, and that must never read as a wall hit.
    horizontal_collision: bool,
}

impl Motion {
    /// Whether the player's last move was stopped by a wall. Read by
    /// `input::movement` to cancel a sticky sprint after hitting one — see
    /// [`crate::config::player::RESET_SPRINT_ON_COLLISION`].
    pub(crate) fn horizontal_collision(&self) -> bool {
        self.horizontal_collision
    }
}

/// Whether terrain collision is currently in effect for the player.
///
/// A testing aid — see `player::debug` (debug builds only) for how it's
/// toggled. Defined unconditionally, unlike the toggle itself, so
/// [`apply_physics`] reads the same in every build: always enabled in
/// release, since nothing there can ever set it otherwise. Same shape as
/// `world::ChunkLock`.
#[derive(Resource)]
pub(crate) struct CollisionEnabled(pub bool);

impl Default for CollisionEnabled {
    fn default() -> Self {
        Self(INITIALLY_ENABLED)
    }
}

/// The starting value of [`CollisionEnabled`]. In debug builds this reads
/// `config::debug::COLLISION_INITIALLY_DISABLED`, but only when
/// `TESTING_TOOLS_ENABLED` is also `true` — otherwise collision would start
/// off with no hotkey able to turn it back on, since the hotkey itself only
/// exists when testing tools are enabled. In release this is always `true`:
/// nothing there can read a debug-only constant, and no real build should
/// ever start unable to stand on the ground.
#[cfg(debug_assertions)]
const INITIALLY_ENABLED: bool = !(crate::config::debug::TESTING_TOOLS_ENABLED
    && crate::config::debug::COLLISION_INITIALLY_DISABLED);

#[cfg(not(debug_assertions))]
const INITIALLY_ENABLED: bool = true;

/// Moves the player by what [`MovementIntent`] asks for, under gravity unless
/// [`Flying`], stopping at terrain unless [`CollisionEnabled`] is off.
pub(super) fn apply_physics(
    time: Res<Time>,
    world: Res<LoadedChunks>,
    collision: Res<CollisionEnabled>,
    mut player: Query<(&mut Transform, &mut Motion, &MovementIntent, &Flying), With<Player>>,
) {
    let Ok((mut transform, mut motion, intent, flying)) = player.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let idle = intent.horizontal == Vec3::ZERO && intent.vertical == 0.0 && !intent.jump;

    // Standing still on the ground, or hovering in flight, is the common
    // case, and nothing can move the player then. Skipping it also leaves
    // `Transform` untouched, which is what keeps chunk loading and the
    // camera follow idle too — both run only when it changes.
    let delta = if flying.0 {
        motion.vertical_velocity = 0.0;
        motion.grounded = false;
        if idle {
            motion.horizontal_collision = false;
            return;
        }
        (intent.horizontal + Vec3::Y * intent.vertical) * dt
    } else {
        if idle && motion.grounded {
            motion.horizontal_collision = false;
            return;
        }
        if intent.jump && motion.grounded {
            motion.vertical_velocity = config::JUMP_SPEED;
        }
        motion.vertical_velocity =
            (motion.vertical_velocity - config::GRAVITY * dt).max(-config::TERMINAL_VELOCITY);
        intent.horizontal * dt + Vec3::Y * motion.vertical_velocity * dt
    };

    if !collision.0 {
        // Noclip: apply the raw delta with no block checks at all. Grounded
        // and collision state stop meaning anything without a floor to rest
        // on, so both go false rather than keep a stale value from before
        // collision was switched off.
        motion.grounded = false;
        motion.horizontal_collision = false;
        transform.translation += delta;
        return;
    }

    let moved = move_and_collide(transform.translation, delta, |block| world.is_solid(block));
    motion.horizontal_collision = moved.blocked.x || moved.blocked.z;

    if moved.stuck {
        // Inside a chunk that hasn't generated yet: wait, from rest, and
        // don't claim to be grounded, so gravity picks up again the moment
        // the chunk arrives.
        motion.vertical_velocity = 0.0;
        motion.grounded = false;
    } else if moved.blocked.y {
        motion.grounded = delta.y < 0.0;
        motion.vertical_velocity = 0.0;
    } else {
        motion.grounded = false;
    }

    if moved.position != transform.translation {
        transform.translation = moved.position;
    }
}

/// The furthest the box moves in one collision step. Under one block, so a
/// single step can never jump clean over a block-thick wall or floor; a
/// longer move (a fast fall, a slow frame) is split into several steps.
const MAX_STEP: f32 = 0.45;

/// How far short of a block the box stops, so rounding can never leave it
/// overlapping what it just stopped against.
const EPSILON: f32 = 1e-3;

/// Where an attempted move ended up.
#[derive(Debug)]
struct Moved {
    position: Vec3,
    /// Which axes were stopped by a solid block.
    blocked: BVec3,
    /// The box started inside something solid and didn't move at all.
    stuck: bool,
}

/// Moves the player's box from `position` by `delta`, one axis at a time,
/// stopping each axis flush against the first solid block in its way.
///
/// Vertical goes first in each step, so landing is settled before sliding:
/// a player walking along the ground never snags on the block they're
/// standing on. If the box already overlaps a solid block before moving —
/// in practice, only a chunk that hasn't generated yet — nothing moves and
/// the result says `stuck`, rather than guessing which way is out.
fn move_and_collide(position: Vec3, delta: Vec3, is_solid: impl Fn(IVec3) -> bool) -> Moved {
    if overlaps_solid(position, &is_solid) {
        return Moved {
            position,
            blocked: BVec3::TRUE,
            stuck: true,
        };
    }

    let steps = (delta.abs().max_element() / MAX_STEP).ceil().max(1.0) as u32;
    let step = delta / steps as f32;
    let mut position = position;
    let mut blocked = [false; 3];

    for _ in 0..steps {
        for axis in [1, 0, 2] {
            if blocked[axis] || step[axis] == 0.0 {
                continue;
            }
            let (next, hit) = move_axis(position, axis, step[axis], &is_solid);
            position = next;
            blocked[axis] = hit;
        }
    }

    Moved {
        position,
        blocked: BVec3::from(blocked),
        stuck: false,
    }
}

/// Moves along one axis by `amount` (at most [`MAX_STEP`]), or, if that
/// would overlap a solid block, stops flush against it. Returns the new
/// position and whether a block got in the way.
fn move_axis(
    position: Vec3,
    axis: usize,
    amount: f32,
    is_solid: &impl Fn(IVec3) -> bool,
) -> (Vec3, bool) {
    let mut moved = position;
    moved[axis] += amount;
    if !overlaps_solid(moved, is_solid) {
        return (moved, false);
    }

    // The step is under a block long, so the only blocks it can have run
    // into are the single layer just past the box's leading face — and that
    // layer's near boundary is a whole number.
    let (min, max) = bounds(moved);
    let (below, above) = extents(axis);
    let flush = if amount > 0.0 {
        (max[axis].floor() - above - EPSILON).max(position[axis])
    } else {
        (min[axis].floor() + 1.0 + below + EPSILON).min(position[axis])
    };

    let mut stopped = position;
    stopped[axis] = flush;
    if overlaps_solid(stopped, is_solid) {
        return (position, true);
    }
    (stopped, true)
}

/// How far the box reaches below and above `position` along `axis`: the feet
/// are the bottom of the box, and it's centred horizontally.
fn extents(axis: usize) -> (f32, f32) {
    if axis == 1 {
        (0.0, config::HITBOX_HEIGHT)
    } else {
        let half = config::HITBOX_WIDTH / 2.0;
        (half, half)
    }
}

/// The box's corners for a player standing at `position`.
fn bounds(position: Vec3) -> (Vec3, Vec3) {
    let half = config::HITBOX_WIDTH / 2.0;
    (
        position - Vec3::new(half, 0.0, half),
        position + Vec3::new(half, config::HITBOX_HEIGHT, half),
    )
}

/// Whether the box at `position` overlaps any solid block.
///
/// A box that only *touches* a block face (its edge exactly on a whole
/// number) doesn't count — `ceil(max) - 1` leaves out the block that starts
/// where the box ends — so standing exactly on the ground isn't overlapping
/// it.
fn overlaps_solid(position: Vec3, is_solid: &impl Fn(IVec3) -> bool) -> bool {
    let (min, max) = bounds(position);
    let low = min.floor().as_ivec3();
    let high = max.ceil().as_ivec3() - IVec3::ONE;
    for y in low.y..=high.y {
        for z in low.z..=high.z {
            for x in low.x..=high.x {
                if is_solid(IVec3::new(x, y, z)) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF_WIDTH: f32 = config::HITBOX_WIDTH / 2.0;

    /// Solid everywhere below `y = 0`, so the ground's surface is at `0`.
    fn ground(block: IVec3) -> bool {
        block.y < 0
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn moving_through_open_air_goes_the_whole_way() {
        let start = Vec3::new(0.5, 5.0, 0.5);
        let delta = Vec3::new(1.0, -1.0, 0.5);
        let moved = move_and_collide(start, delta, ground);
        assert!(moved.position.abs_diff_eq(start + delta, 1e-5));
        assert_eq!(moved.blocked, BVec3::FALSE);
    }

    #[test]
    fn falling_lands_flush_on_the_ground() {
        let moved = move_and_collide(Vec3::new(0.5, 1.0, 0.5), Vec3::new(0.0, -3.0, 0.0), ground);
        assert!(
            close(moved.position.y, EPSILON),
            "landed at {}",
            moved.position.y
        );
        assert!(moved.blocked.y);
    }

    #[test]
    fn a_long_fall_does_not_tunnel_through_a_one_block_floor() {
        let floor = |block: IVec3| block.y == 0;
        let moved = move_and_collide(Vec3::new(0.5, 10.0, 0.5), Vec3::new(0.0, -20.0, 0.0), floor);
        assert!(
            close(moved.position.y, 1.0 + EPSILON),
            "landed at {}",
            moved.position.y
        );
    }

    #[test]
    fn walking_into_a_wall_stops_flush_against_it() {
        let wall = |block: IVec3| block.y < 0 || block.x >= 3;
        let moved = move_and_collide(Vec3::new(1.0, 0.0, 0.5), Vec3::new(5.0, 0.0, 0.0), wall);
        assert!(close(moved.position.x, 3.0 - HALF_WIDTH - EPSILON));
        assert!(moved.blocked.x);
    }

    #[test]
    fn a_wall_only_stops_the_axis_that_hits_it() {
        let wall = |block: IVec3| block.y < 0 || block.x >= 3;
        let start = Vec3::new(1.0, 0.0, 0.5);
        let moved = move_and_collide(start, Vec3::new(5.0, 0.0, 2.0), wall);
        assert!(moved.blocked.x);
        assert!(!moved.blocked.z);
        assert!(
            close(moved.position.z, start.z + 2.0),
            "didn't slide along the wall"
        );
    }

    #[test]
    fn a_ceiling_stops_a_jump_with_the_head_just_under_it() {
        let tunnel = |block: IVec3| block.y < 0 || block.y >= 3;
        let moved = move_and_collide(Vec3::new(0.5, 0.0, 0.5), Vec3::new(0.0, 3.0, 0.0), tunnel);
        assert!(close(
            moved.position.y,
            3.0 - config::HITBOX_HEIGHT - EPSILON
        ));
        assert!(moved.blocked.y);
    }

    #[test]
    fn starting_inside_solid_ground_moves_nothing() {
        let everything = |_: IVec3| true;
        let start = Vec3::new(0.5, 0.5, 0.5);
        let moved = move_and_collide(start, Vec3::new(1.0, -1.0, 1.0), everything);
        assert!(moved.stuck);
        assert_eq!(moved.position, start);
    }

    #[test]
    fn standing_exactly_on_a_block_is_not_overlapping_it() {
        assert!(!overlaps_solid(Vec3::new(0.5, 0.0, 0.5), &ground));
    }
}
