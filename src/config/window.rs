//! Settings you change.
//!
//! Values only. Getting them into the engine is [`crate::window`]'s job,
//! and the reasoning behind each choice is in `docs/ARCHITECTURE.md`.

use bevy::window::PresentMode;

/// Shown in the window header and the OS task switcher.
pub const TITLE: &str = "OpenMC";

/// Starting window width, in logical pixels (points, not raw screen pixels).
///
/// Winit scales this by the monitor's DPI setting when the window is first
/// created, so the window is the same visual size on a 100% and a 150%
/// display — 1280 logical pixels is ~1920 real screen pixels at 150%. See
/// [`crate::window::toggles`] for why that distinction matters at runtime
/// too, not just at startup.
pub const WIDTH: u32 = 1280;

/// Starting window height, in logical pixels. See [`WIDTH`].
pub const HEIGHT: u32 = 720;

/// Start without the OS title bar and border.
///
/// Only the starting value — `input::TOGGLE_BORDERLESS` flips it at runtime.
pub const BORDERLESS: bool = false;

/// Start in fullscreen.
///
/// Only the starting value — `input::TOGGLE_FULLSCREEN` flips it at runtime.
/// Leaving fullscreen restores [`WIDTH`] x [`HEIGHT`], so those double as the
/// remembered windowed size.
pub const FULLSCREEN: bool = false;

/// How finished frames reach the display.
///
/// `AutoVsync` and `AutoNoVsync` pick the best mode the driver actually supports.
/// so they work everywhere — prefer them unless you are measuring something.
/// `Fifo` (tear-free, one frame of latency), `Mailbox` (tear-free and low latency, burns frames and power) and `Immediate` (lowest latency, tears) force one specific mode and may be unsupported.
pub const PRESENT_MODE: PresentMode = PresentMode::AutoVsync;
