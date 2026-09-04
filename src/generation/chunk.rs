use rand::{random, RngExt};
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
    pub runs: Vec<Run>
}

fn compress_to_runs(flat: &[VoxelType]) -> Vec<Run>{
    let mut runs = Vec::new();
    for chunk in flat.chunk_by(|a, b| a == b){
        let val = chunk[0];
        let mut remaining = chunk.len();

        while remaining > 0 {
            let len = remaining.min(u16::MAX as usize);
            runs.push(Run {
                value: val,
                length: len as u16,
            });
            remaining -= len;
        }
    }
    runs
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
        Self { runs: compress_to_runs(&flat) }
    }

    pub fn decompress(&self) -> Vec<VoxelType> {
        let mut flat = Vec::with_capacity(CHUNK_VOLUME);
        for run in &self.runs {
            for _ in 0..run.length {
                flat.push(run.value);
            }
        }
        flat
    }
    pub fn decompress_into(&self, output: &mut Vec<VoxelType>){
        output.clear();
        output.reserve(CHUNK_VOLUME);
        for run in &self.runs{
            output.extend(std::iter::repeat(run.value).take(run.length as usize));
        }
    }
    pub fn build_padded_chunk(
        padded: &mut [VoxelType],
        own: &[VoxelType],
        neighbor_pos_x: Option<&[VoxelType]>,
        neighbor_neg_x: Option<&[VoxelType]>,
        neighbor_pos_z: Option<&[VoxelType]>,
        neighbor_neg_z: Option<&[VoxelType]>,
    ) {
        let _span = info_span!("build_padded_chunk").entered();
        padded.fill(VoxelType::Air);

        for x in 0..CHUNK_X {
            for z in 0..CHUNK_Z {
                for y in 0..CHUNK_Y {
                    let src = x * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y;
                    padded[padded_index(x + 1, y + 1, z + 1)] = own[src];
                }
            }
        }

        if let Some(neighbor) = neighbor_pos_x {
            for z in 0..CHUNK_Z {
                for y in 0..CHUNK_Y {
                    let src = z * CHUNK_Y + y;
                    padded[padded_index(PADDED_X - 1, y + 1, z + 1)] = neighbor[src];
                }
            }
        }

        if let Some(neighbor) = neighbor_neg_x {
            for z in 0..CHUNK_Z {
                for y in 0..CHUNK_Y {
                    let src = (CHUNK_X - 1) * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y;
                    padded[padded_index(0, y + 1, z + 1)] = neighbor[src];
                }
            }
        }

        if let Some(neighbor) = neighbor_pos_z {
            for x in 0..CHUNK_X {
                for y in 0..CHUNK_Y {
                    let src = x * (CHUNK_Z * CHUNK_Y) + y;
                    padded[padded_index(x + 1, y + 1, PADDED_Z - 1)] = neighbor[src];
                }
            }
        }

        if let Some(neighbor) = neighbor_neg_z {
            for x in 0..CHUNK_X {
                for y in 0..CHUNK_Y {
                    let src = x * (CHUNK_Z * CHUNK_Y) + (CHUNK_Z - 1) * CHUNK_Y + y;
                    padded[padded_index(x + 1, y + 1, 0)] = neighbor[src];
                }
            }
        }
    }
    pub fn get_voxel(flat: &[VoxelType], x: usize, y: usize, z: usize) -> VoxelType {
        flat[x * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y]
    }
}
pub fn padded_index(px: usize, py: usize, pz: usize) -> usize {
    px * (PADDED_Z * PADDED_Y) + pz * PADDED_Y + py
}