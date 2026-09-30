//! Movement: turning held keys into what the player wants to do.
//!
//! This reads *intent* only — which way to walk or fly, and whether to jump —
//! and writes it into the player's [`MovementIntent`]. What that intent
//! actually does (gravity, landing, walls) is [`crate::player`]'s physics,
//! which runs right after.
//!
//! Double-taps carry most of the special moves, all timed by
//! [`controls::DOUBLE_TAP_WINDOW`]:
//!
//! - **Movement key**: sprint, when [`controls::SPRINT_MODE`] is `DoubleTap`.
//! - **Space, on foot in Creative**: take off.
//! - **Space, flying**: rise at double speed for as long as it stays held.
//! - **Left Shift, flying**: land — flight stops and gravity takes over.
//!
//! Flight only exists in Creative; in Survival the double-taps of Space and
//! Shift do nothing beyond an ordinary jump.
//!
//! A `DoubleTap` sprint also cancels itself the frame after hitting a wall
//! ([`crate::player::Motion::horizontal_collision`],
//! [`player_config::RESET_SPRINT_ON_COLLISION`]) — running into a block and
//! jumping over it, still holding the movement key the whole time, otherwise
//! keeps the old sprint engaged, since it never got the usual chance to
//! disengage (every movement key released).

use bevy::prelude::*;

// Aliased: inside `crate::input`, a bare `input::` would read as this module.
use crate::config::input as controls;
use crate::config::player as player_config;
use crate::config::player::GameMode;
use crate::player::{ActiveGameMode, Flying, Motion, MovementIntent, Player};

/// Double-tapping any of these keys engages sprint.
///
/// Only consulted when [`controls::SPRINT_MODE`] is
/// [`controls::SprintMode::DoubleTap`].
const MOVEMENT_KEYS: [KeyCode; 4] = [
    controls::FORWARD,
    controls::BACKWARD,
    controls::LEFT,
    controls::RIGHT,
];

/// Every key this control reads. Double-taps are tracked across all of them.
const TRACKED_KEYS: [KeyCode; 6] = [
    controls::FORWARD,
    controls::BACKWARD,
    controls::LEFT,
    controls::RIGHT,
    controls::UP,
    controls::DOWN,
];

/// What the controls remember between frames.
#[derive(Default)]
pub(super) struct Gestures {
    /// The most recent tap of a tracked key, for spotting double-taps.
    last_tap: Option<(KeyCode, f32)>,
    sprinting: bool,
    /// Rising at double speed after a double-tap of Space while flying.
    fast_ascent: bool,
    /// Whether the last intent written asked for anything, so a frame with
    /// no keys held knows whether there's still an old intent to clear.
    moving: bool,
}

/// Reads the movement keys into the player's [`MovementIntent`], and switches
/// flight on and off (Creative only).
pub(super) fn read_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mode: Res<ActiveGameMode>,
    mut player: Query<(&Transform, &mut MovementIntent, &mut Flying, &Motion), With<Player>>,
    mut gestures: Local<Gestures>,
) {
    // Standing still is the common case. It should cost six key lookups and
    // nothing more — no query, no vector maths (which, at opt-level 0, runs
    // unoptimised: glam's functions are `#[inline]` into this crate). Only
    // the first idle frame goes further, to clear the intent it replaces.
    if TRACKED_KEYS.iter().all(|&key| !keys.pressed(key)) {
        gestures.sprinting = false;
        gestures.fast_ascent = false;
        if !gestures.moving {
            return;
        }
    }

    let Ok((transform, mut intent, mut flying, motion)) = player.single_mut() else {
        return;
    };

    cancel_sprint_on_collision(motion.horizontal_collision(), &mut gestures);

    let double_tap = register_taps(&keys, time.elapsed_secs(), &mut gestures.last_tap);
    update_gestures(&keys, double_tap, mode.0, &mut flying, &mut gestures);
    *intent = movement_intent(&keys, transform, flying.0, &gestures);
    gestures.moving = *intent != MovementIntent::default();
}

/// Records every tracked key pressed this frame, and returns the one whose
/// press completed a double-tap, if any.
///
/// A completed double-tap clears the record, so a third tap starts a new
/// pair rather than counting as a second double-tap with the tap before it —
/// tapping Space three times takes off once, not "take off, then fast
/// ascent".
fn register_taps(
    keys: &ButtonInput<KeyCode>,
    now: f32,
    last_tap: &mut Option<(KeyCode, f32)>,
) -> Option<KeyCode> {
    let mut double_tap = None;
    for key in TRACKED_KEYS {
        if !keys.just_pressed(key) {
            continue;
        }
        if is_double_tap(key, now, *last_tap) {
            double_tap = Some(key);
            *last_tap = None;
        } else {
            *last_tap = Some((key, now));
        }
    }
    double_tap
}

