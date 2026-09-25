//! The starting scene: six cubes and a light.
//!
//! Placeholder geometry that exists so there is something to fly around and
//! judge the camera against. It is replaced wholesale by the voxel world,
//! so nothing here is built to grow.

use bevy::prelude::*;

/// Marks everything belonging to the scene, so leaving the state clears it
/// in one query rather than by remembering each entity.
#[derive(Component)]
pub(crate) struct SceneEntity;

/// Edge length of each cube.
const CUBE_SIZE: f32 = 2.0;

/// Gap between cube centres.
const SPACING: f32 = 5.0;

/// Where each cube sits, in units of [`SPACING`].
///
/// Two rows of three, offset in depth so the arrangement reads as solid
/// from the starting camera angle rather than as a flat wall.
const CUBE_CELLS: [(f32, f32); 6] = [
    (-1.0, -1.0),
    (0.0, -1.0),
    (1.0, -1.0),
    (-1.0, 1.0),
    (0.0, 1.0),
    (1.0, 1.0),
];

/// Per-cube colour, so each one is individually identifiable while moving
/// the camera around.
const CUBE_COLORS: [Color; 6] = [
    Color::srgb(0.85, 0.30, 0.30),
    Color::srgb(0.85, 0.60, 0.25),
    Color::srgb(0.80, 0.80, 0.30),
    Color::srgb(0.35, 0.75, 0.40),
    Color::srgb(0.30, 0.55, 0.85),
    Color::srgb(0.60, 0.40, 0.85),
];

/// Spawns the scene when the world is entered.
pub(crate) fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // One mesh handle shared by all six cubes: identical geometry has no
    // reason to be uploaded to the GPU six times.
    let cube = meshes.add(Cuboid::from_length(CUBE_SIZE));

    for (cell, color) in CUBE_CELLS.iter().zip(CUBE_COLORS) {
        let (x, z) = *cell;

        commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(materials.add(StandardMaterial::from_color(color))),
            Transform::from_xyz(x * SPACING, 0.0, z * SPACING),
            SceneEntity,
        ));
    }

    // Angled so the cubes get a lit face and a shaded one, which is what
    // makes them read as solid while the camera moves.
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 12.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        SceneEntity,
    ));
}

/// Despawns the scene when the world is left.
pub(crate) fn despawn(mut commands: Commands, entities: Query<Entity, With<SceneEntity>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}
