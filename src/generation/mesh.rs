use bevy::mesh::{Indices, Mesh, MeshVertexAttribute, VertexFormat};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::asset::RenderAssetUsages;
use std::cell::RefCell;
use tracing::info_span;
use super::voxel_type::*;
use super::chunk::{padded_index, PADDED_X, PADDED_Y, PADDED_Z};
const RIGHT_NORMAL: [f32; 3] = [1.0, 0.0, 0.0];
const LEFT_NORMAL: [f32; 3] = [-1.0, 0.0, 0.0];
const TOP_NORMAL: [f32; 3] = [0.0, 1.0, 0.0];
const BOTTOM_NORMAL: [f32; 3] = [0.0, -1.0, 0.0];
const BACK_NORMAL: [f32; 3] = [0.0,  0.0, 1.0];
const FRONT_NORMAL: [f32; 3] = [0.0,  0.0, -1.0];
const EXPECTED_VERTICES: usize = 8_192;
const EXPECTED_INDICES: usize = 16_384;
const EXPECTED_AO: usize = 8_192;

pub type Ao = u8;
pub struct ChunkMeshScratch {
    pub own_flat: Vec<VoxelType>,
    pub neighbors: [Vec<VoxelType>; 4],
    pub padded: Vec<VoxelType>,

    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub ao: Vec<Ao>,
    pub indices: Vec<u32>,

    pub mask_y: [u32; CHUNK_Y],
    pub mask_z: [u32; CHUNK_Z],
    pub face_keys_y: [[u16; CHUNK_Z]; CHUNK_Y],
    pub face_keys_z: [[u16; CHUNK_X]; CHUNK_Z],
    pub face_keys_x: [[u16; CHUNK_X]; CHUNK_Y],
}

pub const ATTRIBUTE_AO: MeshVertexAttribute =
    MeshVertexAttribute::new(
        "Vertex_AO",
        8,
        VertexFormat::Float32,
    );

impl ChunkMeshScratch {
    pub fn new() -> Self {
        Self {
            own_flat: Vec::with_capacity(CHUNK_VOLUME),
            neighbors: std::array::from_fn(|_| Vec::with_capacity(CHUNK_VOLUME)),
            padded: vec![VoxelType::Air; PADDED_X * PADDED_Y * PADDED_Z],
            positions: Vec::with_capacity(EXPECTED_VERTICES),
            normals: Vec::with_capacity(EXPECTED_VERTICES),
            colors: Vec::with_capacity(EXPECTED_VERTICES),
            ao: Vec::with_capacity(EXPECTED_AO),
            face_keys_y: [[0u16; CHUNK_Z]; CHUNK_Y],
            face_keys_z: [[0u16; CHUNK_X]; CHUNK_Z],
            face_keys_x: [[0u16; CHUNK_X]; CHUNK_Y],
            indices: Vec::with_capacity(EXPECTED_INDICES),
            mask_y: [0u32; CHUNK_Y],
            mask_z: [0u32; CHUNK_Z],
        }
    }

    pub fn clear(&mut self) {
        self.own_flat.clear();
        for n in &mut self.neighbors{
            n.clear();
        }

        self.positions.reserve(EXPECTED_VERTICES);
        self.normals.reserve(EXPECTED_VERTICES);
        self.colors.reserve(EXPECTED_VERTICES);
        self.ao.reserve(EXPECTED_AO);
        self.indices.reserve(EXPECTED_INDICES);

        self.mask_y.fill(0);
        self.mask_z.fill(0);
        self.face_keys_y.fill([0u16; CHUNK_Z]);
        self.face_keys_z.fill([0u16; CHUNK_X]);
        self.face_keys_x.fill([0u16; CHUNK_X]);
    }
}

thread_local! {
    pub static THREAD_SCRATCH: RefCell<ChunkMeshScratch> = RefCell::new(ChunkMeshScratch::new());
}