/// Cancels a sticky `DoubleTap` sprint the frame after `collided` — a wall
/// hit on the last physics step, from
/// [`crate::player::Motion::horizontal_collision`]. A wall never releases
/// the movement key that's held into it, so sprint would otherwise never get
/// the usual chance to disengage (every movement key released) and would
/// still be running once the player jumps clear.
///
/// Gated on [`player_config::RESET_SPRINT_ON_COLLISION`], and harmless
/// either way in `Hold` mode: `update_gestures` overwrites `sprinting`
/// unconditionally there, from the hold key's state each frame.
fn cancel_sprint_on_collision(collided: bool, gestures: &mut Gestures) {
    if collided && player_config::RESET_SPRINT_ON_COLLISION {
        gestures.sprinting = false;
    }
}

/// Updates sprint, flight, and fast ascent from this frame's keys and
/// double-tap.
fn update_gestures(
    keys: &ButtonInput<KeyCode>,
    double_tap: Option<KeyCode>,
    mode: GameMode,
    flying: &mut Flying,
    gestures: &mut Gestures,
) {
    update_sprint(
        keys,
        controls::SPRINT_MODE,
        double_tap,
        &mut gestures.sprinting,
    );

    if mode == GameMode::Creative {
        if double_tap == Some(controls::UP) {
            if flying.0 {
                gestures.fast_ascent = true;
            } else {
                flying.0 = true;
                info!("player: flying");
            }
        } else if double_tap == Some(controls::DOWN) && flying.0 {
            flying.0 = false;
            info!("player: stopped flying");
        }
    }
    if !flying.0 || !keys.pressed(controls::UP) {
        gestures.fast_ascent = false;
    }
}

/// Updates `sprinting` for one `mode`, from this frame's keys and whichever
/// key (if any) just completed a double-tap.
///
/// Takes `mode` as a parameter rather than reading [`controls::SPRINT_MODE`]
/// directly, so tests can exercise both [`controls::SprintMode`] variants
/// regardless of which one the constant currently selects — only
/// `update_gestures`, the caller, needs to change to switch which mode is
/// actually live.
fn update_sprint(
    keys: &ButtonInput<KeyCode>,
    mode: controls::SprintMode,
    double_tap: Option<KeyCode>,
    sprinting: &mut bool,
) {
    match mode {
        controls::SprintMode::Hold => *sprinting = keys.pressed(controls::SPRINT_HOLD_KEY),
        // Engages on a double-tap and stays engaged, the way Minecraft's
        // sprint toggle works, until every movement key is released.
        controls::SprintMode::DoubleTap => {
            if double_tap.is_some_and(|key| MOVEMENT_KEYS.contains(&key)) {
                *sprinting = true;
            }
            if !MOVEMENT_KEYS.iter().any(|&key| keys.pressed(key)) {
                *sprinting = false;
            }
        }
    }
}

/// What the held keys ask the player to do, given which way they face.
///
/// The player's transform only ever rotates about Y (see `input::look`), so
/// its `forward` and `right` are level: walking forward while looking down
/// doesn't aim into the ground.
fn movement_intent(
    keys: &ButtonInput<KeyCode>,
    transform: &Transform,
    flying: bool,
    gestures: &Gestures,
) -> MovementIntent {
    let strafe = axis(keys, controls::RIGHT, controls::LEFT);
    let forward = axis(keys, controls::FORWARD, controls::BACKWARD);
    let sprint = if gestures.sprinting {
        controls::SPRINT_MULTIPLIER
    } else {
        1.0
    };

    if flying {
        let rise = axis(keys, controls::UP, controls::DOWN);
        // `normalize_or_zero`, not `normalize`: normalising a zero vector
        // gives NaN, and a NaN position is unrecoverable.
        let velocity =
            (*transform.right() * strafe + Vec3::Y * rise + *transform.forward() * forward)
                .normalize_or_zero()
                * controls::FLY_SPEED
                * sprint;
        let ascent = if gestures.fast_ascent && velocity.y > 0.0 {
            controls::FAST_ASCENT_MULTIPLIER
        } else {
            1.0
        };
        MovementIntent {
            horizontal: velocity.with_y(0.0),
            vertical: velocity.y * ascent,
            jump: false,
        }
    } else {
        let horizontal = (*transform.right() * strafe + *transform.forward() * forward)
            .normalize_or_zero()
            * controls::WALK_SPEED
            * sprint;
        MovementIntent {
            horizontal,
            vertical: 0.0,
            jump: keys.just_pressed(controls::UP),
        }
    }
}

