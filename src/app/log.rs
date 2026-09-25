//! Logging configuration.
//!
//! Logging is development tooling, so the filters live here in one readable
//! place rather than being scattered across the code that emits logs.
//!
//! Note that logs only reach a console in dev builds: `main.rs` sets
//! `windows_subsystem = "windows"` for release, which detaches the console
//! entirely. Diagnosing a player's release build therefore needs a file
//! layer (`LogPlugin::custom_layer`), which does not exist yet.

use bevy::log::{DEFAULT_FILTER, LogPlugin};
use bevy::prelude::*;

/// Log targets silenced on top of Bevy's defaults.
///
/// The Vulkan loader emits an ERROR for every overlay layer it fails to
/// open — Steam and the Epic launcher register layers that are frequently
/// not installed. They are harmless, fire on every single startup, and bury
/// real errors, so the target is turned off entirely.
///
/// The tradeoff: genuine Vulkan *instance* failures go quiet here too. That
/// is tolerable because a real one stops the app anyway, with a clearer
/// message than the loader's. If a GPU bug ever needs diagnosing, comment
/// this out first.
const SILENCED: &str = "wgpu_hal::vulkan::instance=off";

/// Builds the [`LogPlugin`] with our filters layered on Bevy's defaults.
///
/// Built on [`DEFAULT_FILTER`] rather than replacing it, so Bevy's own
/// sensible defaults survive and ours are purely additive.
pub fn log_plugin() -> LogPlugin {
    LogPlugin {
        filter: format!("{DEFAULT_FILTER},{SILENCED}"),
        ..default()
    }
}