pub fn build_mesh_from_scratch(scratch: &mut ChunkMeshScratch) -> Mesh {
    let _span = info_span!("build_mesh_from_scratch").entered();
    let mut vertex_offset: u32 = 0;

    mesh_right_faces_binary(
        &scratch.padded,
        &mut scratch.mask_y,
        &mut scratch.face_keys_y,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.ao,
        &mut scratch.indices,
        &mut vertex_offset
    );
    mesh_left_faces_binary(
        &scratch.padded,
        &mut scratch.mask_y,
        &mut scratch.face_keys_y,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.ao,
        &mut scratch.indices,
        &mut vertex_offset
    );
    mesh_top_faces_binary(
        &scratch.padded,
        &mut scratch.mask_z,
        &mut scratch.face_keys_z,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.ao,
        &mut scratch.indices,
        &mut vertex_offset
    );

    mesh_bottom_faces_binary(
        &scratch.padded,
        &mut scratch.mask_z,
        &mut scratch.face_keys_z,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.ao,
        &mut scratch.indices,
        &mut vertex_offset
    );

    mesh_front_faces_binary(
        &scratch.padded,
        &mut scratch.mask_y,
        &mut scratch.face_keys_x,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.ao,
        &mut scratch.indices,
        &mut vertex_offset
    );

    mesh_back_faces_binary(
        &scratch.padded,
        &mut scratch.mask_y,
        &mut scratch.face_keys_x,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.ao,
        &mut scratch.indices,
        &mut vertex_offset
    );

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    );
    //println!("Vertices: {}, Indices: {}", scratch.positions.len(), scratch.indices.len());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, std::mem::take(&mut scratch.positions));
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, std::mem::take(&mut scratch.normals));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, std::mem::take(&mut scratch.colors));
    let ao_normalized: Vec<f32> = std::mem::take(&mut scratch.ao)
        .into_iter()
        .map(|occlusion| 1.0 - occlusion as f32 / 3.0)
        .collect();
    mesh.insert_attribute(ATTRIBUTE_AO, ao_normalized);
    mesh.insert_indices(Indices::U32(std::mem::take(&mut scratch.indices)));
    mesh
}

// RIGHT   +X => px + 1
// LEFT    -X => px - 1
//
// TOP     +Y => py + 1
// BOTTOM  -Y => py - 1
//
// BACK    +Z => pz + 1
// FRONT   -Z => pz - 1

