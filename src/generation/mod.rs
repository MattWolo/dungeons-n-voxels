pub mod voxel_type;
pub mod chunk;
pub mod mesh;
pub(crate) mod worldgen;
pub(crate) mod biome_recipes;

use bevy::pbr::wireframe::Wireframe;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

use chunk::Chunk;
use mesh::{build_mesh_from_scratch, THREAD_SCRATCH};
use voxel_type::{CHUNK_X, CHUNK_Z};
use crate::player::Player;

const RENDER_DISTANCE: i32 = 32;
const UNLOAD_DISTANCE: i32 = 36;
pub const WORLD_SEED: u32 = 5345235;
pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(LoadedChunks::default())
            .add_systems(Startup, (setup_chunk_material, spawn_initial_chunks))
            .add_systems(Update, (
                load_chunks_around_player,
                unload_distant_chunks,
                handle_chunk_tasks,
                ));
    }
}
#[derive(Component)]
pub struct ComputeChunkTask(Task<(IVec2, Mesh)>);
#[derive(Component)]
pub struct ChunkCoord(pub IVec2);
#[derive(Resource, Default)]
pub struct LoadedChunks {
    pub entities: HashMap<IVec2, Entity>,
    pub pending: HashSet<IVec2>,
}
#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<StandardMaterial>);

fn spawn_chunk_task(
    commands: &mut Commands,
    coord: IVec2,
    thread_pool: &AsyncComputeTaskPool,
    seed: u32,
) {
    let task = thread_pool.spawn(async move {
        let chunk = Chunk::generate(coord.x, coord.y, seed);

        let west_edge = Chunk::generate_west_edge(coord.x + 1, coord.y, seed);
        let east_edge = Chunk::generate_east_edge(coord.x - 1, coord.y, seed);
        let south_edge = Chunk::generate_south_edge(coord.x, coord.y + 1, seed);
        let north_edge = Chunk::generate_north_edge(coord.x, coord.y - 1, seed);

        THREAD_SCRATCH.with(|scratch_cell| {
            let mut scratch = scratch_cell.borrow_mut();
            let scratch = &mut *scratch;
            scratch.clear();
            
            scratch.own_flat = chunk.flat;

            Chunk::build_padded_chunk(
                &mut scratch.padded,
                &scratch.own_flat,
                Some(&west_edge),
                Some(&east_edge),
                Some(&south_edge),
                Some(&north_edge),
            );

            let mesh = build_mesh_from_scratch(&mut *scratch);
            (coord, mesh)
        })
    });
    commands.spawn((
        ComputeChunkTask(task),
        ChunkCoord(coord),
    ));
}

fn load_chunks_around_player(
    player_query: Query<&Transform, With<Player>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    mut commands: Commands,
) {
    if let Ok(player_transform) = player_query.single() {
        let player_chunk_x = (player_transform.translation.x / CHUNK_X as f32).floor() as i32;
        let player_chunk_z = (player_transform.translation.z / CHUNK_Z as f32).floor() as i32;
        let player_chunk = IVec2::new(player_chunk_x, player_chunk_z);

        let thread_pool = AsyncComputeTaskPool::get();


        for cx in (player_chunk_x - RENDER_DISTANCE)..(player_chunk_x + RENDER_DISTANCE) {
            for cz in (player_chunk_z - RENDER_DISTANCE)..(player_chunk_z + RENDER_DISTANCE) {
                let coord = IVec2::new(cx, cz);

                if !loaded_chunks.entities.contains_key(&coord) && !loaded_chunks.pending.contains(&coord) {
                    loaded_chunks.pending.insert(coord);
                    spawn_chunk_task(&mut commands, coord, thread_pool, WORLD_SEED)
                }
            }
        }
    }
}

fn unload_distant_chunks(
    player_query: Query<&Transform, With<Player>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    mut commands: Commands,
) {
    if let Ok(player_transform) = player_query.single() {
        let player_chunk_x = (player_transform.translation.x / CHUNK_X as f32).floor() as i32;
        let player_chunk_z = (player_transform.translation.z / CHUNK_Z as f32).floor() as i32;

        let chunks_to_unload: Vec<IVec2> = loaded_chunks.entities.iter()
            .filter(|(coord, _)| {
            let dx = coord.x - player_chunk_x;
            let dz = coord.y - player_chunk_z;
            dx.abs() > UNLOAD_DISTANCE || dz.abs() > UNLOAD_DISTANCE
        })
            .map(|(coord, _)| *coord).collect();

        for coord in chunks_to_unload {
            if let Some(entity) = loaded_chunks.entities.remove(&coord) {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn handle_chunk_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputeChunkTask, &ChunkCoord)>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_material: Res<ChunkMaterial>,
    mut loaded_chunks: ResMut<LoadedChunks>,
) {
    for (entity, mut task, chunk_coord) in &mut tasks {
        if let Some((coord, mesh)) = future::block_on(future::poll_once(&mut task.0)) {
            let mesh_entity = commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(chunk_material.0.clone()),
                Transform::from_xyz(
                    coord.x as f32 * CHUNK_X as f32,
                    0.0,
                    coord.y as f32 * CHUNK_Z as f32,
                ),
            )).id();

            loaded_chunks.pending.remove(&coord);
            loaded_chunks.entities.insert(coord, mesh_entity);
            commands.entity(entity).despawn();
        }
    }
}

fn setup_chunk_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });
    commands.insert_resource(ChunkMaterial(handle));
}

fn spawn_initial_chunks(
    mut commands: Commands,
    mut loaded_chunks: ResMut<LoadedChunks>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    const START_DISTANCE: i32 = 4;
    for cx in -START_DISTANCE..START_DISTANCE {
        for cz in -START_DISTANCE..START_DISTANCE {
            let coord = IVec2::new(cx, cz);
            if !loaded_chunks.entities.contains_key(&coord) {
                spawn_chunk_task(&mut commands, coord, thread_pool, WORLD_SEED);
            }
        }
    }
}