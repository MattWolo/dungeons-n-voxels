use bevy::prelude::*;

fn hash2(x: i32, z: i32, seed: u32) -> (f32, f32) {
    let mut h = (x as u32).wrapping_mul(0x27d4eb2f)
        ^ (z as u32).wrapping_mul(0x165667b1)
        ^ seed.wrapping_mul(0x85ebca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a2d39);
    h ^= h >> 15;

    let fx = (h & 0xFFFF) as f32 / 65536.0;
    let fz = ((h >> 16) & 0xFFFF) as f32 / 65536.0;
    (fx, fz)
}

pub fn worley_f1(world_x: f32, world_z: f32, cell_size: f32, seed: u32) -> (f32, IVec2) {
    let cell_x = (world_x / cell_size).floor() as i32;
    let cell_z = (world_z / cell_size).floor() as i32;

    let mut best_dist_sq = f32::MAX;
    let mut best_cell = IVec2::ZERO;

    for dz in -1..=1 {
        for dx in -1..=1 {
            let cx = cell_x + dx;
            let cz = cell_z + dz;
            let (jx, jz) = hash2(cx, cz, seed);

            let point_x = (cx as f32 + jx) * cell_size;
            let point_z = (cz as f32 + jz) * cell_size;

            let dx_ = world_x - point_x;
            let dz_ = world_z - point_z;
            let dist_sq = dx_ * dx_ + dz_ * dz_;

            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_cell = IVec2::new(cx, cz);
            }
        }
    }

    (best_dist_sq.sqrt(), best_cell)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Biome {
    Plains,
    Snowy
    // more variants go here later
}

pub fn biome_for_cell(cell: IVec2, cell_size: f32, seed: u32) -> Biome {
    //let center_x = cell.x as f32 * cell_size;
    //let center_z = cell.y as f32 * cell_size;

    // low frequency relative to height noise this is what makes
    // neighboring cells agree and form large regions instead of noise
    let biome_value = fbm(cell.x as f32 * 0.05, cell.y as f32 * 0.05, 2, 0.5, 2.0, seed.wrapping_add(999));

    if biome_value > 0.05 { Biome::Snowy } else { Biome::Plains }
}

pub fn height_for_biome(biome: Biome, world_x: f32, world_z: f32, seed: u32) -> usize {
    match biome {
        Biome::Plains => {
            let n = fbm(world_x * 0.02, world_z * 0.02, 3, 0.5, 2.0, seed);
            (64.0 + n * 8.0) as usize // gentle
        }
        Biome::Snowy => {
            let n = fbm(world_x * 0.02, world_z * 0.02, 3, 0.5, 2.0, seed);
            (64.0 + n * 12.0) as usize
        }
    }
}

fn gradient_dot(ix: i32, iz: i32, x: f32, z: f32, seed: u32) -> f32 {
    let (rand_a, _) = hash2(ix, iz, seed);
    let angle = rand_a * std::f32::consts::TAU;
    let (gx, gz) = (angle.cos(), angle.sin());

    let dx = x - ix as f32;
    let dz = z - iz as f32;
    gx * dx + gz * dz
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

pub fn gradient_noise(x: f32, z: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let z0 = z.floor() as i32;
    let tx = x - x0 as f32;
    let tz = z - z0 as f32;

    let n00 = gradient_dot(x0, z0, x, z, seed);
    let n10 = gradient_dot(x0 + 1, z0, x, z, seed);
    let n01 = gradient_dot(x0, z0 + 1, x, z, seed);
    let n11 = gradient_dot(x0 + 1, z0 + 1, x, z, seed);

    let u = fade(tx);
    let v = fade(tz);
    let nx0 = n00 + u * (n10 - n00);
    let nx1 = n01 + u * (n11 - n01);
    nx0 + v * (nx1 - nx0)
}

pub fn fbm(x: f32, z: f32, octaves: u32, persistence: f32, lacunarity: f32, seed: u32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_amplitude = 0.0;

    for i in 0..octaves {
        total += gradient_noise(x * frequency, z * frequency, seed.wrapping_add(i)) * amplitude;
        max_amplitude += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    total / max_amplitude
}