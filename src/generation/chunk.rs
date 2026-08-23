use super::types::*;

const PADDED_X: usize = CHUNK_X + 2;
const PADDED_Z: usize = CHUNK_Z + 2;

#[derive(Debug)]
pub struct Run {
    pub value: VoxelType,
    pub length: u16
}

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
    pub fn generate() -> Self {
        let mut flat = Vec::with_capacity(CHUNK_VOLUME);
        for _x in 0..CHUNK_X {
            for _z in 0..CHUNK_Z {
                for y in 0..CHUNK_Y {
                    let voxel = if y < 64 { VoxelType::Grass } else { VoxelType::Air };
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

    pub fn build_padded_chunk(
        own: &[VoxelType],
        neighbor_pos_x: Option<&[VoxelType]>,
        neighbor_neg_x: Option<&[VoxelType]>,
        neighbor_pos_z: Option<&[VoxelType]>,
        neighbor_neg_z: Option<&[VoxelType]>,
    ) -> Vec<VoxelType> {
        let mut padded = vec![VoxelType::Air; PADDED_X * CHUNK_Y * PADDED_Z];

        for x in 0..CHUNK_X {
            for z in 0..CHUNK_Z {
                for y in 0..CHUNK_Y {
                    let src = x * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y;
                    padded[padded_index(x + 1, y, z + 1)] = own[src];
                }
            }
        }

        if let Some(neighbor) = neighbor_pos_x {
            for z in 0..CHUNK_Z {
                for y in 0..CHUNK_Y {
                    let src = 0 * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y;
                    padded[padded_index(PADDED_X - 1, y, z + 1)] = neighbor[src];
                }
            }
        }

        if let Some(neighbor) = neighbor_neg_x {
            for z in 0..CHUNK_Z {
                for y in 0..CHUNK_Y {
                    let src = (CHUNK_X - 1) * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y;
                    padded[padded_index(0, y, z + 1)] = neighbor[src];
                }
            }
        }

        if let Some(neighbor) = neighbor_pos_z {
            for x in 0..CHUNK_X {
                for y in 0..CHUNK_Y {
                    let src = x * (CHUNK_Z * CHUNK_Y) + 0 * CHUNK_Y + y;
                    padded[padded_index(x + 1, y, PADDED_Z - 1)] = neighbor[src];
                }
            }
        }

        if let Some(neighbor) = neighbor_neg_z {
            for x in 0..CHUNK_X {
                for y in 0..CHUNK_Y {
                    let src = x * (CHUNK_Z * CHUNK_Y) + (CHUNK_Z - 1) * CHUNK_Y + y;
                    padded[padded_index(x + 1, y, 0)] = neighbor[src];
                }
            }
        }
        padded
    }

    pub fn get_voxel(flat: &[VoxelType], x: usize, y: usize, z: usize) -> VoxelType {
        flat[x * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y]
    }
}
pub fn padded_index(px: usize, py: usize, pz: usize) -> usize {
    px * (PADDED_Z * CHUNK_Y) + pz * CHUNK_Y + py
}