pub fn mesh_right_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Y],
    face_keys: &mut [[u16; CHUNK_Z]; CHUNK_Y],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    ao: &mut Vec<Ao>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for x in 0..CHUNK_X {
        masks.fill(0);
        face_keys.fill([0u16; CHUNK_Z]);
        for y in 0..CHUNK_Y {
            let mut row_mask = 0u32;

            for z in 0..CHUNK_Z {
                let px = x + 1;
                let py = y + 1;
                let pz = z + 1;

                let voxel = padded[padded_index(px, py, pz)];

                if voxel == VoxelType::Air
                    || is_solid(padded, px + 1, py, pz)
                {
                    continue;
                }

                let ao0 = vertex_ao(
                    is_solid(padded, px + 1, py - 1, pz),
                    is_solid(padded, px + 1, py, pz + 1),
                    is_solid(padded, px + 1, py - 1, pz + 1),
                );

                let ao1 = vertex_ao(
                    is_solid(padded, px + 1, py - 1, pz),
                    is_solid(padded, px + 1, py, pz - 1),
                    is_solid(padded, px + 1, py - 1, pz - 1),
                );

                let ao2 = vertex_ao(
                    is_solid(padded, px + 1, py + 1, pz),
                    is_solid(padded, px + 1, py, pz - 1),
                    is_solid(padded, px + 1, py + 1, pz - 1),
                );

                let ao3 = vertex_ao(
                    is_solid(padded, px + 1, py + 1, pz),
                    is_solid(padded, px + 1, py, pz + 1),
                    is_solid(padded, px + 1, py + 1, pz + 1),
                );

                let ao_mask =
                    (ao0 << 0)
                        | (ao1 << 2)
                        | (ao2 << 4)
                        | (ao3 << 6);

                face_keys[y][z] = pack_face_key(voxel, ao_mask);

                row_mask |= 1u32 << z;
            }
            masks[y] = row_mask;
        }
        // RIGHT:
        //
        // greedy 2D coordinates:
        // row    = Y
        // column = Z
        //
        // AO corner mapping:
        //
        // top-left     = AO 1
        // top-right    = AO 0
        // bottom-right = AO 3
        // bottom-left  = AO 2
        greedy_merge_rows(
            masks,
            face_keys,
            AoCornerMap {
                top_left: 1,
                top_right: 0,
                bottom_right: 3,
                bottom_left: 2,
            },
            |y_start, z_start, width, depth, corner_keys| {
                let min_z = z_start as f32 - 0.5;
                let max_z = min_z + width as f32;

                let min_y = y_start as f32 - 0.5;
                let max_y = min_y + depth as f32;

                let fx = x as f32 + 0.5;
                let base = *vertex_offset;

                positions.extend_from_slice(&[
                    [fx, min_y, max_z],
                    [fx, min_y, min_z],
                    [fx, max_y, min_z],
                    [fx, max_y, max_z],
                ]);

                normals.extend_from_slice(&[RIGHT_NORMAL; 4]);

                let material = unpack_material(corner_keys[0]);

                let voxel = match material {
                    0 => VoxelType::Grass,
                    1 => VoxelType::Snow,
                    2 => VoxelType::Sand,
                    _ => unreachable!(),
                };

                colors.extend_from_slice(&[voxel.face_color(); 4]);

                let ao0 = unpack_ao_corner(corner_keys[1], 0);
                let ao1 = unpack_ao_corner(corner_keys[0], 1);
                let ao2 = unpack_ao_corner(corner_keys[3], 2);
                let ao3 = unpack_ao_corner(corner_keys[2], 3);

                ao.extend_from_slice(&[ao0, ao1, ao2, ao3]);
                let tl = ao0 as i32;
                let tr = ao1 as i32;
                let br = ao2 as i32;
                let bl = ao3 as i32;

                if tl + br > tr + bl {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 3,

                        base + 1,
                        base + 2,
                        base + 3,
                    ]);
                } else {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,

                        base,
                        base + 2,
                        base + 3,
                    ]);
                }

                *vertex_offset += 4;
            },
        );
        //println!("positions: {}, normals: {}, colors: {}, ao: {}", positions.len(), normals.len(), colors.len(), ao.len());
    }
}
pub fn mesh_left_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Y],
    face_keys: &mut [[u16; CHUNK_Z]; CHUNK_Y],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    ao: &mut Vec<Ao>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for x in 0..CHUNK_X {
        masks.fill(0);
        face_keys.fill([0u16; CHUNK_Z]);
        for y in 0..CHUNK_Y {
            let mut row_mask = 0u32;

            for z in 0..CHUNK_Z {
                let px = x + 1;
                let py = y + 1;
                let pz = z + 1;

                let voxel = padded[padded_index(px, py, pz)];

                if voxel == VoxelType::Air
                    || is_solid(padded, px - 1, py, pz)
                {
                    continue;
                }

                let ao0 = vertex_ao(
                    is_solid(padded, px - 1, py - 1, pz),
                    is_solid(padded, px - 1, py, pz - 1),
                    is_solid(padded, px - 1, py - 1, pz - 1),
                );

                let ao1 = vertex_ao(
                    is_solid(padded, px - 1, py - 1, pz),
                    is_solid(padded, px - 1, py, pz + 1),
                    is_solid(padded, px - 1, py - 1, pz + 1),
                );

                let ao2 = vertex_ao(
                    is_solid(padded, px - 1, py + 1, pz),
                    is_solid(padded, px - 1, py, pz + 1),
                    is_solid(padded, px - 1, py + 1, pz + 1),
                );

                let ao3 = vertex_ao(
                    is_solid(padded, px - 1, py + 1, pz),
                    is_solid(padded, px - 1, py, pz - 1),
                    is_solid(padded, px - 1, py + 1, pz - 1),
                );

                let ao_mask =
                    (ao0 << 0)
                        |   (ao1 << 2)
                        |   (ao2 << 4)
                        |   (ao3 << 6);

                face_keys[y][z] = pack_face_key(voxel, ao_mask);

                row_mask |= 1u32 << z;
            }
            masks[y] = row_mask;
        }
        // LEFT:
        //
        // greedy 2D coordinates:
        // row    = Y
        // column = Z
        //
        // AO corner mapping:
        //
        // top-left     = AO 0
        // top-right    = AO 1
        // bottom-right = AO 2
        // bottom-left  = AO 3
        greedy_merge_rows(
            masks,
            face_keys,
            AoCornerMap {
                top_left: 0,
                top_right: 1,
                bottom_right: 2,
                bottom_left: 3,
            },
            |y_start, z_start, width, depth, corner_keys| {
                let min_z = z_start as f32 - 0.5;
                let max_z = min_z + width as f32;

                let min_y = y_start as f32 - 0.5;
                let max_y = min_y + depth as f32;

                let fx = x as f32 - 0.5;
                let base = *vertex_offset;

                positions.extend_from_slice(&[
                    [fx, min_y, min_z],
                    [fx, min_y, max_z],
                    [fx, max_y, max_z],
                    [fx, max_y, min_z],
                ]);

                normals.extend_from_slice(&[LEFT_NORMAL; 4]);

                let material = unpack_material(corner_keys[0]);

                let voxel = match material {
                    0 => VoxelType::Grass,
                    1 => VoxelType::Snow,
                    2 => VoxelType::Sand,
                    _ => unreachable!(),
                };

                colors.extend_from_slice(&[voxel.face_color(); 4]);

                let ao0 = unpack_ao_corner(corner_keys[0], 0);
                let ao1 = unpack_ao_corner(corner_keys[1], 1);
                let ao2 = unpack_ao_corner(corner_keys[2], 2);
                let ao3 = unpack_ao_corner(corner_keys[3], 3);

                ao.extend_from_slice(&[ao0, ao1, ao2, ao3]);
                let tl = ao0 as i32;
                let tr = ao1 as i32;
                let br = ao2 as i32;
                let bl = ao3 as i32;

                if tl + br > tr + bl {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 3,

                        base + 1,
                        base + 2,
                        base + 3,
                    ]);
                } else {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,

                        base,
                        base + 2,
                        base + 3,
                    ]);
                }

                *vertex_offset += 4;
            },
        );
        //println!("positions: {}, normals: {}, colors: {}, ao: {}", positions.len(), normals.len(), colors.len(), ao.len());
    }
}
pub fn mesh_top_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Z],
    face_keys: &mut [[u16; CHUNK_X]; CHUNK_Z],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    ao: &mut Vec<Ao>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for y in 0..CHUNK_Y {
        masks.fill(0);
        face_keys.fill([0u16; CHUNK_X]);
        for z in 0..CHUNK_Z {
            let mut row_mask = 0u32;

            for x in 0..CHUNK_X {
                let px = x + 1;
                let py = y + 1;
                let pz = z + 1;

                let voxel = padded[padded_index(px, py, pz)];

                if voxel == VoxelType::Air
                    || is_solid(padded, px, py + 1, pz)
                {
                    continue;
                }

                let ao0 = vertex_ao(
                    is_solid(padded, px - 1, py + 1, pz),
                    is_solid(padded, px,       py + 1, pz + 1),
                    is_solid(padded, px - 1, py + 1, pz + 1),
                );

                let ao1 = vertex_ao(
                    is_solid(padded, px + 1, py + 1, pz),
                    is_solid(padded, px,        py + 1, pz + 1),
                    is_solid(padded, px + 1, py + 1, pz + 1),
                );

                let ao2 = vertex_ao(
                    is_solid(padded, px + 1, py + 1, pz),
                    is_solid(padded, px,       py + 1, pz - 1),
                    is_solid(padded, px + 1, py + 1, pz - 1),
                );

                let ao3 = vertex_ao(
                    is_solid(padded, px - 1, py + 1, pz),
                    is_solid(padded, px,      py + 1, pz - 1),
                    is_solid(padded, px - 1, py + 1, pz - 1),
                );

                let ao_mask =
                    (ao0 << 0)
                        |   (ao1 << 2)
                        |   (ao2 << 4)
                        |   (ao3 << 6);

                face_keys[z][x] = pack_face_key(voxel, ao_mask);

                row_mask |= 1u32 << x;
            }
            masks[z] = row_mask;
        }
        // TOP:
        //
        // greedy 2D coordinates:
        // row    = Z
        // column = X
        //
        // AO corner mapping:
        //
        // top-left     = AO 3
        // top-right    = AO 2
        // bottom-right = AO 1
        // bottom-left  = AO 0
        greedy_merge_rows(
            masks,
            face_keys,
            AoCornerMap {
                top_left: 3,
                top_right: 2,
                bottom_right: 1,
                bottom_left: 0,
            },
            |z_start, x_start, width, depth, corner_keys| {
                let min_x = x_start as f32 - 0.5;
                let max_x = min_x + width as f32;

                let min_z = z_start as f32 - 0.5;
                let max_z = min_z + depth as f32;

                let fy = y as f32 + 0.5;
                let base = *vertex_offset;

                positions.extend_from_slice(&[
                    [min_x, fy, max_z],
                    [max_x, fy, max_z],
                    [max_x, fy, min_z],
                    [min_x, fy, min_z],
                ]);

                normals.extend_from_slice(&[TOP_NORMAL; 4]);

                let material = unpack_material(corner_keys[0]);

                let voxel = match material {
                    0 => VoxelType::Grass,
                    1 => VoxelType::Snow,
                    2 => VoxelType::Sand,
                    _ => unreachable!(),
                };

                colors.extend_from_slice(&[voxel.face_color(); 4]);

                let ao0 = unpack_ao_corner(corner_keys[3], 0);
                let ao1 = unpack_ao_corner(corner_keys[2], 1);
                let ao2 = unpack_ao_corner(corner_keys[1], 2);
                let ao3 = unpack_ao_corner(corner_keys[0], 3);

                ao.extend_from_slice(&[ao0, ao1, ao2, ao3]);
                let tl = ao0 as i32;
                let tr = ao1 as i32;
                let br = ao2 as i32;
                let bl = ao3 as i32;

                if tl + br > tr + bl {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 3,

                        base + 1,
                        base + 2,
                        base + 3,
                    ]);
                } else {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,

                        base,
                        base + 2,
                        base + 3,
                    ]);
                }

                *vertex_offset += 4;
            },
        );
        //println!("positions: {}, normals: {}, colors: {}, ao: {}", positions.len(), normals.len(), colors.len(), ao.len());
    }
}
pub fn mesh_bottom_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Z],
    face_keys: &mut [[u16; CHUNK_X]; CHUNK_Z],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    ao: &mut Vec<Ao>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for y in 0..CHUNK_Y {
        masks.fill(0);
        face_keys.fill([0u16; CHUNK_X]);
        for z in 0..CHUNK_Z {
            let mut row_mask = 0u32;

            for x in 0..CHUNK_X {
                let px = x + 1;
                let py = y + 1;
                let pz = z + 1;

                let voxel = padded[padded_index(px, py, pz)];

                if voxel == VoxelType::Air
                    || is_solid(padded, px, py - 1, pz)
                {
                    continue;
                }

                let ao0 = vertex_ao(
                    is_solid(padded, px - 1, py - 1, pz),
                    is_solid(padded, px, py - 1, pz - 1),
                    is_solid(padded, px - 1, py - 1, pz - 1),
                );

                let ao1 = vertex_ao(
                    is_solid(padded, px + 1, py - 1, pz),
                    is_solid(padded, px, py - 1, pz - 1),
                    is_solid(padded, px + 1, py - 1, pz - 1),
                );

                let ao2 = vertex_ao(
                    is_solid(padded, px + 1, py - 1, pz),
                    is_solid(padded, px, py - 1, pz + 1),
                    is_solid(padded, px + 1, py - 1, pz + 1),
                );

                let ao3 = vertex_ao(
                    is_solid(padded, px - 1, py - 1, pz),
                    is_solid(padded, px, py - 1, pz + 1),
                    is_solid(padded, px - 1, py - 1, pz + 1),
                );

                let ao_mask =
                    (ao0 << 0)
                        |   (ao1 << 2)
                        |   (ao2 << 4)
                        |   (ao3 << 6);

                face_keys[z][x] = pack_face_key(voxel, ao_mask);

                row_mask |= 1u32 << x;
            }
            masks[z] = row_mask;
        }
        // BOTTOM:
        //
        // greedy 2D coordinates:
        // row    = Z
        // column = X
        //
        // AO corner mapping:
        //
        // top-left     = AO 0
        // top-right    = AO 1
        // bottom-right = AO 2
        // bottom-left  = AO 3
        greedy_merge_rows(
            masks,
            face_keys,
            AoCornerMap {
                top_left: 0,
                top_right: 1,
                bottom_right: 2,
                bottom_left: 3,
            },
            |z_start, x_start, width, depth, corner_keys| {
                let min_x = x_start as f32 - 0.5;
                let max_x = min_x + width as f32;

                let min_z = z_start as f32 - 0.5;
                let max_z = min_z + depth as f32;

                let fy = y as f32 - 0.5;
                let base = *vertex_offset;

                positions.extend_from_slice(&[
                    [min_x, fy, min_z],
                    [max_x, fy, min_z],
                    [max_x, fy, max_z],
                    [min_x, fy, max_z],
                ]);

                normals.extend_from_slice(&[BOTTOM_NORMAL; 4]);

                let material = unpack_material(corner_keys[0]);

                let voxel = match material {
                    0 => VoxelType::Grass,
                    1 => VoxelType::Snow,
                    2 => VoxelType::Sand,
                    _ => unreachable!(),
                };

                colors.extend_from_slice(&[voxel.face_color(); 4]);

                let ao0 = unpack_ao_corner(corner_keys[0], 0);
                let ao1 = unpack_ao_corner(corner_keys[1], 1);
                let ao2 = unpack_ao_corner(corner_keys[2], 2);
                let ao3 = unpack_ao_corner(corner_keys[3], 3);

                ao.extend_from_slice(&[ao0, ao1, ao2, ao3]);
                let tl = ao0 as i32;
                let tr = ao1 as i32;
                let br = ao2 as i32;
                let bl = ao3 as i32;

                if tl + br > tr + bl {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 3,

                        base + 1,
                        base + 2,
                        base + 3,
                    ]);
                } else {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,

                        base,
                        base + 2,
                        base + 3,
                    ]);
                }

                *vertex_offset += 4;
            },
        );
        //println!("positions: {}, normals: {}, colors: {}, ao: {}", positions.len(), normals.len(), colors.len(), ao.len());
    }
}

