use crate::generation::chunk::padded_index;

pub const CHUNK_X: usize = 32;
pub const CHUNK_Y: usize = 512;
pub const CHUNK_Z: usize = 32;
pub const CHUNK_VOLUME: usize = CHUNK_X * CHUNK_Y * CHUNK_Z;
pub const WORLD_MIN_Y: i32 = -128;
pub const WORLD_MAX_Y_EXCLUSIVE: i32 = WORLD_MIN_Y + CHUNK_Y as i32;
pub const MESH_SECTION_Y: usize = 64;
pub const MESH_SECTION_COUNT: usize = CHUNK_Y / MESH_SECTION_Y;

#[inline]
pub const fn local_y_to_world_y(local_y: usize) -> i32 {
    WORLD_MIN_Y + local_y as i32
}

#[inline]
pub fn world_y_to_local_y(world_y: i32) -> Option<usize> {
    let local_y = world_y - WORLD_MIN_Y;

    if local_y < 0 || local_y >= CHUNK_Y as i32{
        None
    } else {
        Some(local_y as usize)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum VoxelType {
    Air = 0,
    Grass = 1,
    Snow = 2,
    Sand = 3,
}

impl VoxelType {
    #[inline]
    pub fn is_solid(padded: &[VoxelType], x: usize, y: usize, z: usize) -> bool {
        if y == 0 {
            return true;
        }
        padded[padded_index(x, y, z)] != VoxelType::Air
    }
    pub fn face_color(self) -> [f32; 4] {
        match self {
            VoxelType::Air => [0.0, 0.0, 0.0, 0.0],
            VoxelType::Grass => [0.2, 1.0, 0.2, 1.0],
            VoxelType::Snow => [0.8, 0.8, 0.8, 1.0],
            VoxelType::Sand => [1.0, 1.0, 0.5, 1.0],
        }
    }
}