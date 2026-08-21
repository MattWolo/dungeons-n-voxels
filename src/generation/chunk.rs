use super::types::*;

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

    pub fn get_voxel(flat: &[VoxelType], x: usize, y: usize, z: usize) -> VoxelType {
        flat[x * (CHUNK_Z * CHUNK_Y) + z * CHUNK_Y + y]
    }
}