pub fn mesh_front_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Y],
    face_keys: &mut [[u16; CHUNK_X]; CHUNK_Y],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    ao: &mut Vec<Ao>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for z in 0..CHUNK_Z {
        masks.fill(0);
        face_keys.fill([0u16; CHUNK_X]);
        for y in 0..CHUNK_Y {
            let mut row_mask = 0u32;

            for x in 0..CHUNK_X {
                let px = x + 1;
                let py = y + 1;
                let pz = z + 1;

                let voxel = padded[padded_index(px, py, pz)];

                if voxel == VoxelType::Air
                    || is_solid(padded, px, py, pz - 1)
                {
                    continue;
                }

                let ao0 = vertex_ao(
                    is_solid(padded, px + 1, py, pz - 1),
                    is_solid(padded, px, py - 1, pz - 1),
                    is_solid(padded, px + 1, py - 1, pz - 1),
                );

                let ao1 = vertex_ao(
                    is_solid(padded, px - 1, py, pz - 1),
                    is_solid(padded, px, py - 1, pz - 1),
                    is_solid(padded, px - 1, py - 1, pz - 1),
                );

                let ao2 = vertex_ao(
                    is_solid(padded, px - 1, py, pz - 1),
                    is_solid(padded, px, py + 1, pz - 1),
                    is_solid(padded, px - 1, py + 1, pz - 1),
                );

                let ao3 = vertex_ao(
                    is_solid(padded, px + 1, py, pz - 1),
                    is_solid(padded, px, py + 1, pz - 1),
                    is_solid(padded, px + 1, py + 1, pz - 1),
                );

                let ao_mask =
                    (ao0 << 0)
                        |   (ao1 << 2)
                        |   (ao2 << 4)
                        |   (ao3 << 6);

                face_keys[y][x] = pack_face_key(voxel, ao_mask);

                row_mask |= 1u32 << x;
            }
            masks[y] = row_mask;
        }
        // FRONT:
        //
        // greedy 2D coordinates:
        // row    = Y
        // column = X
        //
        // AO corner mapping:
        //
        // top-left     = AO 1
        // top-right    = AO 0
        // bottom-right = AO 3
        // bottom-left  = AO 2
        greedy_merge_rows(
            masks,
            face_keys,
            AoCornerMap {
                top_left: 1,
                top_right: 0,
                bottom_right: 3,
                bottom_left: 2,
            },
            |y_start, x_start, width, depth, corner_keys| {
                let min_x = x_start as f32 - 0.5;
                let max_x = min_x + width as f32;

                let min_y = y_start as f32 - 0.5;
                let max_y = min_y + depth as f32;

                let fz = z as f32 - 0.5;
                let base = *vertex_offset;

                positions.extend_from_slice(&[
                    [max_x, min_y, fz],
                    [min_x, min_y, fz],
                    [min_x, max_y, fz],
                    [max_x, max_y, fz],
                ]);

                normals.extend_from_slice(&[FRONT_NORMAL; 4]);

                let material = unpack_material(corner_keys[0]);

                let voxel = match material {
                    0 => VoxelType::Grass,
                    1 => VoxelType::Snow,
                    2 => VoxelType::Sand,
                    _ => unreachable!(),
                };

                colors.extend_from_slice(&[voxel.face_color(); 4]);

                let ao0 = unpack_ao_corner(corner_keys[1], 0);
                let ao1 = unpack_ao_corner(corner_keys[0], 1);
                let ao2 = unpack_ao_corner(corner_keys[3], 2);
                let ao3 = unpack_ao_corner(corner_keys[2], 3);

                ao.extend_from_slice(&[ao0, ao1, ao2, ao3]);
                let tl = ao0 as i32;
                let tr = ao1 as i32;
                let br = ao2 as i32;
                let bl = ao3 as i32;

                if tl + br > tr + bl {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 3,

                        base + 1,
                        base + 2,
                        base + 3,
                    ]);
                } else {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,

                        base,
                        base + 2,
                        base + 3,
                    ]);
                }

                *vertex_offset += 4;
            },
        );
        //println!("positions: {}, normals: {}, colors: {}, ao: {}", positions.len(), normals.len(), colors.len(), ao.len());
    }
}

