//! The composition root: every part of the game, registered in one place.

use bevy::prelude::*;
#[cfg(debug_assertions)]
use bevy::render::RenderPlugin as BevyRenderPlugin;
#[cfg(debug_assertions)]
use bevy::render::settings::{RenderCreation, WgpuFeatures, WgpuSettings};

use crate::camera::CameraPlugin;
use crate::input::GameInputPlugin;
use crate::player::PlayerPlugin;
use crate::render::RenderPlugin;
use crate::states::GameStatePlugin;
use crate::utils::log::log_plugin;
use crate::window::{WindowControlPlugin, primary_window_plugin};
use crate::world::WorldPlugin;

/// Assembles the whole game.
///
/// Adding a domain to the game means adding its plugin here and nowhere
/// else. This includes Bevy's own plugins — enabling, configuring, or
/// disabling one of them is as much a decision about what the app is made of
/// as adding one of ours, so it belongs in this one list too.
pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // The engine must come first. It installs `StatesPlugin`, which
        // `GameStatePlugin` needs in place before it calls `init_state` —
        // registering them the other way round is not a compile error, just a
        // runtime warning and a state machine that never transitions.
        let default_plugins = DefaultPlugins
            .set(primary_window_plugin())
            .set(log_plugin())
            // Gilrs: the gamepad backend. There is no controller support,
            // and left enabled it polls the OS for gamepad events every
            // frame. Disabled at runtime rather than dropped from
            // Cargo.toml's `bevy` features — unlike audio (below), Bevy
            // doesn't gate it behind a feature at all, so a Cargo-level
            // removal isn't available. Re-enable when controller support
            // is built. It's a leaf plugin — nothing else in Bevy depends
            // on it, so disabling it cannot break startup.
            //
            // Audio used to be disabled here the same way. It no longer
            // needs to be: the `audio` Cargo feature was dropped instead
            // (see Cargo.toml), which removes `bevy_audio` from the
            // dependency tree entirely rather than just skipping the
            // plugin at runtime — the stronger version of the same
            // decision. `bevy::audio::AudioPlugin` no longer exists to
            // reference, which is exactly what broke this line when the
            // feature was dropped without updating it in the same pass.
            .disable::<bevy::gilrs::GilrsPlugin>();

        // Debug builds only: request the two GPU features `render::debug`'s
        // wireframe toggle (`F8`) needs. Requesting them here, once, is the
        // only way to get them — `WireframePlugin` itself can only check
        // afterwards whether the device ended up with them, not ask for
        // them itself. If the adapter doesn't support them, Bevy warns and
        // the plugin quietly does nothing rather than failing to start; see
        // `render::debug`'s module doc for what to check if wireframes never
        // appear. Requested unconditionally in every debug build regardless
        // of `config::debug::TESTING_TOOLS_ENABLED`, since that's a
        // runtime "is this in use right now" switch, not something GPU
        // capability negotiation at startup should depend on.
        #[cfg(debug_assertions)]
        let default_plugins = default_plugins.set(BevyRenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                features: WgpuFeatures::POLYGON_MODE_LINE | WgpuFeatures::IMMEDIATES,
                ..default()
            })),
            ..default()
        });

        app.add_plugins(default_plugins);

        // Our domains. `GameStatePlugin` brings in every state itself, so
        // adding a state never touches this file — only `states/mod.rs`.
        // `WorldPlugin` before `RenderPlugin`: `render/` depends on `world/`
        // and never the reverse, so this list reads in the same direction.
        app.add_plugins((
            GameStatePlugin,
            PlayerPlugin,
            CameraPlugin,
            GameInputPlugin,
            WindowControlPlugin,
            WorldPlugin,
            RenderPlugin,
        ));

        // Bevy's own wireframe rendering — debug builds only, alongside the
        // GPU features it needs above. `render::debug` owns everything about
        // *when* wireframes actually show (the `F8` toggle, the config
        // default); this just makes the capability exist at all.
        #[cfg(debug_assertions)]
        app.add_plugins(bevy::pbr::wireframe::WireframePlugin::default());
    }
}
