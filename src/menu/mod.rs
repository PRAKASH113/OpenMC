//! The main menu state.
//!
//! Everything this state owns lives in `menu/`.

pub mod screen;

use bevy::prelude::*;

use crate::app::GameState;

/// Owns what the game shows and runs in the menu.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), screen::spawn)
            .add_systems(OnExit(GameState::Menu), screen::despawn);
    }
}
