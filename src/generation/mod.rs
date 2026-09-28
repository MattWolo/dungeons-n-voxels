pub mod voxel_type;
pub mod chunk;
pub mod mesh;
pub mod worldgen;
pub mod biome_recipes;
pub mod environment;
mod moon_material;
mod voxel_material;
mod moon_generation;
mod LOD;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::ExtendedMaterial;
use bevy::pbr::wireframe::Wireframe;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_sky_gradient::plugin::GradientTextureHandle;
use futures_lite::future;

use chunk::Chunk;
use mesh::{
    build_mesh_section_from_scratch,
    pack_vertex_data,
    ATTRIBUTE_VOXEL_DATA,
    THREAD_SCRATCH,
};
use voxel_type::{
    CHUNK_X,
    CHUNK_Z,
    MESH_SECTION_COUNT,
    WORLD_MIN_Y,
};
use crate::generation::LOD::{lod_for_distance_sq, LodLevel, FULL_DETAIL_DISTANCE};
use crate::generation::voxel_material::{VoxelMaterialExtension, VoxelMaterialSettings};
use crate::generation::voxel_type::{VoxelType, CHUNK_Y};
use crate::generation::worldgen::sample_terrain;
use crate::player::Player;

const RENDER_DISTANCE: i32 = 32;
const UNLOAD_DISTANCE: i32 = 33;
const FAR_LOD_STEP: i32 = 2;
const FAR_LOD_SIZE: usize = CHUNK_X / FAR_LOD_STEP as usize;
const FAR_CELLS_X: usize = CHUNK_X / FAR_LOD_STEP as usize;
const FAR_CELLS_Z: usize = CHUNK_Z / FAR_CELLS_X;
const FAR_PADDED_SIZE: usize = FAR_LOD_SIZE + 2;
const FAR_GRID_X: usize = FAR_CELLS_X + 2;
const FAR_GRID_Z: usize = FAR_CELLS_Z + 2;
pub const WORLD_SEED: u32 = 5345235;
const MAX_PENDING_COLUMNS: usize = 64;
const MAX_NEW_COLUMNS_PER_FRAME: usize = 8;
const MAX_FINISHED_COLUMNS_PER_FRAME: usize = 8;
const MANDATORY_DISTANCE: i32 = FULL_DETAIL_DISTANCE;
pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(LoadedChunks::default())
            .insert_resource(ChunkStreamState::default())
            .insert_resource(ChunkLoadOffsets::new())
            .add_plugins(MaterialPlugin::<ChunkMaterialHandle>::default())
            .add_systems(Startup, (setup_chunk_material, /*spawn_initial_chunks*/))
            .add_systems(Update, (
                handle_chunk_tasks,
                unload_distant_chunks,
                load_chunks_around_player,
            ).chain());
    }
}

pub struct BuiltSectionMesh{
    pub section_index: usize,
    pub mesh: Mesh,
    pub triangle_count: usize,
    pub vertex_count: usize,
}
pub type SectionMeshes = Vec<BuiltSectionMesh>;
#[derive(Component)]
pub struct ComputeChunkTask(Task<BuiltColumn>);
#[derive(Component)]
pub struct ChunkCoord(pub IVec2);
#[derive(Resource, Default)]
pub struct LoadedChunks {
    pub columns: HashMap<IVec2, LoadedColumn>,
    pub pending: HashSet<IVec2>,
}
pub struct LoadedColumn {
    pub entities: Vec<Entity>,
    pub lod: LodLevel,
}
pub struct BuiltColumn {
    pub coord: IVec2,
    pub lod: LodLevel,
    pub meshes: Vec<BuiltSectionMesh>,
}
#[derive(Component, Debug, Clone, Copy)]
pub struct ChunkSection {
    pub column: IVec2,
    pub section_index: usize,
}
#[derive(Component, Clone, Copy)]
pub struct ChunkMeshStats {
    pub triangles: usize,
    pub vertices: usize,
}
#[derive(Clone, Copy)]
struct LodCell {
    height: i32,
    voxel: VoxelType,
}
#[derive(Resource, Default)]
pub struct ChunkStreamState {
    pub center: Option<IVec2>,
    pub cursor: usize,
}
#[derive(Resource)]
pub struct ChunkLoadOffsets {
    pub offsets: Vec<(i32, IVec2)>,
}
impl ChunkLoadOffsets {
    pub fn new() -> Self {
        let mut offsets = Vec::new();

        for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let distance_sq = dx*dx + dz*dz;
                if distance_sq > RENDER_DISTANCE * RENDER_DISTANCE {
                    continue;
                }
                offsets.push((distance_sq, IVec2::new(dx, dz),));
            }
        }
        offsets.sort_unstable_by_key(|(distance_sq, _)| *distance_sq);
        Self { offsets }
    }
}

