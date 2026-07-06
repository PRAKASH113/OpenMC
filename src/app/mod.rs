pub mod screen;
pub mod states;

use bevy::prelude::*;

use crate::dev_tools;
use crate::loading;
use crate::menu;
use crate::paused;
use crate::playing;
use states::GameState;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>().add_plugins((
            loading::LoadingPlugin,
            menu::MenuPlugin,
            playing::PlayingPlugin,
            paused::PausedPlugin,
            dev_tools::DevToolsPlugin,
        ));
    }
}
