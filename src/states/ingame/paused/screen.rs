//! The pause screen.
//!
//! A translucent overlay rather than an opaque screen: the world is still
//! loaded underneath, and showing it is what distinguishes pausing from
//! leaving to the menu. The Exit button leaves the world entirely, back to
//! [`GameState::Menu`] — pausing suspends the world, Exit tears it down.

use bevy::prelude::*;

use crate::states::GameState;

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

const BUTTON_IDLE: Color = Color::srgb(0.40, 0.20, 0.20);
const BUTTON_HOVERED: Color = Color::srgb(0.56, 0.28, 0.28);
const BUTTON_PRESSED: Color = Color::srgb(0.42, 0.16, 0.16);

/// Marks this screen's entities so they can be cleared on exit.
#[derive(Component)]
pub(crate) struct PausedScreen;

/// The button that leaves the world for the menu.
#[derive(Component)]
pub(crate) struct ExitButton;

/// The Exit button, on a frame its [`Interaction`] changed.
type ExitButtonChanged = (Changed<Interaction>, With<ExitButton>);

/// Spawns the screen when the state is entered.
pub(crate) fn spawn(mut commands: Commands) {
    commands.spawn((
        PausedScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(24.0),
            ..default()
        },
        BackgroundColor(OVERLAY),
        children![
            (
                Text::new(LABEL),
                TextFont {
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
            (
                ExitButton,
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(64.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(BUTTON_IDLE),
                children![(
                    Text::new("Exit"),
                    TextFont {
                        font_size: FontSize::Px(28.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                )],
            ),
        ],
    ));
}

/// Despawns the screen when the state is left.
pub(crate) fn despawn(mut commands: Commands, screens: Query<Entity, With<PausedScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}

/// Colours the Exit button for its current [`Interaction`], and leaves for
/// [`GameState::Menu`] the frame it's clicked.
///
/// `Changed<Interaction>` so this costs nothing on the far more common frame
/// where the mouse isn't touching the button at all.
pub(crate) fn handle_exit_button(
    mut button: Query<(&Interaction, &mut BackgroundColor), ExitButtonChanged>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Ok((interaction, mut color)) = button.single_mut() else {
        return;
    };

    *color = BackgroundColor(match interaction {
        Interaction::Pressed => BUTTON_PRESSED,
        Interaction::Hovered => BUTTON_HOVERED,
        Interaction::None => BUTTON_IDLE,
    });

    if *interaction == Interaction::Pressed {
        next.set(GameState::Menu);
    }
}