pub fn mesh_back_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Y],
    face_keys: &mut [[u16; CHUNK_X]; CHUNK_Y],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    ao: &mut Vec<Ao>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for z in 0..CHUNK_Z {
        masks.fill(0);
        face_keys.fill([0u16; CHUNK_X]);
        for y in 0..CHUNK_Y {
            let mut row_mask = 0u32;

            for x in 0..CHUNK_X {
                let px = x + 1;
                let py = y + 1;
                let pz = z + 1;

                let voxel = padded[padded_index(px, py, pz)];

                if voxel == VoxelType::Air
                    || is_solid(padded, px, py, pz + 1)
                {
                    continue;
                }

                let ao0 = vertex_ao(
                    is_solid(padded, px - 1, py + 1, pz + 1),
                    is_solid(padded, px, py + 1, pz + 1),
                    is_solid(padded, px - 1, py + 1, pz + 1),
                );

                let ao1 = vertex_ao(
                    is_solid(padded, px + 1, py + 1, pz + 1),
                    is_solid(padded, px, py + 1, pz + 1),
                    is_solid(padded, px + 1, py + 1, pz + 1),
                );

                let ao2 = vertex_ao(
                    is_solid(padded, px + 1, py + 1, pz + 1),
                    is_solid(padded, px, py + 1, pz + 1),
                    is_solid(padded, px + 1, py + 1, pz + 1),
                );

                let ao3 = vertex_ao(
                    is_solid(padded, px - 1, py + 1, pz + 1),
                    is_solid(padded, px, py + 1, pz + 1),
                    is_solid(padded, px - 1, py + 1, pz + 1),
                );

                let ao_mask =
                    (ao0 << 0)
                        |   (ao1 << 2)
                        |   (ao2 << 4)
                        |   (ao3 << 6);

                face_keys[y][x] = pack_face_key(voxel, ao_mask);

                row_mask |= 1u32 << x;
            }
            masks[y] = row_mask;
        }
        // BACK:
        //
        // greedy 2D coordinates:
        // row    = Y
        // column = X
        //
        // AO corner mapping:
        //
        // top-left     = AO 3
        // top-right    = AO 2
        // bottom-right = AO 1
        // bottom-left  = AO 0
        greedy_merge_rows(
            masks,
            face_keys,
            AoCornerMap {
                top_left: 3,
                top_right: 2,
                bottom_right: 1,
                bottom_left: 0,
            },
            |y_start, x_start, width, depth, corner_keys| {
                let min_x = x_start as f32 - 0.5;
                let max_x = min_x + width as f32;

                let min_y = y_start as f32 - 0.5;
                let max_y = min_y + depth as f32;

                let fz = z as f32 + 0.5;
                let base = *vertex_offset;

                positions.extend_from_slice(&[
                    [min_x, min_y, fz],
                    [max_x, min_y, fz],
                    [max_x, max_y, fz],
                    [min_x, max_y, fz],
                ]);

                normals.extend_from_slice(&[BACK_NORMAL; 4]);

                let material = unpack_material(corner_keys[0]);

                let voxel = match material {
                    0 => VoxelType::Grass,
                    1 => VoxelType::Snow,
                    2 => VoxelType::Sand,
                    _ => unreachable!(),
                };

                colors.extend_from_slice(&[voxel.face_color(); 4]);

                let ao0 = unpack_ao_corner(corner_keys[0], 3);
                let ao1 = unpack_ao_corner(corner_keys[1], 2);
                let ao2 = unpack_ao_corner(corner_keys[2], 1);
                let ao3 = unpack_ao_corner(corner_keys[3], 0);

                ao.extend_from_slice(&[ao0, ao1, ao2, ao3]);
                let tl = ao0 as i32;
                let tr = ao1 as i32;
                let br = ao2 as i32;
                let bl = ao3 as i32;

                if tl + br > tr + bl {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 3,

                        base + 1,
                        base + 2,
                        base + 3,
                    ]);
                } else {
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,

                        base,
                        base + 2,
                        base + 3,
                    ]);
                }

                *vertex_offset += 4;
            },
        );
        //println!("positions: {}, normals: {}, colors: {}, ao: {}", positions.len(), normals.len(), colors.len(), ao.len());
    }
}

