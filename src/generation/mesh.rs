use bevy::mesh::{Indices, Mesh};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::asset::RenderAssetUsages;
use super::types::*;
use super::chunk::{padded_index, Chunk};

const DIRECTIONS: [(i32, i32, i32); 6] = [
    ( 1, 0, 0), (-1,  0,  0),
    ( 0, 1, 0), ( 0, -1,  0),
    ( 0, 0, 1), ( 0,  0, -1),
];

const FACE_VERTICES: [[[f32; 3]; 4]; 6] = [
    // right (+x) -> Looking along -x: +z is left, -z is right
    [[0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5]],
    // left (-x) -> Looking along +x: -z is left, +z is right
    [[-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5]],
    // top (+y) -> Looking along -y: -x is left, +x is right
    [[-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5]],
    // bottom (-y) -> Looking along +y: -x is left, +x is right
    [[-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5]],
    // back (+z) -> Looking along -z: +x is left, -x is right
    [[0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, -0.5, 0.5]],
    // forward (-z) -> Looking along +z: -x is left, +x is right
    [[-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, -0.5, -0.5]],
];

const FACE_NORMALS: [[f32; 3]; 6] = [
    [ 1.0,  0.0,  0.0],   // +x
    [-1.0,  0.0,  0.0],   // -x
    [ 0.0,  1.0,  0.0],   // +y
    [ 0.0, -1.0,  0.0],   // -y
    [ 0.0,  0.0,  1.0],   // +z
    [ 0.0,  0.0, -1.0],   // -z
];

// #[inline(always)]
// fn is_transparent(flat: &[VoxelType], x: i32, y: i32, z: i32) -> bool {
//     // If neighbor coordinate is out of bounds, treat as Air so outer chunk faces render
//     if x < 0 || x >= CHUNK_X as i32 || y < 0 || y >= CHUNK_Y as i32 || z < 0 || z >= CHUNK_Z as i32 {
//         return true;
//     }
//     Chunk::get_voxel(flat, x as usize, y as usize, z as usize) == VoxelType::Air
// }

fn is_transparent_padded(padded: &[VoxelType], px: i32, py: i32, pz: i32) -> bool {
    if py < 0 || py >= CHUNK_Y as i32 {
        return true; // still a real edge — no vertical neighbor chunks exist
    }
    padded[padded_index(px as usize, py as usize, pz as usize)] == VoxelType::Air
}

pub fn create_chunk_mesh(chunk: &Chunk, padded: &[VoxelType]) -> Mesh{

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_offset: u32 = 0;

    let mut current_idx: usize = 0;
    for run in &chunk.runs {
        let run_len = run.length as usize;
        if run.value == VoxelType::Air {
            current_idx += run_len;
            continue;
        }

        let color = run.value.face_color();

        let mut y = current_idx % CHUNK_Y;
        let mut z = (current_idx / CHUNK_Y) % CHUNK_Z;
        let mut x = current_idx / (CHUNK_Y * CHUNK_Z);

        for _ in 0..run_len {
            let fx = x as f32;
            let fy = y as f32;
            let fz = z as f32;

            for (face_idx, (dx, dy, dz)) in DIRECTIONS.into_iter().enumerate() {
                if is_transparent_padded(padded, x as i32 + 1 + dx, y as i32 + dy, z as i32 + 1 + dz) {
                    let base = vertex_offset;
                    let v = FACE_VERTICES[face_idx];

                    positions.extend_from_slice(&[
                        [fx + v[0][0], fy + v[0][1], fz + v[0][2]],
                        [fx + v[1][0], fy + v[1][1], fz + v[1][2]],
                        [fx + v[2][0], fy + v[2][1], fz + v[2][2]],
                        [fx + v[3][0], fy + v[3][1], fz + v[3][2]],
                    ]);

                    normals.extend_from_slice(&[FACE_NORMALS[face_idx]; 4]);
                    colors.extend_from_slice(&[color; 4]);
                    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

                    vertex_offset += 4;
                }
            }

            y += 1;
            if y == CHUNK_Y {
                y = 0;
                z += 1;
                if z == CHUNK_Z {
                    z = 0;
                    x += 1;
                }
            }
            current_idx += 1;
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
}