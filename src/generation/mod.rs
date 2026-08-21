pub mod types;
pub mod chunk;
pub mod mesh;

use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use chunk::Chunk;
use mesh::create_chunk_mesh;

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App){
        app.add_systems(Startup, spawn_chunk);
    }
}

fn spawn_chunk(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chunk = Chunk::generate();
    let chunk_mesh = create_chunk_mesh(&chunk);

    commands.spawn((
        Mesh3d(meshes.add(chunk_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial{base_color: Color::WHITE, ..default()})),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Wireframe
        ));
}