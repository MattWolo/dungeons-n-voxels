use rand::{random, RngExt};
use crate::generation::worldgen::{biome_for_cell, height_for_biome, worley_f1, Biome};
use super::types::*;

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
    flat.chunk_by(|a, b| a == b)
        .map(|chunk| Run{
            value: chunk[0],
            length: chunk.len() as u16,
        })
        .collect()
}
impl Chunk {
    pub fn generate(chunk_x: i32, chunk_z: i32, world_seed: u32) -> Self {
        let mut flat = Vec::with_capacity(CHUNK_VOLUME);
        for x in 0..CHUNK_X {
            for z in 0..CHUNK_Z {
                let world_x = (chunk_x * CHUNK_X as i32 + x as i32) as f32;
                let world_z = (chunk_z * CHUNK_Z as i32 + z as i32) as f32;
                let (_dist, cell) = worley_f1(world_x, world_z, 256.0, world_seed);
                let biome = biome_for_cell(cell, 256.0, world_seed);
                //println!("{:?}", biome);
                let height = height_for_biome(biome, world_x, world_z, world_seed);
                let surface_material = match biome {
                    Biome::Plains => VoxelType::Grass,
                    Biome::Snowy => VoxelType::Snow,
                };
                for y in 0..CHUNK_Y {
                    let voxel = if y < height {
                        surface_material
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
        //let mut padded = vec![VoxelType::Air; PADDED_X * PADDED_Y * PADDED_Z];
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