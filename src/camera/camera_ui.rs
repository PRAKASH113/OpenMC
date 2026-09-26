//! The UI camera: menus, screens, and later the HUD are drawn through it.
//!
//! It is spawned once at startup and lives for the whole run, so state
//! screens only spawn their own content and never contend over the view.
//!
//! It is built from Bevy's `Camera2d` component, but it is not a 2D game
//! camera — nothing here draws sprites, and the `2d` engine feature is not
//! needed for it. `Camera2d` is simply the camera kind Bevy renders UI
//! through; the render pass it needs comes with the `ui` feature.

use bevy::camera::CameraOutputMode;
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
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
            // Clear to *transparent*, every frame. This is not optional, and
            // `ClearColorConfig::None` is wrong here even though it reads
            // like "draw over the world".
            //
            // Bevy gives each camera an intermediate texture keyed on its
            // target, format *and MSAA level*. This camera is `Msaa::Off`
            // and the world camera is not, so this one draws into a texture
            // of its own — the world is never in it — and the result is
            // composited onto the window afterwards (see `output_mode`).
            // With `None` that texture is never cleared: each frame's UI
            // lands on top of every previous frame's. A translucent overlay
            // then thickens to near-black within a few frames, and stays on
            // screen forever after its entity is despawned. And because
            // Bevy's texture cache hands the same pool of textures to both
            // cameras, this camera can also pick up an old opaque world
            // frame and paint it over the live one, which looks like frozen
            // controls. Clearing to `Color::NONE` means the texture only ever
            // holds this frame's UI, with the world showing through wherever
            // there is none.
            clear_color: ClearColorConfig::Custom(Color::NONE),
            // How that texture reaches the window. The UI pipeline blends
            // onto a transparent texture, so what it leaves behind is
            // *premultiplied* colour; Bevy's default for a later camera,
            // straight alpha blending, would multiply by alpha a second time
            // and darken every translucent pixel, such as text edges. The
            // window clear stays at its default, which only takes effect for
            // the first camera to write to the window each frame: the world
            // camera while in a world, this one in the menus.
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                clear_color: ClearColorConfig::Default,
            },
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
