pub mod generation;
mod metrics;
mod player;
mod controls;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::prelude::*;
use generation::ChunkPlugin;
use crate::controls::ControlsPlugin;
use crate::metrics::MetricsPlugin;
use crate::player::PlayerPlugin;
use crate::generation::environment::EnvironmentPlugin;

#[derive(Component)]
struct PerfUi;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins((EnvironmentPlugin, MetricsPlugin, ChunkPlugin, PlayerPlugin, ControlsPlugin, FrameTimeDiagnosticsPlugin::default()))
        .run();
}

