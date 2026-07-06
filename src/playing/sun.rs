use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

/// Distance of the sun from the camera. Kept far and re-centered on the camera
/// every frame (see `sync_sky_to_camera`) so flying around never changes its size.
pub const SUN_DISTANCE: f32 = 400.0;
/// Side length of the sun cube.
pub const SUN_SIZE: f32 = 24.0;
/// Euler tilt applied to the cube so it reads as a cube (an edge/corner facing
/// the camera) instead of a flat axis-aligned square.
const SUN_TILT: (f32, f32, f32) = (0.45, 0.6, 0.3);
/// Over-bright emissive color (values above 1.0 push it past the camera's
/// bloom threshold), giving the cube a soft glow instead of a flat hard edge.
const SUN_EMISSIVE: LinearRgba = LinearRgba::rgb(4.0, 3.6, 2.4);

/// Marker for the visible sun cube, re-centered on the camera every frame
/// (unlike the `DirectionalLight`, whose position doesn't affect lighting).
#[derive(Component)]
pub struct SunCube;

pub fn spawn_sun(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    marker: impl Component + Clone,
    sun_direction: Vec3,
) {
    let position = sun_direction * SUN_DISTANCE;

    commands.spawn((
        marker.clone(),
        DirectionalLight {
            illuminance: 6000.0,
            ..default()
        },
        Transform::from_translation(position).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        marker,
        SunCube,
        Mesh3d(meshes.add(Cuboid::new(SUN_SIZE, SUN_SIZE, SUN_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            emissive: SUN_EMISSIVE,
            unlit: false,
            // The sun sits `SUN_DISTANCE` (400) units out — deliberately far
            // beyond the terrain's `DistanceFog` range (fully opaque by
            // `RENDER_DISTANCE * CHUNK_SIZE`, currently 128), since it's meant
            // to read as infinitely distant, not part of the local foggy
            // atmosphere. Without this it fades to solid fog color, i.e. it
            // looks gray instead of its own emissive color.
            fog_enabled: false,
            ..default()
        })),
        Transform::from_translation(position).with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            SUN_TILT.0,
            SUN_TILT.1,
            SUN_TILT.2,
        )),
        NotShadowCaster,
        NotShadowReceiver,
    ));
}
