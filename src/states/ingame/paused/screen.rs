//! The pause screen.
//!
//! A translucent overlay rather than an opaque screen: the world is still
//! loaded underneath, and showing it is what distinguishes pausing from
//! leaving to the menu.

use bevy::prelude::*;

/// Dark wash over the world. The alpha is what makes the world readable
/// behind it while still dimming it enough for the text to carry.
///
/// Earlier values (0.75, then 0.45 and 0.3) all looked solid black. That was
/// never the alpha: the UI camera's texture was not being cleared, so the
/// overlay piled up on itself frame after frame until it was opaque (see
/// `camera::camera_ui` and `docs/IMPROVEMENTS.md`). With that fixed, the
/// alpha means what it says.
const OVERLAY: Color = Color::srgba(0.05, 0.03, 0.08, 0.5);
const LABEL: &str = "Paused";

/// Marks this screen's entities so they can be cleared on exit.
#[derive(Component)]
pub(crate) struct PausedScreen;

/// Spawns the screen when the state is entered.
pub(crate) fn spawn(mut commands: Commands) {
    commands.spawn((
        PausedScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(OVERLAY),
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
pub(crate) fn despawn(mut commands: Commands, screens: Query<Entity, With<PausedScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}