fn build_full_column(coord: IVec2, seed: u32, ) -> Vec<BuiltSectionMesh> {
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

        let mut section_meshes: SectionMeshes = Vec::with_capacity(MESH_SECTION_COUNT);

        for section_index in 0..MESH_SECTION_COUNT {
            if let Some(section) = build_mesh_section_from_scratch(scratch, section_index) {
                section_meshes.push(section);
            }
        }
        section_meshes
    })
}

fn emit_lod_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    vertex_data: &mut Vec<u32>,
    indices: &mut Vec<u32>,

    vertices: [[f32; 3]; 4],
    normal: [f32; 3],
    voxel: VoxelType,
) {
    let base = positions.len() as u32;
    positions.extend_from_slice(&vertices);
    normals.extend_from_slice(&[normal; 4]);
    let packed = pack_vertex_data(voxel, 0);
    vertex_data.extend_from_slice(&[packed; 4]);

    indices.extend_from_slice(&[
        base,
        base + 1,
        base + 2,
        base,
        base + 2,
        base + 3,
    ]);
}

fn build_far_column(coord: IVec2, seed: u32) -> Vec<BuiltSectionMesh> {
    let _span = info_span!("build_far_column").entered();
    let mut cells = vec![
        LodCell {
            height: WORLD_MIN_Y,
            voxel: VoxelType::Grass,
        };
        FAR_PADDED_SIZE * FAR_PADDED_SIZE
    ];

    let lod_index = |x: usize, z: usize| {
        z * FAR_PADDED_SIZE + x
    };

    for pz in 0..FAR_PADDED_SIZE {
        for px in 0..FAR_PADDED_SIZE {
            let cell_x =
                px as i32 - 1;

            let cell_z =
                pz as i32 - 1;

            cells[lod_index(px, pz)] =
                sample_lod_cell(
                    coord,
                    cell_x,
                    cell_z,
                    seed,
                );
        }
    }

    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut vertex_data = Vec::<u32>::new();
    let mut indices = Vec::<u32>::new();
    let step = FAR_LOD_STEP as f32;

    for z in 0..FAR_LOD_SIZE {
        for x in 0..FAR_LOD_SIZE {
            let px = x + 1;
            let pz = z + 1;
            let current = cells[lod_index(px, pz)];
            let left = cells[lod_index(px - 1, pz)];
            let right = cells[lod_index(px + 1, pz)];
            let front = cells[lod_index(px, pz - 1)];
            let back = cells[lod_index(px, pz + 1)];

            let min_x = x as f32 * step - 0.5;
            let max_x = min_x + step;
            let min_z = z as f32 * step - 0.5;
            let max_z = min_z + step;

            let top_y = (current.height - WORLD_MIN_Y) as f32 - 0.5;

            emit_lod_quad(
                &mut positions,
                &mut normals,
                &mut vertex_data,
                &mut indices,
                [
                    [min_x, top_y, max_z],
                    [max_x, top_y, max_z],
                    [max_x, top_y, min_z],
                    [min_x, top_y, min_z],
                ],
                [0.0, 1.0, 0.0],
                current.voxel,
            );

            if current.height > right.height {
                let low_y = (right.height - WORLD_MIN_Y) as f32 - 0.5;
                emit_lod_quad(
                    &mut positions,
                    &mut normals,
                    &mut vertex_data,
                    &mut indices,

                    [
                        [max_x, low_y, max_z],
                        [max_x, low_y, min_z],
                        [max_x, top_y, min_z],
                        [max_x, top_y, max_z],
                    ],
                    [1.0, 0.0, 0.0],
                    current.voxel,
                );
            }

            if current.height > left.height {
                let low_y = (left.height - WORLD_MIN_Y) as f32 - 0.5;

                emit_lod_quad(
                    &mut positions,
                    &mut normals,
                    &mut vertex_data,
                    &mut indices,

                    [
                        [min_x, low_y, min_z],
                        [min_x, low_y, max_z],
                        [min_x, top_y, max_z],
                        [min_x, top_y, min_z],
                    ],

                    [-1.0, 0.0, 0.0],
                    current.voxel,
                );
            }

            if current.height > front.height {
                let low_y = (front.height - WORLD_MIN_Y) as f32 - 0.5;
                emit_lod_quad(
                    &mut positions,
                    &mut normals,
                    &mut vertex_data,
                    &mut indices,

                    [
                        [max_x, low_y, min_z],
                        [min_x, low_y, min_z],
                        [min_x, top_y, min_z],
                        [max_x, top_y, min_z],
                    ],
                    [0.0, 0.0, -1.0],
                    current.voxel,
                );
            }
            if current.height > back.height {
                let low_y = (back.height - WORLD_MIN_Y) as f32 - 0.5;

                emit_lod_quad(
                    &mut positions,
                    &mut normals,
                    &mut vertex_data,
                    &mut indices,

                    [
                        [min_x, low_y, max_z],
                        [max_x, low_y, max_z],
                        [max_x, top_y, max_z],
                        [min_x, top_y, max_z],
                    ],
                    [0.0, 0.0, 1.0],
                    current.voxel,
                );
            }
        }
    }

    if indices.is_empty() {
        return Vec::new();
    }
    let triangle_count = indices.len() / 3;
    let vertex_count = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        positions,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        normals,
    );
    mesh.insert_attribute(
        ATTRIBUTE_VOXEL_DATA,
        vertex_data,
    );
    mesh.insert_indices(
        Indices::U32(indices)
    );

    vec![
        BuiltSectionMesh {
            section_index: 0,
            mesh,
            triangle_count,
            vertex_count,
        }
    ]
}

