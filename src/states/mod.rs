//! The game's states: the state machine, and one folder per state.
//!
//! [`GameState`] is where the player is. [`InGameState`] is what is
//! happening inside a loaded world, and only exists while [`GameState`] is
//! [`GameState::InGame`] — so "paused with no world loaded" is not a state
//! the program can represent.
//!
//! The folder layout mirrors that hierarchy. Each top-level state has a
//! folder here, and a sub-state lives *inside* its parent's folder — which
//! is why `paused/` is in `ingame/`. What a state shows and runs lives in
//! its own folder, so adding behaviour to a state never means editing this
//! file.

mod ingame;
mod loading;
mod menu;

use bevy::prelude::*;

use ingame::InGamePlugin;
use loading::LoadingPlugin;
use menu::MenuPlugin;

/// Where the player is.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Solid loading at startup: preparing what the game needs before the
    /// menu can appear. Moves on to [`GameState::Menu`] by itself once done.
    ///
    /// This is *boot* loading only. Generating a world belongs inside
    /// [`GameState::InGame`] as its first sub-state, because the world must
    /// exist while it is being built — see `docs/ARCHITECTURE.md`.
    #[default]
    Loading,
    /// Main menu.
    Menu,
    /// A world is loaded.
    InGame,
}

/// What is happening inside a loaded world.
///
/// A sub-state of [`GameState::InGame`]: Bevy only creates it on entering
/// that state and removes it on leaving, so pausing cannot be reached from
/// the menu or the loading screen. That constraint is structural, not a
/// runtime check — there is no code path to write wrongly.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::InGame)]
pub enum InGameState {
    /// The world is live and accepting input.
    #[default]
    Playing,
    /// Suspended. The world stays loaded; its systems stop.
    Paused,
}

/// Installs the state machine and every state.
///
/// Each state folder registers its own children in turn — `ingame` registers
/// `paused` — so this lists only the top level.
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        // The machine first: every state plugin below registers systems
        // against these states' schedules.
        app.init_state::<GameState>()
            .add_sub_state::<InGameState>()
            .add_systems(
                Update,
                (
                    log_state_change::<GameState>,
                    log_state_change::<InGameState>,
                ),
            );

        app.add_plugins((LoadingPlugin, MenuPlugin, InGamePlugin));

        // Debug-only, so a shipped build cannot teleport between states on
        // a keypress.
        #[cfg(debug_assertions)]
        app.add_systems(Update, debug_jump_to_state);
    }
}

/// Logs every change of state `S`.
///
/// Generic so both the top-level state and the in-game sub-state report
/// through one implementation. Identity transitions are skipped —
/// re-entering the state you are already in is not a change worth a line.
fn log_state_change<S: States>(mut transitions: MessageReader<StateTransitionEvent<S>>) {
    for transition in transitions.read() {
        if transition.exited == transition.entered {
            continue;
        }

        match (&transition.exited, &transition.entered) {
            (Some(from), Some(to)) => info!("state: {from:?} -> {to:?}"),
            (None, Some(to)) => info!("state: entering {to:?}"),
            (Some(from), None) => info!("state: leaving {from:?}"),
            (None, None) => {}
        }
    }
}

/// Temporary debug shortcuts: each key jumps straight to a state so every
/// screen can be checked before the states drive themselves.
///
/// [`InGameState::Paused`] is deliberately absent — it is only reachable by
/// pausing while in a world, which is the whole point of it being a
/// sub-state.
///
/// Jumping to [`GameState::Loading`] bounces straight back to the menu,
/// because loading currently has nothing to wait for. That is still useful:
/// it exercises the `Loading -> Menu` exit.
///
/// Delete this table and [`debug_jump_to_state`] once real transitions exist.
#[cfg(debug_assertions)]
const DEBUG_JUMPS: [(KeyCode, GameState); 3] = [
    (crate::config::input::DEBUG_GOTO_LOADING, GameState::Loading),
    (crate::config::input::DEBUG_GOTO_MENU, GameState::Menu),
    (crate::config::input::DEBUG_GOTO_INGAME, GameState::InGame),
];

/// Applies the [`DEBUG_JUMPS`] shortcuts. Debug builds only.
#[cfg(debug_assertions)]
fn debug_jump_to_state(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<GameState>>) {
    for (key, state) in DEBUG_JUMPS {
        if keys.just_pressed(key) {
            next.set(state);
            return;
        }
    }
}
