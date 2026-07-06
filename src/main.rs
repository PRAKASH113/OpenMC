mod app;
mod config;
mod dev_tools;
mod input;
mod loading;
mod menu;
mod paused;
mod playing;

use bevy::{
    log::{LogPlugin, Level},
    prelude::*,
    window::{PresentMode, WindowMode}
};

use crate::app::AppPlugin;
use crate::config::window::*;
use crate::input::InputPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: WINDOW_TITLE.into(),
                        mode: if BORDERLESS_MODE {
                            WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
                        } else {
                            WindowMode::Windowed
                        },
                        // Deliberately no vsync for now (see docs/performance-investigation.md):
                        // `Fifo` caps FPS to the monitor's refresh rate, which would mask the
                        // real before/after numbers while optimizations get reintroduced one
                        // at a time. Confirmed this wasn't hiding a real bug: FPS reaches 60+
                        // uncapped, so the engine itself is healthy. Switch back to
                        // `PresentMode::Fifo` once this optimization round is done and this
                        // becomes a normal-play build again, not a benchmarking one.
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    filter: "info,wgpu_hal=off,wgpu_core=off,openmc=debug".into(),
                    level: Level::INFO,
                    ..default()
                }),
        )

        .add_plugins((
            AppPlugin,
            InputPlugin,
        ))

        .run();
}