fn quantize_lod_height(height: i32, step: i32) -> i32 {
    let local = (height - WORLD_MIN_Y).clamp(0, CHUNK_Y as i32);
    let quantized = (local / step) * step;
    WORLD_MIN_Y + quantized
}
fn sample_lod_cell(coord: IVec2, cell_x: i32, cell_z: i32, seed: u32) -> LodCell {
    let chunk_world_x = coord.x * CHUNK_X as i32;
    let chunk_world_z = coord.y * CHUNK_Z as i32;
    let half_step = FAR_LOD_STEP / 2;
    let world_x = chunk_world_x + cell_x * FAR_LOD_STEP + half_step;
    let world_z = chunk_world_z + cell_z * FAR_LOD_STEP + half_step;
    let sampled = sample_terrain(world_x as f32, world_z as f32, seed);
    LodCell {
        height: quantize_lod_height(sampled.height, FAR_LOD_STEP),
        voxel: sampled.surface,
    }
}

fn spawn_chunk_task(
    commands: &mut Commands,
    coord: IVec2,
    lod: LodLevel,
    thread_pool: &AsyncComputeTaskPool,
    seed: u32,
) {
    let task = thread_pool.spawn(async move {
        let meshes = match lod {
            LodLevel::Full => {
                build_full_column(coord, seed, )
            }
            LodLevel::Far => {
                build_far_column(coord, seed)
            }
        };

        BuiltColumn {
            coord,
            lod,
            meshes,
        }
    });

    commands.spawn(
        ComputeChunkTask(task),
    );
}

