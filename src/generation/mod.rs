pub mod types;
pub mod chunk;
pub mod mesh;

use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use chunk::Chunk;
use mesh::create_chunk_mesh;
use types::{CHUNK_X, CHUNK_Z};
const RENDER_DISTANCE: i32 = 4;
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
    let material_handle = materials.add(StandardMaterial{
        base_color: Color::WHITE,
        ..default()
    });
    for cx in 0..RENDER_DISTANCE{
        for cz in 0..RENDER_DISTANCE{
            let chunk = Chunk::generate();
            let chunk_mesh = create_chunk_mesh(&chunk);

            commands.spawn((
                Mesh3d(meshes.add(chunk_mesh)),
                MeshMaterial3d(material_handle.clone()),
                Transform::from_xyz(
                    cx as f32 * CHUNK_X as f32,
                    0.0,
                    cz as f32 * CHUNK_Z as f32
                ),
                Wireframe
            ));
        }
    }
}