/// Whether pressing `key` right now counts as a double-tap of the same key,
/// given when it (or another key) was last tapped.
fn is_double_tap(key: KeyCode, now: f32, last_tap: Option<(KeyCode, f32)>) -> bool {
    matches!(
        last_tap,
        Some((last_key, last_time))
            if last_key == key && now - last_time < controls::DOUBLE_TAP_WINDOW
    )
}

/// One movement axis from a pair of opposing keys.
///
/// `1.0` when only `positive` is held, `-1.0` when only `negative` is, and
/// `0.0` when neither or both are — holding both cancels rather than letting
/// one silently win.
fn axis(keys: &ButtonInput<KeyCode>, positive: KeyCode, negative: KeyCode) -> f32 {
    match (keys.pressed(positive), keys.pressed(negative)) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(keys: &[KeyCode]) -> ButtonInput<KeyCode> {
        let mut input = ButtonInput::default();
        for &key in keys {
            input.press(key);
        }
        input
    }

    /// Presses `key` as a fresh tap at time `now` and returns what
    /// `register_taps` made of it.
    fn tap(key: KeyCode, now: f32, last_tap: &mut Option<(KeyCode, f32)>) -> Option<KeyCode> {
        register_taps(&held(&[key]), now, last_tap)
    }

    /// A double-tap of `key` in `mode`, applied to `flying`.
    fn double_tap_in(mode: GameMode, key: KeyCode, flying: bool) -> (Flying, Gestures) {
        let mut flying = Flying(flying);
        let mut gestures = Gestures::default();
        update_gestures(&held(&[key]), Some(key), mode, &mut flying, &mut gestures);
        (flying, gestures)
    }

    #[test]
    fn axis_is_zero_with_no_keys() {
        assert_eq!(axis(&held(&[]), KeyCode::KeyW, KeyCode::KeyS), 0.0);
    }

    #[test]
    fn axis_follows_a_single_key() {
        assert_eq!(
            axis(&held(&[KeyCode::KeyW]), KeyCode::KeyW, KeyCode::KeyS),
            1.0
        );
        assert_eq!(
            axis(&held(&[KeyCode::KeyS]), KeyCode::KeyW, KeyCode::KeyS),
            -1.0
        );
    }

    #[test]
    fn opposing_keys_cancel() {
        let both = held(&[KeyCode::KeyW, KeyCode::KeyS]);
        assert_eq!(axis(&both, KeyCode::KeyW, KeyCode::KeyS), 0.0);
    }

    #[test]
    fn no_previous_tap_is_not_a_double_tap() {
        assert!(!is_double_tap(KeyCode::KeyW, 1.0, None));
    }

    #[test]
    fn same_key_within_the_window_is_a_double_tap() {
        let last = Some((KeyCode::KeyW, 1.0));
        assert!(is_double_tap(
            KeyCode::KeyW,
            1.0 + controls::DOUBLE_TAP_WINDOW - 0.01,
            last
        ));
    }

    #[test]
    fn same_key_outside_the_window_is_not_a_double_tap() {
        let last = Some((KeyCode::KeyW, 1.0));
        assert!(!is_double_tap(
            KeyCode::KeyW,
            1.0 + controls::DOUBLE_TAP_WINDOW + 0.01,
            last
        ));
    }

    #[test]
    fn a_different_key_is_not_a_double_tap() {
        let last = Some((KeyCode::KeyW, 1.0));
        assert!(!is_double_tap(KeyCode::KeyA, 1.05, last));
    }

    #[test]
    fn hold_mode_sprints_only_while_the_key_is_pressed() {
        let mut sprinting = false;
        update_sprint(
            &held(&[controls::SPRINT_HOLD_KEY]),
            controls::SprintMode::Hold,
            None,
            &mut sprinting,
        );
        assert!(sprinting);

        update_sprint(&held(&[]), controls::SprintMode::Hold, None, &mut sprinting);
        assert!(!sprinting);
    }

    #[test]
    fn hold_mode_ignores_a_movement_double_tap() {
        // A double-tap of a movement key is only `DoubleTap` mode's trigger;
        // `Hold` must read only the hold key, whatever else is passed in.
        let mut sprinting = false;
        update_sprint(
            &held(&[]),
            controls::SprintMode::Hold,
            Some(controls::FORWARD),
            &mut sprinting,
        );
        assert!(!sprinting);
    }

    #[test]
    fn double_tap_mode_engages_sprint_on_a_movement_double_tap() {
        let mut sprinting = false;
        update_sprint(
            &held(&[controls::FORWARD]),
            controls::SprintMode::DoubleTap,
            Some(controls::FORWARD),
            &mut sprinting,
        );
        assert!(sprinting);
    }

    #[test]
    fn double_tap_mode_disengages_once_every_movement_key_is_released() {
        let mut sprinting = true;
        update_sprint(
            &held(&[]),
            controls::SprintMode::DoubleTap,
            None,
            &mut sprinting,
        );
        assert!(!sprinting);
    }

    #[test]
    fn double_tap_mode_stays_engaged_while_any_movement_key_is_held() {
        let mut sprinting = true;
        update_sprint(
            &held(&[controls::LEFT]),
            controls::SprintMode::DoubleTap,
            None,
            &mut sprinting,
        );
        assert!(sprinting);
    }

    #[test]
    fn a_wall_collision_cancels_an_engaged_sprint() {
        let mut gestures = Gestures {
            sprinting: true,
            ..default()
        };
        cancel_sprint_on_collision(true, &mut gestures);
        assert!(!gestures.sprinting);
    }

    #[test]
    fn no_collision_leaves_sprint_untouched() {
        let mut gestures = Gestures {
            sprinting: true,
            ..default()
        };
        cancel_sprint_on_collision(false, &mut gestures);
        assert!(gestures.sprinting);
    }

    #[test]
    fn a_second_tap_of_the_same_key_completes_a_double_tap() {
        let mut last_tap = None;
        assert_eq!(tap(controls::UP, 1.0, &mut last_tap), None);
        assert_eq!(tap(controls::UP, 1.1, &mut last_tap), Some(controls::UP));
    }

    #[test]
    fn a_third_tap_starts_a_new_pair() {
        let mut last_tap = None;
        tap(controls::UP, 1.0, &mut last_tap);
        tap(controls::UP, 1.1, &mut last_tap);
        assert_eq!(tap(controls::UP, 1.2, &mut last_tap), None);
    }

    #[test]
    fn double_tapping_up_in_creative_takes_off() {
        let (flying, _) = double_tap_in(GameMode::Creative, controls::UP, false);
        assert!(flying.0);
    }

    #[test]
    fn double_tapping_up_in_survival_never_takes_off() {
        let (flying, _) = double_tap_in(GameMode::Survival, controls::UP, false);
        assert!(!flying.0);
    }

    #[test]
    fn double_tapping_up_while_flying_engages_fast_ascent() {
        let (flying, gestures) = double_tap_in(GameMode::Creative, controls::UP, true);
        assert!(flying.0);
        assert!(gestures.fast_ascent);
    }

    #[test]
    fn double_tapping_down_while_flying_lands() {
        let (flying, _) = double_tap_in(GameMode::Creative, controls::DOWN, true);
        assert!(!flying.0);
    }

    #[test]
    fn on_foot_the_intent_is_level_at_walking_speed() {
        let intent = movement_intent(
            &held(&[controls::FORWARD]),
            &Transform::IDENTITY,
            false,
            &Gestures::default(),
        );
        assert!(
            intent
                .horizontal
                .abs_diff_eq(Vec3::NEG_Z * controls::WALK_SPEED, 1e-5)
        );
        assert_eq!(intent.vertical, 0.0);
    }

    #[test]
    fn pressing_up_on_foot_asks_for_a_jump() {
        let intent = movement_intent(
            &held(&[controls::UP]),
            &Transform::IDENTITY,
            false,
            &Gestures::default(),
        );
        assert!(intent.jump);
    }

    #[test]
    fn fast_ascent_doubles_the_climb() {
        let keys = held(&[controls::UP]);
        let normal = movement_intent(&keys, &Transform::IDENTITY, true, &Gestures::default());
        let fast = movement_intent(
            &keys,
            &Transform::IDENTITY,
            true,
            &Gestures {
                fast_ascent: true,
                ..default()
            },
        );
        assert!((fast.vertical - normal.vertical * controls::FAST_ASCENT_MULTIPLIER).abs() < 1e-5);
    }
}
