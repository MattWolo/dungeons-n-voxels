use crate::generation::worldgen::{sample_terrain};
use super::voxel_type::*;
use tracing::info_span;

pub const PADDED_X: usize = CHUNK_X + 2;
pub const PADDED_Y: usize = CHUNK_Y + 2;
pub const PADDED_Z: usize = CHUNK_Z + 2;

#[derive(Debug, Clone)]
pub struct Run {
    pub value: VoxelType,
    pub length: u16
}
#[derive(Debug, Clone)]
pub struct Chunk{
    pub flat: Vec<VoxelType>
}
impl Chunk {
    pub fn generate(chunk_x: i32, chunk_z: i32, world_seed: u32) -> Self {
        let _span = info_span!("main_generate").entered();
        let mut flat = Vec::with_capacity(CHUNK_VOLUME);
        for x in 0..CHUNK_X {
            for z in 0..CHUNK_Z {
                let world_x = (chunk_x * CHUNK_X as i32 + x as i32) as f32;
                let world_z = (chunk_z * CHUNK_Z as i32 + z as i32) as f32;
                let sampled = sample_terrain(world_x, world_z, world_seed);
                for y in 0..CHUNK_Y {
                    let voxel = if y < sampled.height {
                        sampled.surface
                    } else {
                        VoxelType::Air
                    };
                    flat.push(voxel);
                }
            }
        }
        Self { flat }
    }

    pub fn build_padded_chunk(
        padded: &mut [VoxelType],
        own: &[VoxelType],
        neighbor_pos_x: Option<&[VoxelType]>, // neighbor_east
        neighbor_neg_x: Option<&[VoxelType]>, // neighbor_west
        neighbor_pos_z: Option<&[VoxelType]>, // neighbor_north
        neighbor_neg_z: Option<&[VoxelType]>, // neighbor_south
    ) {
        padded.fill(VoxelType::Air);

        let own_stride_zy   = CHUNK_Z * CHUNK_Y;
        let padded_stride_zy = PADDED_Z * PADDED_Y;
        let padded_stride_y  = PADDED_Y;

        //Copy chunk into the center
        for x in 0..CHUNK_X {
            let own_x   = x * own_stride_zy;
            let padded_x = (x + 1) * padded_stride_zy;
            for z in 0..CHUNK_Z {
                let own_start  = own_x + z * CHUNK_Y;
                let padded_start = padded_x + (z + 1) * padded_stride_y + 1;
                padded[padded_start..padded_start + CHUNK_Y]
                    .copy_from_slice(&own[own_start..own_start + CHUNK_Y]);
            }
        }

        //East neighbor
        if let Some(edge) = neighbor_pos_x {
            let base = (PADDED_X - 1) * padded_stride_zy;
            for z in 0..CHUNK_Z {
                let dst = base + (z + 1) * padded_stride_y + 1;
                let src = z * CHUNK_Y;
                padded[dst..dst + CHUNK_Y].copy_from_slice(&edge[src..src + CHUNK_Y]);
            }
        }

        //West neighbor
        if let Some(edge) = neighbor_neg_x {
            for z in 0..CHUNK_Z {
                let dst = (z + 1) * padded_stride_y + 1;
                let src = z * CHUNK_Y;
                padded[dst..dst + CHUNK_Y].copy_from_slice(&edge[src..src + CHUNK_Y]);
            }
        }

        //North neighbor
        if let Some(edge) = neighbor_pos_z {
            let base = (PADDED_Z - 1) * padded_stride_y;
            for x in 0..CHUNK_X {
                let dst = (x + 1) * padded_stride_zy + base + 1;
                let src = x * CHUNK_Y;
                padded[dst..dst + CHUNK_Y].copy_from_slice(&edge[src..src + CHUNK_Y]);
            }
        }

        //South neighbor
        if let Some(edge) = neighbor_neg_z {
            for x in 0..CHUNK_X {
                let dst = (x + 1) * padded_stride_zy + 1;
                let src = x * CHUNK_Y;
                padded[dst..dst + CHUNK_Y].copy_from_slice(&edge[src..src + CHUNK_Y]);
            }
        }
    }

    pub fn get_voxel(flat: &[VoxelType], x: usize, y: usize, z: usize) -> VoxelType {
        flat[x * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y]
    }

    pub fn generate_border_column(world_x: f32, world_z: f32, seed: u32) -> Vec<VoxelType> {
        let sampled = sample_terrain(world_x, world_z, seed);
        (0..CHUNK_Y)
            .map(|y| if y < sampled.height { sampled.surface } else { VoxelType::Air })
            .collect()
    }

    pub fn generate_west_edge(chunk_x: i32, chunk_z: i32, seed: u32) -> Vec<VoxelType> {
        let mut edge = Vec::with_capacity(CHUNK_Z * CHUNK_Y);
        for z in 0..CHUNK_Z {
            let world_x = (chunk_x * CHUNK_X as i32) as f32;
            let world_z = (chunk_z * CHUNK_Z as i32 + z as i32) as f32;
            edge.extend(Self::generate_border_column(world_x, world_z, seed));
        }
        edge
    }

    pub fn generate_east_edge(chunk_x: i32, chunk_z: i32, seed: u32) -> Vec<VoxelType> {
        let mut edge = Vec::with_capacity(CHUNK_Z * CHUNK_Y);
        for z in 0..CHUNK_Z {
            let world_x = (chunk_x * CHUNK_X as i32 + (CHUNK_X as i32 - 1)) as f32;
            let world_z = (chunk_z * CHUNK_Z as i32 + z as i32) as f32;
            edge.extend(Self::generate_border_column(world_x, world_z, seed));
        }
        edge
    }

    pub fn generate_south_edge(chunk_x: i32, chunk_z: i32, seed: u32) -> Vec<VoxelType> {
        let mut edge = Vec::with_capacity(CHUNK_X * CHUNK_Y);
        for x in 0..CHUNK_X {
            let world_x = (chunk_x * CHUNK_X as i32 + x as i32) as f32;
            let world_z = (chunk_z * CHUNK_Z as i32) as f32;
            edge.extend(Self::generate_border_column(world_x, world_z, seed));
        }
        edge
    }

    pub fn generate_north_edge(chunk_x: i32, chunk_z: i32, seed: u32) -> Vec<VoxelType> {
        let mut edge = Vec::with_capacity(CHUNK_X * CHUNK_Y);
        for x in 0..CHUNK_X {
            let world_x = (chunk_x * CHUNK_X as i32 + x as i32) as f32;
            let world_z = (chunk_z * CHUNK_Z as i32 + (CHUNK_Z as i32 - 1)) as f32;
            edge.extend(Self::generate_border_column(world_x, world_z, seed));
        }
        edge
    }

}
pub fn padded_index(px: usize, py: usize, pz: usize) -> usize {
    px * (PADDED_Z * PADDED_Y) + pz * PADDED_Y + py
}