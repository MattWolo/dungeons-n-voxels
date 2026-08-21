use bevy::mesh::{Indices, Mesh};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::asset::RenderAssetUsages;
use super::types::*;
use super::chunk::Chunk;

fn is_transparent(flat: &[VoxelType], x: i32, y: i32, z: i32) -> bool {
    // If neighbor coordinate is out of bounds, treat as Air so outer chunk faces render
    if x < 0 || x >= CHUNK_X as i32 || y < 0 || y >= CHUNK_Y as i32 || z < 0 || z >= CHUNK_Z as i32 {
        return true;
    }
    Chunk::get_voxel(flat, x as usize, y as usize, z as usize) == VoxelType::Air
}

pub(crate) fn create_chunk_mesh(chunk: &Chunk) -> Mesh{
    let voxels = chunk.decompress(); //Decompress to make O(1) face culling

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_offset: u32 = 0;

    let directions = [
        ( 1,  0,  0), (-1,  0,  0),
        ( 0,  1,  0), ( 0, -1,  0),
        ( 0,  0,  1), ( 0,  0, -1),
    ];

    let face_vertices: [[[f32; 3]; 4]; 6] = [
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

    let face_normals: [[f32; 3]; 6] = [
        [ 1.0,  0.0,  0.0],   // +x
        [-1.0,  0.0,  0.0],   // -x
        [ 0.0,  1.0,  0.0],   // +y
        [ 0.0, -1.0,  0.0],   // -y
        [ 0.0,  0.0,  1.0],   // +z
        [ 0.0,  0.0, -1.0],   // -z
    ];

    let face_tris = [0_u32, 1, 2, 0, 2, 3];

    let mut current_idx: usize = 0;
    for run in &chunk.runs {
        if run.value == VoxelType::Air {
            current_idx += run.length as usize;
            continue;
        }

        let color = run.value.face_color();

        for _ in 0..run.length{
            let y = current_idx % CHUNK_Y;
            let z = (current_idx / CHUNK_Y) % CHUNK_Z;
            let x = current_idx / (CHUNK_Y * CHUNK_Z);

            let fx = x as f32;
            let fy = y as f32;
            let fz = z as f32;

            for (face_idx, (dx, dy, dz)) in directions.iter().enumerate() {
                if is_transparent(&voxels, x as i32 + dx, y as i32 + dy, z as i32 + dz) {
                    let base = vertex_offset;

                    // push the 4 vertices of this face
                    for j in 0..4 {
                        let v = face_vertices[face_idx][j];
                        positions.push([fx + v[0], fy + v[1], fz + v[2]]);
                        normals.push(face_normals[face_idx]);
                        colors.push(color);
                    }

                    for &idx in &face_tris {
                        indices.push(base + idx);
                    }

                    vertex_offset += 4;
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