//! The game's cameras.
//!
//! Each camera kind gets its own submodule with its own plugin, and this
//! module registers them. The UI camera (`camera_ui`) draws the interface
//! and lives for the whole run; the world camera (`camera_world`) renders the
//! 3D world and exists only while one is loaded. Both are named for what they
//! show, not for how they render.
//!
//! Camera settings that only matter to one camera — draw order, MSAA — live
//! with that camera rather than in shared config, because they are exactly
//! the kind of thing the UI and world cameras disagree about. Their draw
//! orders are the one contract between them: the UI sits above the world.

mod camera_ui;
mod camera_world;

use bevy::prelude::*;

use camera_ui::UiCameraPlugin;
use camera_world::WorldCameraPlugin;

// What the rest of the crate needs to steer the world camera — its marker and
// its orientation. `crate::input` is the consumer.
pub(crate) use camera_world::{LookAngles, WorldCamera};

/// Registers every camera.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((UiCameraPlugin, WorldCameraPlugin));
    }
}
