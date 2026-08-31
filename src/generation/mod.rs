pub mod types;
pub mod chunk;
pub mod mesh;

use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use std::collections::HashMap;

use chunk::Chunk;
use mesh::{build_mesh_from_scratch, THREAD_SCRATCH};
use types::{CHUNK_X, CHUNK_Z};
const RENDER_DISTANCE: i32 = 2;
pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App){
        app
            .add_systems(Startup, spawn_chunk_tasks)
            .add_systems(Update, handle_chunk_tasks);
    }
}

#[derive(Component)]
pub struct ComputeChunkTask(Task<(IVec2, Mesh)>);

fn spawn_chunk_tasks(mut commands: Commands) {
    let thread_pool = AsyncComputeTaskPool::get();

    let mut chunks: HashMap<IVec2, Chunk> = HashMap::new();
    for cx in 0..RENDER_DISTANCE {
        for cz in 0..RENDER_DISTANCE {
            chunks.insert(IVec2::new(cx, cz), Chunk::generate());
        }
    }

    for cx in 0..RENDER_DISTANCE {
        for cz in 0..RENDER_DISTANCE {
            let coord = IVec2::new(cx, cz);


            let npx = chunks.get(&IVec2::new(cx + 1, cz)).cloned();
            let nnx = chunks.get(&IVec2::new(cx - 1, cz)).cloned();
            let npz = chunks.get(&IVec2::new(cx, cz + 1)).cloned();
            let nnz = chunks.get(&IVec2::new(cx, cz - 1)).cloned();

            let chunk = chunks.get(&coord).unwrap().clone();

            let task = thread_pool.spawn(async move {
                THREAD_SCRATCH.with(|scratch_cell| {
                    let mut scratch_guard = scratch_cell.borrow_mut();
                    let scratch = &mut *scratch_guard;

                    scratch.clear();

                    chunk.decompress_into(&mut scratch.own_flat);

                    let neighbors = [npx, nnx, npz, nnz];
                    for (i, neighbor) in neighbors.iter().enumerate() {
                        if let Some(n) = neighbor {
                            n.decompress_into(&mut scratch.neighbors[i]);
                        }
                    }

                    Chunk::build_padded_chunk(
                        &mut scratch.padded,
                        &scratch.own_flat,
                        neighbors[0].as_ref().map(|_| scratch.neighbors[0].as_slice()),
                        neighbors[1].as_ref().map(|_| scratch.neighbors[1].as_slice()),
                        neighbors[2].as_ref().map(|_| scratch.neighbors[2].as_slice()),
                        neighbors[3].as_ref().map(|_| scratch.neighbors[3].as_slice()),
                    );
                    let mesh = build_mesh_from_scratch(scratch);
                    (coord, mesh)
                })
            });
            commands.spawn(ComputeChunkTask(task));
        }
    }
}

fn handle_chunk_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputeChunkTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });

    for (entity, mut task) in &mut tasks {
        if let Some((coord, mesh)) = future::block_on(future::poll_once(&mut task.0)) {
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material_handle.clone()),
                Transform::from_xyz(
                    coord.x as f32 * CHUNK_X as f32,
                    0.0,
                    coord.y as f32 * CHUNK_Z as f32,
                ),
                Wireframe,
            ));

            commands.entity(entity).despawn();
        }
    }
}