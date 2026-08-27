use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::render::mesh::Indices;
use crate::PerfUi;

pub struct MetricsPlugin;

impl Plugin for MetricsPlugin {
    fn build(&self, app: &mut App){
        app
            .add_systems(Startup, setup_ui)
            .add_systems(Update, update_perf_stats);
    }
}

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("FPS: -- | Triangles: --"),
        TextFont{
            font_size: FontSize::Px(33.0),
            ..default()
        },
        PerfUi,
    ));
}

fn update_perf_stats(
    diagnostics: Res<DiagnosticsStore>,
    meshes: Res<Assets<Mesh>>,
    mesh_query: Query<&Mesh3d>,
    mut text_query: Query<&mut Text, With<PerfUi>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.0);

    let mut total_triangles = 0;
    for mesh_handle in &mesh_query {
        if let Some(mesh) = meshes.get(mesh_handle) {
            if let Some(indices) = mesh.indices() {
                let index_count = match indices {
                    Indices::U16(vec) => vec.len(),
                    Indices::U32(vec) => vec.len(),
                };
                total_triangles += index_count / 3;
            }
        }
    }

    for mut text in &mut text_query {
        **text = format!("FPS: {fps:.0} | Triangles: {total_triangles}");
    }
}