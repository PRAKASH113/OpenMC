//! The UI camera: menus, screens, and later the HUD are drawn through it.
//!
//! It is spawned once at startup and lives for the whole run, so state
//! screens only spawn their own content and never contend over the view.
//!
//! It is built from Bevy's `Camera2d` component, but it is not a 2D game
//! camera — nothing here draws sprites, and the `2d` engine feature is not
//! needed for it. `Camera2d` is simply the camera kind Bevy renders UI
//! through; the render pass it needs comes with the `ui` feature.

use bevy::prelude::*;
use bevy::render::view::Msaa;

/// Marks the UI camera.
///
/// Once the 3D world camera exists both are `Camera` entities, and this is
/// what tells them apart in a query.
#[derive(Component)]
pub struct UiCamera;

/// Draw order for the UI camera.
///
/// Higher orders render later and therefore on top. The 3D world camera
/// takes a lower value when it arrives, so the UI stays above the world
/// without either camera having to know about the other.
const ORDER: isize = 1;

/// Spawns and owns the UI camera.
pub struct UiCameraPlugin;

impl Plugin for UiCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ui_camera);
    }
}

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: ORDER,
            // This camera draws after the world camera, so clearing here
            // would erase the world. The world camera clears instead.
            //
            // In states with no world camera the screen is therefore never
            // cleared — which is safe only because every such state paints
            // an opaque full-screen background. A state that does not must
            // either clear itself or bring its own camera.
            clear_color: ClearColorConfig::None,
            ..default()
        },
        // The UI is axis-aligned quads and text sampled from a font atlas,
        // so multisampling changes nothing visible here while costing a 4x
        // render target and the bandwidth to resolve it every frame. Bevy
        // defaults to `Sample4`. The 3D camera, where geometry edges
        // actually alias, picks its own level.
        Msaa::Off,
        UiCamera,
    ));
}
