mod camera;
pub mod generation;
mod metrics;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::prelude::*;
use camera::CameraPlugin;
use generation::ChunkPlugin;
use crate::metrics::MetricsPlugin;

#[derive(Component)]
struct PerfUi;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins((CameraPlugin, ChunkPlugin, MetricsPlugin, FrameTimeDiagnosticsPlugin::default()))
        .run();
}

