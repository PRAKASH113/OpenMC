use bevy::prelude::*;

use crate::app::screen::spawn_fullscreen_text_screen;
use crate::app::states::GameState;

#[derive(Component, Clone, Copy)]
struct MenuScreen;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), setup)
            .add_systems(OnExit(GameState::Menu), teardown);
    }
}

fn setup(mut commands: Commands) {
    spawn_fullscreen_text_screen(&mut commands, MenuScreen, Color::BLACK, "Start Play");
}

fn teardown(mut commands: Commands, screens: Query<Entity, With<MenuScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}
