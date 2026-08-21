pub const CHUNK_X: usize = 32;
pub const CHUNK_Y: usize = 256;
pub const CHUNK_Z: usize = 32;
pub const CHUNK_VOLUME: usize = CHUNK_X * CHUNK_Y * CHUNK_Z;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VoxelType {
Air,
Grass,
}

impl VoxelType {
    pub fn face_color(&self) -> [f32; 4] {
        match self {
            VoxelType::Grass => [0.2, 1.0, 0.2, 1.0],
                VoxelType::Air => [0.0, 0.0, 0.0, 0.0],
        }
    }
}