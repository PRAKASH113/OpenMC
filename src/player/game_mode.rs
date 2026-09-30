//! Game modes: which set of rules the player is playing under.
//!
//! The modes themselves ([`GameMode`]) and the starting one live in
//! `config::player`. This module holds which one is active, switches it on
//! [`crate::config::input::TOGGLE_GAME_MODE`], and offers [`in_creative`] as a
//! run condition for anything only allowed in Creative — currently every
//! testing-tool key.

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

use super::physics::CollisionEnabled;
use super::{Flying, Player};
use crate::config::input;
use crate::config::player::{self as config, GameMode};
use crate::states::InGameState;

/// The game mode currently in effect.
///
/// A resource rather than a component on the player: it outlives any one
/// visit to a world, and systems that don't otherwise touch the player (the
/// testing-tool keys) need to read it as a run condition.
#[derive(Resource)]
pub(crate) struct ActiveGameMode(pub GameMode);

impl Default for ActiveGameMode {
    fn default() -> Self {
        Self(config::INITIAL_GAME_MODE)
    }
}

/// Run condition: whether the game is in [`GameMode::Creative`].
pub(crate) fn in_creative(mode: Res<ActiveGameMode>) -> bool {
    mode.0 == GameMode::Creative
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<ActiveGameMode>().add_systems(
        Update,
        toggle_game_mode.run_if(
            input_just_pressed(input::TOGGLE_GAME_MODE).and_then(in_state(InGameState::Playing)),
        ),
    );
}

/// Whether the player should be flying the moment they enter `mode`.
pub(super) fn starts_flying(mode: GameMode) -> bool {
    mode == GameMode::Creative && config::CREATIVE_STARTS_FLYING
}

/// Switches to the other mode, puts the player's flight into the state the
/// new mode starts with — never flying in Survival, and whatever
/// `CREATIVE_STARTS_FLYING` says in Creative — and re-enables collision.
///
/// Collision always comes back on here, regardless of
/// [`crate::config::debug::COLLISION_INITIALLY_DISABLED`] or whatever the
/// hotkey last left it at: leaving Creative with it switched off must never
/// strand the player noclipping through Survival, and re-entering Creative
/// should start from a clean, collidable state rather than remembering a
/// previous session's testing setup.
fn toggle_game_mode(
    mut mode: ResMut<ActiveGameMode>,
    mut collision: ResMut<CollisionEnabled>,
    mut player: Query<&mut Flying, With<Player>>,
) {
    mode.0 = match mode.0 {
        GameMode::Survival => GameMode::Creative,
        GameMode::Creative => GameMode::Survival,
    };
    info!("player: game mode -> {:?}", mode.0);

    collision.0 = true;
    if let Ok(mut flying) = player.single_mut() {
        flying.0 = starts_flying(mode.0);
    }
}
