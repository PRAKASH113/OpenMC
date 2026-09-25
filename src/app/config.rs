//! Settings you change.
//!
//! Values only. Getting them into the engine is [`super::window`]'s job,
//! and the reasoning behind each choice is in `docs/ARCHITECTURE.md`.

use bevy::window::PresentMode;

/// Shown in the window header and the OS task switcher.
pub const TITLE: &str = "OpenMC";

/// Starting window width, in physical pixels.
pub const WIDTH: u32 = 1280;

/// Starting window height, in physical pixels.
pub const HEIGHT: u32 = 720;

/// Drop the OS title bar and border.
pub const BORDERLESS: bool = false;

/// How finished frames reach the display.
///
/// `AutoVsync` and `AutoNoVsync` pick the best mode the driver actually supports.
/// so they work everywhere — prefer them unless you are measuring something.
/// `Fifo` (tear-free, one frame of latency), `Mailbox` (tear-free and low latency, burns frames and power) and `Immediate` (lowest latency, tears) force one specific mode and may be unsupported.
pub const PRESENT_MODE: PresentMode = PresentMode::AutoVsync;
