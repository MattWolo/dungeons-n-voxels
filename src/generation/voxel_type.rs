pub const CHUNK_X: usize = 32;
pub const CHUNK_Y: usize = 512;
pub const CHUNK_Z: usize = 32;
pub const CHUNK_VOLUME: usize = CHUNK_X * CHUNK_Y * CHUNK_Z;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VoxelType {
    Air,
    Grass,
    Snow,
    Sand,
}

impl VoxelType {
    pub fn face_color(&self) -> [f32; 4] {
        match self {
            VoxelType::Air => [0.0, 0.0, 0.0, 0.0],
            VoxelType::Grass => [0.2, 1.0, 0.2, 1.0],
            VoxelType::Snow => [0.8, 0.8, 0.8, 1.0],
            VoxelType::Sand => [1.0, 1.0, 0.8, 1.0],
        }
    }
}