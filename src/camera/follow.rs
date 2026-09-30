//! The third-person view: keeping the world camera behind the player.
//!
//! The camera orbits a pivot at the player's head height. Yaw and pitch both
//! come from the player's [`LookAngles`], so looking around swings the camera
//! around the player rather than turning it in place. The camera always looks
//! straight at the pivot, which keeps the player centred in the view.
//!
//! No collision yet: the camera can end up inside terrain when the player
//! backs up against a hill. See `docs/AUDIT.md`.

use bevy::prelude::*;

use super::WorldCamera;
// Aliased: inside `crate::camera`, a bare `camera::` would read as this
// module rather than `crate::config::camera`.
use crate::config::camera as config;
use crate::player::{LookAngles, Player};

/// Moves the world camera to its place behind the player.
///
/// Skipped outright on any frame the player neither moved nor turned, which
/// covers standing still and the whole of `Paused`: change detection on the
/// player's transform and look angles, checked before the camera is touched.
pub(super) fn follow_player(
    player: Query<(Ref<Transform>, Ref<LookAngles>), With<Player>>,
    // `Without<Player>` proves to Bevy the two queries never alias the same
    // `Transform`; without it the system fails to build.
    mut camera: Query<&mut Transform, (With<WorldCamera>, Without<Player>)>,
) {
    let Ok((player_transform, angles)) = player.single() else {
        return;
    };
    if !player_transform.is_changed() && !angles.is_changed() {
        return;
    }
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };

    *camera_transform = third_person_transform(player_transform.translation, &angles);
}

/// Where the camera goes for a player at `player_position` looking along
/// `angles`: [`config::THIRD_PERSON_DISTANCE`] back from a pivot at
/// [`config::THIRD_PERSON_PIVOT_HEIGHT`] above the player's feet, facing the
/// pivot.
///
/// "Back" is along the view's own +Z (Bevy's forward is −Z), so pitching the
/// view down swings the camera up and over the player, and pitching up
/// swings it down behind them.
fn third_person_transform(player_position: Vec3, angles: &LookAngles) -> Transform {
    let rotation = Quat::from_euler(EulerRot::YXZ, angles.yaw, angles.pitch, 0.0);
    let pivot = player_position + Vec3::Y * config::THIRD_PERSON_PIVOT_HEIGHT;
    let offset = rotation * Vec3::Z * config::THIRD_PERSON_DISTANCE;

    Transform::from_translation(pivot + offset).with_rotation(rotation)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    fn pivot_of(player_position: Vec3) -> Vec3 {
        player_position + Vec3::Y * config::THIRD_PERSON_PIVOT_HEIGHT
    }

    #[test]
    fn looking_straight_ahead_puts_the_camera_directly_behind() {
        let player = Vec3::new(3.0, 10.0, -2.0);
        let camera = third_person_transform(player, &LookAngles::default());

        // Facing −Z, so "behind" is +Z, at pivot height.
        let expected = pivot_of(player) + Vec3::Z * config::THIRD_PERSON_DISTANCE;
        assert!(camera.translation.abs_diff_eq(expected, EPSILON));
        assert!(camera.forward().abs_diff_eq(Vec3::NEG_Z, EPSILON));
    }

    #[test]
    fn the_camera_is_always_the_configured_distance_from_the_pivot() {
        let player = Vec3::new(-7.0, 4.0, 11.0);
        for (yaw, pitch) in [(0.0, 0.0), (1.2, -0.6), (-2.5, 0.9), (4.0, -1.4)] {
            let camera = third_person_transform(player, &LookAngles { yaw, pitch });
            let distance = camera.translation.distance(pivot_of(player));
            assert!(
                (distance - config::THIRD_PERSON_DISTANCE).abs() < EPSILON,
                "yaw {yaw}, pitch {pitch}: camera {distance} from the pivot"
            );
        }
    }

    #[test]
    fn the_camera_always_looks_at_the_pivot() {
        let player = Vec3::new(5.0, 0.0, 5.0);
        for (yaw, pitch) in [(0.3, -0.4), (-1.7, 0.2), (3.0, -1.1)] {
            let camera = third_person_transform(player, &LookAngles { yaw, pitch });
            let toward_pivot = (pivot_of(player) - camera.translation).normalize();
            assert!(
                camera.forward().abs_diff_eq(toward_pivot, EPSILON),
                "yaw {yaw}, pitch {pitch}: camera doesn't face the player"
            );
        }
    }

    #[test]
    fn looking_down_raises_the_camera_above_the_pivot() {
        let player = Vec3::ZERO;
        let camera = third_person_transform(
            player,
            &LookAngles {
                yaw: 0.0,
                pitch: -0.5,
            },
        );
        assert!(camera.translation.y > pivot_of(player).y);
    }
}
