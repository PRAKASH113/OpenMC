use bevy::dev_tools::diagnostics_overlay::{
    DiagnosticsOverlay, DiagnosticsOverlayItem, DiagnosticsOverlayPlugin, DiagnosticsOverlayStatistic,
};
use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, Diagnostics, FrameTimeDiagnosticsPlugin, RegisterDiagnostic,
    SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;

use crate::app::states::GameState;
use crate::playing::PlayingScreen;

const TRIANGLE_COUNT: DiagnosticPath = DiagnosticPath::const_new("triangle_count");

/// Count of entities tagged `PlayingScreen` — i.e. things actually spawned
/// for this scene (camera, sun light, sun cube, the test cube), not Bevy's
/// own internal bookkeeping (every `Resource` and every `Observer` is stored
/// as its own entity — hundreds of them, see `docs/performance.md`). Bevy's
/// built-in `EntityCountDiagnosticsPlugin` counts *all* of that; deliberately
/// not used here since a stat that can't be trusted at a glance isn't worth
/// having.
const GAME_ENTITY_COUNT: DiagnosticPath = DiagnosticPath::const_new("game_entity_count");

/// Single diagnostics HUD: FPS, frame time, triangle count, an honest
/// game-entity count, and process CPU/memory usage. No GPU usage/utilization
/// diagnostic — Bevy has no built-in cross-platform "GPU %" metric (the
/// closest thing, `bevy_render::diagnostic::RenderDiagnosticsPlugin`, reports
/// GPU *frame time* per render pass via timestamp queries, not a utilization
/// percentage, and isn't guaranteed to be supported by every GPU/backend) —
/// left out rather than half-implemented. Only present while
/// `GameState::Playing`, since that's the only demanding scene.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            SystemInformationDiagnosticsPlugin::default(),
            DiagnosticsOverlayPlugin,
        ))
        .register_diagnostic(Diagnostic::new(TRIANGLE_COUNT).with_suffix(" tris"))
        .register_diagnostic(Diagnostic::new(GAME_ENTITY_COUNT))
        .add_systems(OnEnter(GameState::Playing), setup)
        .add_systems(OnExit(GameState::Playing), teardown)
        .add_systems(
            Update,
            (update_triangle_count, update_game_entity_count).run_if(in_state(GameState::Playing)),
        );
    }
}

/// A raw, exact value (not the overlay's default EMA-smoothed average) —
/// discrete counts aren't worth EMA-smoothing.
fn exact(path: DiagnosticPath) -> DiagnosticsOverlayItem {
    DiagnosticsOverlayItem {
        path,
        statistic: DiagnosticsOverlayStatistic::Value,
        precision: 0,
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(DiagnosticsOverlay::new(
        "Performance",
        vec![
            FrameTimeDiagnosticsPlugin::FPS.into(),
            FrameTimeDiagnosticsPlugin::FRAME_TIME.into(),
            exact(TRIANGLE_COUNT),
            exact(GAME_ENTITY_COUNT),
            SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE.into(),
            SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE.into(),
        ],
    ));
}

fn teardown(mut commands: Commands, overlays: Query<Entity, With<DiagnosticsOverlay>>) {
    for entity in &overlays {
        commands.entity(entity).despawn();
    }
}

fn update_triangle_count(
    mut diagnostics: Diagnostics,
    meshes: Res<Assets<Mesh>>,
    query: Query<&Mesh3d>,
) {
    let triangle_count: usize = query
        .iter()
        .filter_map(|mesh3d| meshes.get(&mesh3d.0))
        .filter_map(|mesh| mesh.indices())
        .map(|indices| indices.len() / 3)
        .sum();
    diagnostics.add_measurement(&TRIANGLE_COUNT, || triangle_count as f64);
}

fn update_game_entity_count(mut diagnostics: Diagnostics, query: Query<Entity, With<PlayingScreen>>) {
    let count = query.iter().count();
    diagnostics.add_measurement(&GAME_ENTITY_COUNT, || count as f64);
}
