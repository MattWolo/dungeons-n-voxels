use bevy::mesh::{Indices, Mesh};
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
const EXPECTED_VERTICES: usize = 2048;
const EXPECTED_INDICES: usize = 2048;

pub struct ChunkMeshScratch {
    pub own_flat: Vec<VoxelType>,
    pub neighbors: [Vec<VoxelType>; 4],
    pub padded: Vec<VoxelType>,

    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,

    pub mask_y: [u32; CHUNK_Y],
    pub mask_z: [u32; CHUNK_Z],
}

impl ChunkMeshScratch {
    pub fn new() -> Self {
        Self {
            own_flat: Vec::with_capacity(CHUNK_VOLUME),
            neighbors: std::array::from_fn(|_| Vec::with_capacity(CHUNK_VOLUME)),
            padded: vec![VoxelType::Air; PADDED_X * PADDED_Y * PADDED_Z],
            positions: Vec::with_capacity(EXPECTED_VERTICES),
            normals: Vec::with_capacity(EXPECTED_VERTICES),
            colors: Vec::with_capacity(EXPECTED_VERTICES),
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
        self.positions.clear();
        self.normals.clear();
        self.colors.clear();
        self.indices.clear();

        self.positions.reserve(EXPECTED_VERTICES);
        self.normals.reserve(EXPECTED_VERTICES);
        self.colors.reserve(EXPECTED_VERTICES);
        self.indices.reserve(EXPECTED_INDICES);

        self.mask_y.fill(0);
        self.mask_z.fill(0);
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
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.indices,
        &mut vertex_offset
    );
    mesh_left_faces_binary(
        &scratch.padded,
        &mut scratch.mask_y,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.indices,
        &mut vertex_offset
    );
    mesh_top_faces_binary(
        &scratch.padded,
        &mut scratch.mask_z,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.indices,
        &mut vertex_offset
    );

    mesh_bottom_faces_binary(
        &scratch.padded,
        &mut scratch.mask_z,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.indices,
        &mut vertex_offset
    );

    mesh_front_faces_binary(
        &scratch.padded,
        &mut scratch.mask_y,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.indices,
        &mut vertex_offset
    );

    mesh_back_faces_binary(
        &scratch.padded,
        &mut scratch.mask_y,
        &mut scratch.positions,
        &mut scratch.normals,
        &mut scratch.colors,
        &mut scratch.indices,
        &mut vertex_offset
    );

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, std::mem::take(&mut scratch.positions));
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, std::mem::take(&mut scratch.normals));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, std::mem::take(&mut scratch.colors));
    mesh.insert_indices(Indices::U32(std::mem::take(&mut scratch.indices)));

    mesh
}

pub fn mesh_right_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Y],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for x in 0..CHUNK_X{
        masks.fill(0);
        for y in 0..CHUNK_Y {
            let mut row_mask = 0u32;
            for z in 0..CHUNK_Z{
                let current = padded[padded_index(x + 1, y + 1, z + 1)];
                let neighbor_right = padded[padded_index(x + 2, y + 1, z + 1)];
                if current != VoxelType::Air && neighbor_right == VoxelType::Air {
                    row_mask |= 1 << z;
                }
            }
            masks[y] = row_mask;
        }

        greedy_merge_rows(masks, |y_start, z_start, width, depth| {
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

            let voxel = padded[padded_index(x + 1, y_start + 1, z_start + 1)];
            colors.extend_from_slice(&[voxel.face_color(); 4]);

            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);

            *vertex_offset += 4;
        });
    }
}

