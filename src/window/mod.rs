//! The window: building it at startup, and changing it at runtime.
//!
//! These are two different lifecycles, so they are two files. [`setup`] runs
//! once, turning [`crate::config::window`] into Bevy's window settings before
//! the app starts. [`toggles`] runs every frame for the life of the game,
//! handling the borderless and fullscreen keys by mutating the live `Window`
//! component, which is how Bevy expects runtime window changes to be made.
//!
//! This is the only place that knows the engine's window vocabulary, so the
//! config stays a plain list of values.

mod setup;
mod toggles;

use bevy::window::{MonitorSelection, WindowMode};

pub use setup::primary_window_plugin;
pub use toggles::WindowControlPlugin;

/// The fullscreen mode both halves use — the starting mode when
/// `config::FULLSCREEN` is set, and the mode the toggle switches to.
///
/// Borderless fullscreen rather than exclusive: it alt-tabs instantly and
/// does not change the display's video mode, which is what players expect
/// from a modern game. Exclusive fullscreen would be
/// `WindowMode::Fullscreen(..)`.
const FULLSCREEN_MODE: WindowMode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
