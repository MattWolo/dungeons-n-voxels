mod camera;
pub mod generation;
mod metrics;
mod player;
mod controls;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::prelude::*;
use generation::ChunkPlugin;
//use crate::camera::CameraPlugin;
use crate::controls::ControlsPlugin;
use crate::metrics::MetricsPlugin;
use crate::player::PlayerPlugin;

#[derive(Component)]
struct PerfUi;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins((/*CameraPlugin*/ ChunkPlugin, MetricsPlugin, PlayerPlugin, ControlsPlugin, FrameTimeDiagnosticsPlugin::default()))
        .run();
}