#[inline]
fn vertex_ao(
    side1: bool,
    side2: bool,
    corner: bool,
) -> u8 {
    if side1 && side2 {
        3
    } else {
        side1 as u8 + side2 as u8 + corner as u8
    }
}

#[inline]
fn is_solid(
    padded: &[VoxelType],
    x: usize,
    y: usize,
    z: usize,
) -> bool {
    padded[padded_index(x, y, z)] != VoxelType::Air
}

#[inline]
fn material_id(voxel: VoxelType) -> u16 {
    match voxel {
        VoxelType::Grass => 0,
        VoxelType::Snow  => 1,
        VoxelType::Sand  => 2,
        VoxelType::Air   => unreachable!(),
    }
}

#[inline]
fn pack_face_key(voxel: VoxelType, ao_mask: u8) -> u16 {
    material_id(voxel) | ((ao_mask as u16) << 2)
}

#[inline]
fn unpack_ao_corner(key: u16, corner: usize) -> u8 {
    ((unpack_ao(key) >> (corner * 2)) & 0b11) as u8
}

#[inline]
fn unpack_material(key: u16) -> u16 {
    key & 0b11
}

#[inline]
fn unpack_ao(key: u16) -> u8 {
    (key >> 2) as u8
}

#[derive(Clone, Copy)]
struct AoCornerMap {
    // AO corner indices for:
    //
    // top-left
    // top-right
    // bottom-right
    // bottom-left
    //
    // in the 2D row/column space used by greedy_merge_rows.
    top_left: usize,
    top_right: usize,
    bottom_right: usize,
    bottom_left: usize,
}
#[inline(always)]
fn material_of(key: u16) -> u16 {
    key & 0b11
}

