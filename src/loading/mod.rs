use bevy::prelude::*;

use crate::app::screen::spawn_fullscreen_text_screen;
use crate::app::states::GameState;

#[derive(Component, Clone, Copy)]
struct LoadingScreen;

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), setup)
            .add_systems(OnExit(GameState::Loading), teardown);
    }
}

fn setup(mut commands: Commands) {
    spawn_fullscreen_text_screen(
        &mut commands,
        LoadingScreen,
        Color::srgb(0.1, 0.1, 0.1),
        "Loading...",
    );
}

fn teardown(mut commands: Commands, screens: Query<Entity, With<LoadingScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}
