use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    Loading,
    #[default]
    Menu,
    Playing,
}

/// Only exists while `GameState::Playing` — pressing Escape outside of Playing
/// has nothing to toggle, which is what makes Paused reachable only from Playing.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::Playing)]
pub enum PauseState {
    #[default]
    Unpaused,
    Paused,
}
