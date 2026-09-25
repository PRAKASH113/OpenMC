//! The boot loading state: preparing what the game needs before the menu.
//!
//! This is *solid* loading — a full screen, nothing else running — and it is
//! only the startup kind. Two related things deliberately do not live here:
//!
//! - **World generation** belongs to `InGame`, as its first sub-state, since
//!   the world must exist while it is being built.
//! - **Soft loading** (a brief overlay over whatever state is current) is
//!   not a state at all, because entering a state would tear down the screen
//!   it is meant to sit on top of.
//!
//! Both are designed in `docs/ARCHITECTURE.md` and built when first needed.

mod screen;

use bevy::prelude::*;

use crate::states::GameState;

/// Owns the loading screen, and moves on to the menu once loading is done.
pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), screen::spawn)
            .add_systems(OnExit(GameState::Loading), screen::despawn)
            .add_systems(
                Update,
                // Once real boot assets exist, this gains a second condition
                // — `.and_then(boot_assets_loaded)` — and the loading screen
                // stays up until they arrive. The system itself won't change.
                finish.run_if(in_state(GameState::Loading)),
            );
    }
}

/// Leaves loading for the menu.
///
/// There is nothing to load yet, so this fires on the first frame. It still
/// matters: without it, a release build — which has no debug keys — would
/// sit on the loading screen forever.
fn finish(mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::Menu);
}
