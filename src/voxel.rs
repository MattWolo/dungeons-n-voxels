use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

pub const CHUNK_X: usize = 32;
pub const CHUNK_Y: usize = 256;
pub const CHUNK_Z: usize = 32;
pub const CHUNK_VOLUME: usize = CHUNK_X * CHUNK_Y * CHUNK_Z;
pub struct VoxelPlugin;

#[derive(Clone, Copy, PartialEq, Debug)]
enum VoxelType{
    Air,
    Grass
}

impl VoxelType{
    pub fn face_color(&self) -> [f32;4]{
        match self{
            VoxelType::Grass => [0.2, 1.0, 0.2, 1.0],
            VoxelType::Air => [0.0, 0.0, 0.0, 0.0]
        }
    }
}

#[derive(Debug)]
struct Run {
    value: VoxelType,
    length: u16
}

struct Chunk{
    runs: Vec<Run>
}

fn compress_to_runs(flat: &[VoxelType]) -> Vec<Run>{
    flat.chunk_by(|a, b| a == b)
        .map(|chunk| Run{
            value: chunk[0],
            length: chunk.len() as u16,
        })
        .collect()
}

#[test]
fn test_compress() {
    let flat = vec![VoxelType::Air, VoxelType::Air, VoxelType::Air, VoxelType::Grass];
    let runs = compress_to_runs(&flat);
    println!("{:?}", runs);
}

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_chunk);
    }
}

impl Chunk {
    pub fn generate() -> Self{
        let mut flat = Vec::with_capacity(CHUNK_VOLUME);

        for _x in 0..CHUNK_X {
            for _z in 0..CHUNK_Z {
                for y in 0..CHUNK_Y {
                    let voxel = if y < 64 {
                        VoxelType::Grass
                    } else {
                        VoxelType::Air
                    };
                    flat.push(voxel);
                }
            }
        }

        Self {
            runs: compress_to_runs(&flat) //runs are RLE
        }
    }

    /// RLE runs into a flat slice for fast meshing lookup
    pub fn decompress(&self) -> Vec<VoxelType> {
        let mut flat = Vec::with_capacity(CHUNK_VOLUME);
        for run in &self.runs {
            for _ in 0..run.length {
                flat.push(run.value);
            }
        }
        flat
    }

    /// 3D coordinates into the flat 1D array index (X -> Z -> Y loop order)
    pub fn get_voxel(flat: &[VoxelType], x: usize, y: usize, z: usize) -> VoxelType {
        let index = x * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y;
        flat[index]
    }
}

/// Helper function to check if a neighbor position is Air or outside the chunk
fn is_transparent(flat: &[VoxelType], x: i32, y: i32, z: i32) -> bool {
    // If neighbor coordinate is out of bounds, treat as Air so outer chunk faces render
    if x < 0 || x >= CHUNK_X as i32 || y < 0 || y >= CHUNK_Y as i32 || z < 0 || z >= CHUNK_Z as i32 {
        return true;
    }
    Chunk::get_voxel(flat, x as usize, y as usize, z as usize) == VoxelType::Air
}

fn create_chunk_mesh(chunk: &Chunk) -> Mesh{
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

fn spawn_chunk(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chunk = Chunk::generate();
    let chunk_mesh = create_chunk_mesh(&chunk);

    commands.spawn((
        Mesh3d(meshes.add(chunk_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial{
            base_color: Color::WHITE,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}