pub fn mesh_left_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Y],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for x in 0..CHUNK_X{
        masks.fill(0);
        for y in 0..CHUNK_Y {
            let mut row_mask = 0u32;
            for z in 0..CHUNK_Z{
                let current = padded[padded_index(x + 1, y + 1, z + 1)];
                let neighbor_right = padded[padded_index(x, y + 1, z + 1)];
                if current != VoxelType::Air && neighbor_right == VoxelType::Air {
                    row_mask |= 1 << z;
                }
            }
            masks[y] = row_mask;
        }

        greedy_merge_rows(masks, |y_start, z_start, width, depth| {
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

            let voxel = padded[padded_index(x + 1, y_start + 1, z_start + 1)];
            colors.extend_from_slice(&[voxel.face_color(); 4]);

            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);

            *vertex_offset += 4;
        });
    }
}
pub fn mesh_top_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Z],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for y in 0..CHUNK_Y {
        masks.fill(0);
        for z in 0..CHUNK_Z {
            let mut row_mask = 0u32;
            for x in 0..CHUNK_X {
                let current = padded[padded_index(x + 1, y + 1, z + 1)];
                let neighbor_above = padded[padded_index(x + 1, y + 2, z + 1)];
                if current != VoxelType::Air && neighbor_above == VoxelType::Air {
                    row_mask |= 1 << x;
                }
            }
            masks[z] = row_mask;
        }

        greedy_merge_rows(masks, |z, x_start, width, depth| {
            let min_x = x_start as f32 - 0.5;
            let max_x = min_x + width as f32;
            let min_z = z as f32 - 0.5;
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

            let voxel = padded[padded_index(x_start + 1, y + 1, z + 1)];
            colors.extend_from_slice(&[voxel.face_color(); 4]);

            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);

            *vertex_offset += 4;
        });
    }
}
pub fn mesh_bottom_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Z],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for y in 0..CHUNK_Y{
        masks.fill(0);
        for z in 0..CHUNK_Z {
            let mut row_mask = 0u32;
            for x in 0..CHUNK_X{
                let current = padded[padded_index(x + 1, y + 1, z + 1)];
                let neighbor_below = padded[padded_index(x + 1, y, z + 1)];
                if current != VoxelType::Air && neighbor_below == VoxelType::Air && y > 0 {
                    row_mask |= 1 << x;
                }
            }
            masks[z] = row_mask;
        }

        greedy_merge_rows(masks, |z, x_start, width, depth| {
            let min_x = x_start as f32 - 0.5;
            let max_x = min_x + width as f32;
            let min_z = z as f32 - 0.5;
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

            let voxel = padded[padded_index(x_start + 1, y + 1, z + 1)];
            colors.extend_from_slice(&[voxel.face_color(); 4]);

            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);

            *vertex_offset += 4;
        });

    }
}

pub fn mesh_front_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Y],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for z in 0..CHUNK_Z{
        masks.fill(0);
        for y in 0..CHUNK_Y {
            let mut row_mask = 0u32;
            for x in 0..CHUNK_X{
                let current = padded[padded_index(x + 1, y + 1, z + 1)];
                let neighbor_front = padded[padded_index(x + 1, y + 1, z)];
                if current != VoxelType::Air && neighbor_front == VoxelType::Air {
                    row_mask |= 1 << x;
                }
            }
            masks[y] = row_mask;
        }

        greedy_merge_rows(masks, |y_start, x_start, width, depth| {
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

            let voxel = padded[padded_index(x_start + 1, y_start + 1, z + 1)];
            colors.extend_from_slice(&[voxel.face_color(); 4]);

            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);

            *vertex_offset += 4;
        });
    }
}

pub fn mesh_back_faces_binary(
    padded: &[VoxelType],
    masks: &mut [u32; CHUNK_Y],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
) {
    for z in 0..CHUNK_Z{
        masks.fill(0);
        for y in 0..CHUNK_Y {
            let mut row_mask = 0u32;
            for x in 0..CHUNK_X{
                let current = padded[padded_index(x + 1, y + 1, z + 1)];
                let neighbor_back = padded[padded_index(x + 1, y + 1, z + 2)];
                if current != VoxelType::Air && neighbor_back == VoxelType::Air {
                    row_mask |= 1 << x;
                }
            }
            masks[y] = row_mask;
        }

        greedy_merge_rows(masks, |y_start, x_start, width, depth| {
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

            let voxel = padded[padded_index(x_start + 1, y_start + 1, z + 1)];
            colors.extend_from_slice(&[voxel.face_color(); 4]);

            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);

            *vertex_offset += 4;
        });
    }
}
fn greedy_merge_rows(masks: &mut [u32], mut emit_quad: impl FnMut(usize, usize, usize, usize)) {
    for row in 0..masks.len() {
        while masks[row] != 0 {
            let x_start = masks[row].trailing_zeros() as usize;
            let remaining = masks[row] >> x_start;
            let width = remaining.trailing_ones() as usize;
            let width_mask = if width == 32 { !0u32 } else { (1u32 << width) - 1 } << x_start;
            let mut depth = 1;

            while row + depth < masks.len() && (masks[row + depth] & width_mask) == width_mask {
                depth += 1;
            }
            for d in 0..depth {
                masks[row + d] &= !width_mask;
            }

            emit_quad(row, x_start, width, depth);
        }
    }
}