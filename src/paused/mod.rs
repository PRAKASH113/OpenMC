use bevy::prelude::*;

use crate::app::states::PauseState;

#[derive(Component)]
struct PausedOverlay;

pub struct PausedPlugin;

impl Plugin for PausedPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<PauseState>()
            .add_systems(OnEnter(PauseState::Paused), setup)
            .add_systems(OnExit(PauseState::Paused), teardown);
    }
}

/// Dims the still-visible Playing scene behind a translucent panel + "Paused" text.
/// No `Camera2d` here on purpose: the scene's own `Camera3d` stays alive and active
/// while paused (that's the point — nothing gets torn down), and Bevy UI renders on
/// top of whichever camera is present regardless of whether it's 2D or 3D.
fn setup(mut commands: Commands) {
    commands.spawn((
        PausedOverlay,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        children![(
            Text::new("Paused"),
            TextFont {
                font_size: FontSize::Px(50.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}

fn teardown(mut commands: Commands, overlays: Query<Entity, With<PausedOverlay>>) {
    for entity in &overlays {
        commands.entity(entity).despawn();
    }
}
