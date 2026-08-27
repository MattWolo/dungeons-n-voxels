pub mod types;
pub mod chunk;
pub mod mesh;

use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use chunk::Chunk;
use mesh::create_chunk_mesh;
use std::collections::HashMap;
use types::{CHUNK_X, CHUNK_Z};
const RENDER_DISTANCE: i32 = 8;
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

    //generate every chunk and store it in a map
    let mut chunks: HashMap<(i32, i32), Chunk> = HashMap::new();
    for cx in 0..RENDER_DISTANCE {
        for cz in 0..RENDER_DISTANCE{
            chunks.insert((cx, cz), Chunk::generate());
        }
    }

    for cx in 0..RENDER_DISTANCE{
        for cz in 0..RENDER_DISTANCE{
            let chunk = &chunks[&(cx, cz)];
            let own_flat = chunk.decompress();

            let npx = chunks.get(&(cx + 1, cz)).map(|c| c.decompress());
            let nnx = chunks.get(&(cx - 1, cz)).map(|c| c.decompress());
            let npz = chunks.get(&(cx, cz + 1)).map(|c| c.decompress());
            let nnz = chunks.get(&(cx, cz - 1)).map(|c| c.decompress());

            let padded = Chunk::build_padded_chunk(
                &own_flat,
                npx.as_deref(),
                nnx.as_deref(),
                npz.as_deref(),
                nnz.as_deref(),
            );

            let chunk_mesh = create_chunk_mesh(chunk, &padded);

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