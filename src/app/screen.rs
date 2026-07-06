use bevy::prelude::*;

/// Spawns a `Camera2d` plus a fullscreen colored `Node` with centered text, both
/// tagged with `marker` so the caller's `OnExit` teardown system can despawn them again.
pub fn spawn_fullscreen_text_screen(
    commands: &mut Commands,
    marker: impl Component + Clone,
    color: Color,
    text: &str,
) {
    commands.spawn((marker.clone(), Camera2d));
    commands.spawn((
        marker,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(color),
        children![(
            Text::new(text.to_string()),
            TextFont {
                font_size: FontSize::Px(50.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}
