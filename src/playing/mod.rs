mod player;
mod sky;
mod sun;
mod world;

use bevy::light::GlobalAmbientLight;
use bevy::mesh::Meshable;
use bevy::pbr::{DistanceFog, FogFalloff, MaterialPlugin};
use bevy::prelude::*;

use crate::app::states::GameState;
use crate::config::camera::FOV_DEGREES;
use crate::config::world::{CHUNK_SIZE, RENDER_DISTANCE};
use sky::{SkyMaterial, SkyUniform, SKY_DOME_RADIUS};
use sun::SunCube;
use world::MAX_SURFACE_HEIGHT;

pub(crate) use player::{collision_size, resolve_movement, ControlMode, Grounded, PlayerVelocity};
pub(crate) use world::{ChunkManager, ChunkTile, ChunkWireframe};

/// Bevy's `GlobalAmbientLight` default (`brightness: 80.0`) is tiny next to
/// the sun's `illuminance: 6000.0` — faces with no direct line to the sun
/// were reading as almost pure black, a harsh, unnatural-looking contrast
/// rather than a soft shadow. Raised further (300 wasn't enough) to roughly
/// a fifth of the sun's illuminance: a flat, cheap global fill light (not
/// real bounced/indirect lighting) standing in for "light scattered by the
/// sky," enough to make shadowed faces readable without washing out the
/// sunlit side. Tune to taste — there's no physically "correct" value here.
const AMBIENT_BRIGHTNESS: f32 = 1200.0;

/// Direction the sun shines from (and where the sun cube sits in the sky).
const SUN_DIRECTION: Vec3 = Vec3::new(0.3, 0.6, -0.7);

/// Clearance above the highest possible generated terrain column, so the
/// camera never spawns buried in the ground regardless of the noise value.
const SPAWN_CLEARANCE: f64 = 15.0;

/// Fog fully opaque right at the render-distance edge (`RENDER_DISTANCE`
/// chunks out), so a chunk streaming in or out at the boundary fades through
/// fog instead of visibly popping into/out of existence. Fog starts well
/// before that so the transition itself doesn't read as a visible line.
const FOG_END: f32 = (RENDER_DISTANCE * CHUNK_SIZE) as f32;
const FOG_START: f32 = FOG_END * 0.4;

const INITIAL_YAW: f32 = 0.0;
const INITIAL_PITCH: f32 = -0.4;

/// Spawns above the chunk at the origin, comfortably above any possible
/// generated terrain height, looking down at it.
fn initial_camera_pos() -> Vec3 {
    Vec3::new(16.0, (MAX_SURFACE_HEIGHT + SPAWN_CLEARANCE) as f32, 16.0)
}

/// Tags every entity that belongs to the Playing scene (camera, sky, sun,
/// chunks), so `teardown` can despawn all of it on `OnExit(Playing)` — also
/// used by `dev_tools` to count actual game entities, excluding Bevy's own
/// internal bookkeeping entities (see `docs/performance.md`).
#[derive(Component, Clone, Copy)]
pub(crate) struct PlayingScreen;

/// Marker for the sky dome mesh, re-centered on the camera every frame so it
/// never matters how far the player flies.
#[derive(Component)]
struct SkyDome;

/// The player's free-fly camera. Movement/look input lives in `input::playing`;
/// this just holds the orientation state that system reads and writes.
#[derive(Component)]
pub struct PlayerCamera {
    pub yaw: f32,
    pub pitch: f32,
}

pub struct PlayingPlugin;

impl Plugin for PlayingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GlobalAmbientLight {
            brightness: AMBIENT_BRIGHTNESS,
            ..default()
        })
            .add_plugins((
                MaterialPlugin::<SkyMaterial>::default(),
                world::WorldPlugin,
                player::PlayerPlugin,
            ))
            .add_systems(OnEnter(GameState::Playing), setup)
            .add_systems(OnExit(GameState::Playing), teardown)
            .add_systems(
                Update,
                sync_sky_to_camera.run_if(in_state(GameState::Playing)),
            );
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sun_direction = sun_direction();
    let initial_rotation =
        Quat::from_axis_angle(Vec3::Y, INITIAL_YAW) * Quat::from_axis_angle(Vec3::X, INITIAL_PITCH);

    commands.spawn((
        PlayingScreen,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_DEGREES.to_radians(),
            ..default()
        }),
        PlayerCamera {
            yaw: INITIAL_YAW,
            pitch: INITIAL_PITCH,
        },
        PlayerVelocity::default(),
        Grounded::default(),
        Transform::from_translation(initial_camera_pos()).with_rotation(initial_rotation),
        DistanceFog {
            color: fog_color(),
            falloff: FogFalloff::Linear {
                start: FOG_START,
                end: FOG_END,
            },
            ..default()
        },
    ));

    // Fragment-shaded (not vertex-shaded), so a low-poly dome looks identical to a
    // high-poly one for far less triangle/vertex-processing cost every frame.
    let sky_mesh = Sphere::new(SKY_DOME_RADIUS).mesh().ico(0).unwrap();
    commands.spawn((
        PlayingScreen,
        SkyDome,
        Mesh3d(meshes.add(sky_mesh)),
        MeshMaterial3d(sky_materials.add(SkyMaterial {
            params: SkyUniform::default(),
        })),
    ));

    sun::spawn_sun(&mut commands, &mut meshes, &mut materials, PlayingScreen, sun_direction);
}

fn teardown(mut commands: Commands, screens: Query<Entity, With<PlayingScreen>>) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}

fn sync_sky_to_camera(
    camera: Single<&Transform, (With<PlayerCamera>, Without<SkyDome>, Without<SunCube>)>,
    mut dome: Single<&mut Transform, (With<SkyDome>, Without<SunCube>)>,
    mut sun_cube: Single<&mut Transform, (With<SunCube>, Without<SkyDome>)>,
) {
    dome.translation = camera.translation;
    sun_cube.translation = camera.translation + sun_direction() * sun::SUN_DISTANCE;
}

fn sun_direction() -> Vec3 {
    SUN_DIRECTION.normalize()
}

/// Matches the sky dome's horizon color (rather than an arbitrary gray) so
/// distant chunks fade into the sky instead of into a visibly different fog
/// color right at the horizon.
fn fog_color() -> Color {
    let horizon = SkyUniform::default().horizon_color;
    Color::srgb(horizon.x, horizon.y, horizon.z)
}
