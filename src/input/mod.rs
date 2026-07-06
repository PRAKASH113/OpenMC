mod playing;

use bevy::prelude::*;

use crate::app::states::{GameState, PauseState};
use crate::config::controls::KeyBinds;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyBinds>()
            .add_systems(Update, switch_state)
            .add_systems(
                Update,
                (
                    playing::movement_and_look.run_if(
                        in_state(GameState::Playing).and_then(in_state(PauseState::Unpaused)),
                    ),
                    playing::toggle_pause.run_if(in_state(GameState::Playing)),
                    playing::toggle_terrain_wireframe.run_if(in_state(GameState::Playing)),
                ),
            )
            .add_systems(OnEnter(GameState::Playing), playing::lock_cursor)
            .add_systems(OnExit(GameState::Playing), playing::release_cursor)
            .add_systems(OnEnter(PauseState::Paused), playing::release_cursor)
            .add_systems(OnEnter(PauseState::Unpaused), playing::lock_cursor);
    }
}

fn switch_state(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    let shift_held = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    // Shift+1 is reserved for `playing::toggle_terrain_wireframe`.
    if keys.just_pressed(KeyCode::Digit1) && !shift_held {
        next_state.set(GameState::Loading);
    } else if keys.just_pressed(KeyCode::Digit2) {
        next_state.set(GameState::Menu);
    } else if keys.just_pressed(KeyCode::Digit3) {
        next_state.set(GameState::Playing);
    }
}
