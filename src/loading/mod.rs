//! The loading state: assets are being prepared, nothing is interactive.
//!
//! Everything this state owns lives in `loading/`.

pub mod screen;

use bevy::prelude::*;

use crate::app::GameState;

/// Owns what the game shows and runs while loading.
pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), screen::spawn)
            .add_systems(OnExit(GameState::Loading), screen::despawn);
    }
}
