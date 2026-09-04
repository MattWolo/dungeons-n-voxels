pub mod voxel_type;
pub mod chunk;
pub mod mesh;
mod worldgen;
mod biome_recipes;

use bevy::pbr::wireframe::Wireframe;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
//use std::collections::HashMap;

use chunk::Chunk;
use mesh::{build_mesh_from_scratch, THREAD_SCRATCH};
use voxel_type::{CHUNK_X, CHUNK_Z};
const RENDER_DISTANCE: i32 = 64;
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
#[derive(Resource, Default)]
pub struct LoadedChunks {
    pub entities: HashMap<IVec2, Entity>,
}
#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<StandardMaterial>);

fn spawn_chunk_tasks(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
    let _span = info_span!("spawn_chunk_tasks").entered();

    let material_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });

    commands.insert_resource(ChunkMaterial(material_handle));

    let thread_pool = AsyncComputeTaskPool::get();
    let seed = 5345235;

    for cx in 0..RENDER_DISTANCE {
        for cz in 0..RENDER_DISTANCE {
            let coord = IVec2::new(cx, cz);

            let task = thread_pool.spawn(async move {

                let chunk = Chunk::generate(coord.x, coord.y, seed);

                let npx = Chunk::generate(coord.x + 1, coord.y, seed);
                let nnx = Chunk::generate(coord.x - 1, coord.y, seed);
                let npz = Chunk::generate(coord.x, coord.y + 1, seed);
                let nnz = Chunk::generate(coord.x, coord.y - 1, seed);

                THREAD_SCRATCH.with(|scratch_cell| {
                    let mut scratch = scratch_cell.borrow_mut();
                    let scratch = &mut *scratch;
                    scratch.clear();

                    chunk.decompress_into(&mut scratch.own_flat);
                    npx.decompress_into(&mut scratch.neighbors[0]);
                    nnx.decompress_into(&mut scratch.neighbors[1]);
                    npz.decompress_into(&mut scratch.neighbors[2]);
                    nnz.decompress_into(&mut scratch.neighbors[3]);

                    Chunk::build_padded_chunk(
                        &mut scratch.padded,
                        &scratch.own_flat,
                        Some(&scratch.neighbors[0]),
                        Some(&scratch.neighbors[1]),
                        Some(&scratch.neighbors[2]),
                        Some(&scratch.neighbors[3]),
                    );

                    let mesh = build_mesh_from_scratch(&mut *scratch);
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
    chunk_material: Res<ChunkMaterial>,
) {
    let _span = info_span!("handle_chunk_tasks").entered();
    for (entity, mut task) in &mut tasks {
        if let Some((coord, mesh)) = future::block_on(future::poll_once(&mut task.0)) {
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(chunk_material.0.clone()),
                Transform::from_xyz(
                    coord.x as f32 * CHUNK_X as f32,
                    0.0,
                    coord.y as f32 * CHUNK_Z as f32,
                ),
                //Wireframe,
            ));

            commands.entity(entity).despawn();
        }
    }
}