fn greedy_merge_rows<const D1: usize, const D2: usize>(
    masks: &mut [u32],
    face_keys: &[[u16; D2]; D1],
    ao_map: AoCornerMap,
    mut emit_quad: impl FnMut(
        usize,
        usize,
        usize,
        usize,
        [u16; 4],
    ),
) {
    let same_material =
        |row: usize,
         col0: usize,
         width: usize,
         depth: usize|
         -> bool {
            let anchor =
                material_of(face_keys[row][col0]);

            (row..row + depth).all(|r| {
                (col0..col0 + width).all(|c| {
                    material_of(face_keys[r][c])
                        == anchor
                })
            })
        };

    let has_uniform_ao =
        |row: usize,
         col0: usize,
         width: usize,
         depth: usize|
         -> bool {
            let expected = unpack_ao_corner(
                face_keys[row][col0],
                ao_map.top_left,
            );

            for r in row..row + depth {
                for c in col0..col0 + width {
                    let key = face_keys[r][c];

                    if unpack_ao_corner(
                        key,
                        ao_map.top_left,
                    ) != expected
                    {
                        return false;
                    }

                    if unpack_ao_corner(
                        key,
                        ao_map.top_right,
                    ) != expected
                    {
                        return false;
                    }

                    if unpack_ao_corner(
                        key,
                        ao_map.bottom_right,
                    ) != expected
                    {
                        return false;
                    }

                    if unpack_ao_corner(
                        key,
                        ao_map.bottom_left,
                    ) != expected
                    {
                        return false;
                    }
                }
            }

            true
        };

    for row in 0..masks.len() {
        while masks[row] != 0 {
            let column_start =
                masks[row].trailing_zeros() as usize;

            let mut width = 1;

            let anchor_uniform = has_uniform_ao(
                row,
                column_start,
                1,
                1,
            );

            if anchor_uniform {
                while column_start + width < D2 {
                    let next_bit =
                        1u32 << (column_start + width);

                    if masks[row] & next_bit == 0 {
                        break;
                    }

                    let new_width = width + 1;

                    if !same_material(
                        row,
                        column_start,
                        new_width,
                        1,
                    ) {
                        break;
                    }

                    if !has_uniform_ao(
                        row,
                        column_start,
                        new_width,
                        1,
                    ) {
                        break;
                    }

                    width = new_width;
                }
            }

            let width_mask =
                if width == 32 {
                    u32::MAX
                } else {
                    ((1u32 << width) - 1)
                        << column_start
                };

            let mut depth = 1;

            if anchor_uniform {
                while row + depth < D1 {
                    let next_row = row + depth;

                    if (masks[next_row] & width_mask)
                        != width_mask
                    {
                        break;
                    }

                    let new_depth = depth + 1;

                    if !same_material(
                        row,
                        column_start,
                        width,
                        new_depth,
                    ) {
                        break;
                    }

                    if !has_uniform_ao(
                        row,
                        column_start,
                        width,
                        new_depth,
                    ) {
                        break;
                    }

                    depth = new_depth;
                }
            }

            for d in 0..depth {
                masks[row + d] &= !width_mask;
            }

            let top_left =
                face_keys[row][column_start];

            let top_right =
                face_keys[row]
                    [column_start + width - 1];

            let bottom_right =
                face_keys[row + depth - 1]
                    [column_start + width - 1];

            let bottom_left =
                face_keys[row + depth - 1]
                    [column_start];

            emit_quad(
                row,
                column_start,
                width,
                depth,
                [
                    top_left,
                    top_right,
                    bottom_right,
                    bottom_left,
                ],
            );
        }
    }
}