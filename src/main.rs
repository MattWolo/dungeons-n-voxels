mod camera;
mod voxel;

use bevy::prelude::*;
use camera::CameraPlugin;
use voxel::VoxelPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((CameraPlugin, VoxelPlugin))
        .run();
}