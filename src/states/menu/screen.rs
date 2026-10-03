//! The main menu screen.
//!
//! Placeholder background and title. The one real piece of UI is the Play
//! button, which is the game's first and only way into [`GameState::InGame`]
//! that isn't a debug shortcut.

use bevy::prelude::*;

use crate::states::GameState;

/// Full-screen menu backdrop, stretched to cover the window.
const BACKGROUND_IMAGE: &str = "textures/ui/menu_background.png";
const LABEL: &str = "Menu";

const BUTTON_IDLE: Color = Color::srgb(0.20, 0.24, 0.40);
const BUTTON_HOVERED: Color = Color::srgb(0.28, 0.34, 0.56);
const BUTTON_PRESSED: Color = Color::srgb(0.16, 0.42, 0.24);

/// Marks this screen's entities so they can be cleared on exit.
#[derive(Component)]
pub(crate) struct MenuScreen;

/// The button that starts a new world.
#[derive(Component)]
pub(crate) struct PlayButton;

/// The Play button, on a frame its [`Interaction`] changed.
type PlayButtonChanged = (Changed<Interaction>, With<PlayButton>);

/// Spawns the screen when the state is entered.
pub(crate) fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        MenuScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(24.0),
            ..default()
        },
        ImageNode {
            image: asset_server.load(BACKGROUND_IMAGE),
            // Fill the node without preserving the source aspect ratio, so
            // the image always covers the window exactly.
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
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
                PlayButton,
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
                    Text::new("Play"),
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
pub(crate) fn despawn(mut commands: Commands, screens: Query<Entity, With<MenuScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}

/// Colours the Play button for its current [`Interaction`], and enters
/// [`GameState::InGame`] the frame it's clicked.
///
/// `Changed<Interaction>` so this costs nothing on the far more common frame
/// where the mouse isn't touching the button at all.
pub(crate) fn handle_play_button(
    mut button: Query<(&Interaction, &mut BackgroundColor), PlayButtonChanged>,
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
        next.set(GameState::InGame);
    }
}
