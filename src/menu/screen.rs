//! The main menu screen.
//!
//! Placeholder: a flat colour with the state's name on it, so transitions
//! are visible. Replaced by the real menu (title, buttons) later.

use bevy::prelude::*;

const BACKGROUND: Color = Color::srgb(0.10, 0.14, 0.28);
const LABEL: &str = "Menu";

/// Marks this screen's entities so they can be cleared on exit.
#[derive(Component)]
pub(crate) struct MenuScreen;

/// Spawns the screen when the state is entered.
pub(crate) fn spawn(mut commands: Commands) {
    commands.spawn((
        MenuScreen,
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
pub(crate) fn despawn(mut commands: Commands, screens: Query<Entity, With<MenuScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}
