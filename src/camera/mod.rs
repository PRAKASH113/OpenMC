//! The game's cameras.
//!
//! Each camera kind gets its own submodule with its own plugin, and this
//! module registers them. The 2D camera renders the UI and lives for the
//! whole run; the 3D camera renders the world and exists only while one is
//! loaded.
//!
//! Camera settings that only matter to one camera — draw order, MSAA — live
//! with that camera rather than in shared config, because they are exactly
//! the kind of thing the 2D and 3D cameras disagree about. Their draw orders
//! are the one contract between them: the UI sits above the world.

mod camera_2d;
mod camera_3d;

use bevy::prelude::*;

use camera_2d::Camera2dPlugin;
use camera_3d::Camera3dPlugin;

/// Registers every camera.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((Camera2dPlugin, Camera3dPlugin));
    }
}
