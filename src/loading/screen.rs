//! The loading screen.
//!
//! Placeholder: a flat colour with the state's name on it, so transitions
//! are visible. Replaced by the real loading UI (progress, logo) later.

use bevy::prelude::*;

const BACKGROUND: Color = Color::srgb(0.08, 0.08, 0.10);
const LABEL: &str = "Loading";

/// Marks this screen's entities so they can be cleared on exit.
#[derive(Component)]
pub(crate) struct LoadingScreen;

/// Spawns the screen when the state is entered.
pub(crate) fn spawn(mut commands: Commands) {
    commands.spawn((
        LoadingScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(BACKGROUND),
        children![(
            Text::new(LABEL),
            TextFont {
                font_size: FontSize::Px(64.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}

/// Despawns the screen when the state is left.
pub(crate) fn despawn(mut commands: Commands, screens: Query<Entity, With<LoadingScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}