fn load_chunks_around_player(
    player_query: Query<&Transform, With<Player>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    load_offsets: Res<ChunkLoadOffsets>,
    mut stream_state: ResMut<ChunkStreamState>,
    mut commands: Commands,
) {
    let _span = info_span!("load_chunks_around_player").entered();
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_chunk_x = (player_transform.translation.x / CHUNK_X as f32).floor() as i32;
    let player_chunk_z = (player_transform.translation.z / CHUNK_Z as f32).floor() as i32;

    let player_chunk = IVec2::new(
        player_chunk_x,
        player_chunk_z,
    );

    if stream_state.center != Some(player_chunk) {
        stream_state.center = Some(player_chunk);
        stream_state.cursor = 0;
    }

    if stream_state.cursor >= load_offsets.offsets.len()
    {
        return;
    }

    let available_slots = MAX_PENDING_COLUMNS.saturating_sub(loaded_chunks.pending.len());
    if available_slots == 0 {
        return;
    }
    let amount_to_spawn = MAX_NEW_COLUMNS_PER_FRAME.min(available_slots);

    let near_ready = mandatory_near_area_loaded(&loaded_chunks, player_chunk);

    let thread_pool = AsyncComputeTaskPool::get();

    let mut spawned = 0usize;

    while spawned < amount_to_spawn && stream_state.cursor < load_offsets.offsets.len() {
        let (distance_sq, offset) = load_offsets.offsets[stream_state.cursor];
        let coord = player_chunk + offset;
        let desired_lod = lod_for_distance_sq(distance_sq);
        if !near_ready && desired_lod == LodLevel::Far {
            break;
        }
        stream_state.cursor += 1;

        let needs_build = match loaded_chunks.columns.get(&coord) {
            None => true,
            Some(column) => {
                column.lod != desired_lod
            }
        };
        if !needs_build {
            continue;
        }
        if loaded_chunks.pending.contains(&coord) {
            continue;
        }
        loaded_chunks.pending.insert(coord);

        spawn_chunk_task(
            &mut commands,
            coord,
            desired_lod,
            thread_pool,
            WORLD_SEED,
        );
        spawned += 1;
    }
}

fn mandatory_near_area_loaded(
    loaded_chunks: &LoadedChunks,
    player_chunk: IVec2,
) -> bool {
    const R: i32 = MANDATORY_DISTANCE;

    for dx in -R..=R {
        for dz in -R..=R {
            if dx * dx + dz * dz > R * R {
                continue;
            }

            let coord = player_chunk
                + IVec2::new(dx, dz);

            let Some(column) =
                loaded_chunks.columns.get(&coord)
            else {
                return false;
            };

            if column.lod != LodLevel::Full {
                return false;
            }
        }
    }

    true
}
fn unload_distant_chunks(
    player_query: Query<&Transform, With<Player>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    mut commands: Commands,
) {
    let _span = info_span!("unload_distant_chunks").entered();
    if let Ok(player_transform) = player_query.single() {
        let player_chunk_x = (player_transform.translation.x / CHUNK_X as f32).floor() as i32;
        let player_chunk_z = (player_transform.translation.z / CHUNK_Z as f32).floor() as i32;

        let chunks_to_unload: Vec<IVec2> = loaded_chunks.columns.iter()
            .filter(|(coord, _)| {
                let dx = coord.x - player_chunk_x;
                let dz = coord.y - player_chunk_z;
                let distance_sq = dx * dx + dz * dz;
                let unload_sq = UNLOAD_DISTANCE * UNLOAD_DISTANCE;
                distance_sq > unload_sq
            })
            .map(|(coord, _)| *coord).collect();

        for coord in chunks_to_unload {
            if let Some(column) = loaded_chunks.columns.remove(&coord) {
                for entity in column.entities {
                    commands.entity(entity).try_despawn();
                }
            }
        }
    }
}

