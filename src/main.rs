mod camera;
pub mod generation;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::prelude::*;
use camera::CameraPlugin;
use generation::ChunkPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins((CameraPlugin, ChunkPlugin))
        .run();
}