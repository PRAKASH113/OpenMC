//! The game's cameras.
//!
//! Each camera kind gets its own submodule with its own plugin, and this
//! module registers them. The UI camera draws the interface and lives for
//! the whole run; the 3D camera renders the world and exists only while one
//! is loaded.
//!
//! Camera settings that only matter to one camera — draw order, MSAA — live
//! with that camera rather than in shared config, because they are exactly
//! the kind of thing the UI and world cameras disagree about. Their draw
//! orders are the one contract between them: the UI sits above the world.

mod camera_3d;
mod camera_ui;

use bevy::prelude::*;

use camera_3d::Camera3dPlugin;
use camera_ui::UiCameraPlugin;

/// Registers every camera.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((UiCameraPlugin, Camera3dPlugin));
    }
}