fn handle_chunk_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputeChunkTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_material: Res<ChunkMaterial>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    player_query: Query<&Transform, With<Player>>,
) {
    let _span = info_span!("handle_chunk_tasks").entered();
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_chunk_x = (player_transform.translation.x / CHUNK_X as f32).floor() as i32;
    let player_chunk_z = (player_transform.translation.z / CHUNK_Z as f32).floor() as i32;

    let mut handled_this_frame = 0usize;

    for (task_entity, mut task) in &mut tasks {
        if handled_this_frame >= MAX_FINISHED_COLUMNS_PER_FRAME {
            break;
        }

        let Some(built) = future::block_on(future::poll_once(&mut task.0))
        else {
            continue;
        };

        handled_this_frame += 1;

        let BuiltColumn {
            coord,
            lod,
            meshes: built_meshes,
        } = built;

        let dx = coord.x - player_chunk_x;
        let dz = coord.y - player_chunk_z;
        let distance_sq = dx * dx + dz * dz;

        if distance_sq > UNLOAD_DISTANCE * UNLOAD_DISTANCE {
            loaded_chunks.pending.remove(&coord);

            commands.entity(task_entity).try_despawn();

            continue;
        }

        let desired_lod = lod_for_distance_sq(distance_sq);

        if lod != desired_lod {
            loaded_chunks.pending.remove(&coord);
            commands.entity(task_entity).try_despawn();

            continue;
        }

        let mut new_entities = Vec::with_capacity(built_meshes.len());

        for section in built_meshes {
            let entity = commands.spawn((
                Mesh3d(meshes.add(section.mesh)),
                MeshMaterial3d(chunk_material.0.clone()),
                Transform::from_xyz(
                    coord.x as f32 * CHUNK_X as f32,
                    WORLD_MIN_Y as f32,
                    coord.y as f32 * CHUNK_Z as f32,
                ),

                ChunkSection {
                    column:coord,
                    section_index: section.section_index,
                },

                ChunkMeshStats {
                    triangles: section.triangle_count,
                    vertices: section.vertex_count,
                }
                //Wireframe
            )).id();
            new_entities.push(entity);
        }

        if let Some(old_column) = loaded_chunks.columns.remove(&coord) {
            for entity in old_column.entities {
                commands.entity(entity).try_despawn();
            }
        }

        loaded_chunks.columns.insert(
            coord,
            LoadedColumn{
                entities: new_entities,
                lod,
            },
        );
        loaded_chunks.pending.remove(&coord);
        commands.entity(task_entity).try_despawn();
    }
}
pub type ChunkMaterialHandle = ExtendedMaterial<StandardMaterial, VoxelMaterialExtension>;

#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<ChunkMaterialHandle>);

fn setup_chunk_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<ChunkMaterialHandle>>,
    gradient_texture: Res<GradientTextureHandle>,
) {
    let _span = info_span!("setup_chunk_material").entered();
    let handle = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.9,
            reflectance: 0.1,
            fog_enabled: false,
            ..default()
        },
        extension: VoxelMaterialExtension {
            settings: VoxelMaterialSettings {
                distance_start: 500.0, //fog
                distance_end: 900.0,
                ..default()
            },
            sky_texture: gradient_texture.render_target.clone(),
        },
    });
    commands.insert_resource(ChunkMaterial(handle));
}

// fn spawn_initial_chunks(
//     mut commands: Commands,
//     mut loaded_chunks: ResMut<LoadedChunks>,
// ) {
//     let thread_pool = AsyncComputeTaskPool::get();
//     const START_DISTANCE: i32 = 4;
//     for cx in -START_DISTANCE..START_DISTANCE {
//         for cz in -START_DISTANCE..START_DISTANCE {
//             let distance_sq = cx * cx + cz * cz;
//             let coord = IVec2::new(cx, cz);
//             let desired_lod = lod_for_distance_sq(distance_sq);
//             if loaded_chunks.columns.contains_key(&coord) || loaded_chunks.pending.contains(&coord) {
//                 continue;
//             }
//             loaded_chunks.pending.insert(coord);
//
//             spawn_chunk_task(&mut commands, coord, desired_lod, thread_pool, WORLD_SEED);
//         }
//     }
// }