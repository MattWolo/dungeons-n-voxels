use crate::generation::biome_recipes::{SampledTerrain, BIOME_RECIPES, BiomeRecipe};
use crate::generation::voxel_type::{VoxelType, CHUNK_Y};

pub fn sample_terrain(world_x: f32, world_z: f32, seed: u32) -> SampledTerrain {
    let temp = sample_temperature(world_x, world_z, seed);
    let moisture = sample_moisture(world_x, world_z, seed);
    let erosion = sample_erosion(world_x, world_z, seed);

    let mut total_weight = 0.0;
    let mut blended_base = 0.0;
    let mut blended_knoll_amp = 0.0;
    let mut blended_ridge_amp = 0.0;

    let mut dominant_weight = -1.0;
    let mut dominant_surface = VoxelType::Grass;

    for recipe in BIOME_RECIPES {
        let d_temp = temp - recipe.temp;
        let d_moist = moisture - recipe.moisture;
        let d_erosion = erosion - recipe.erosion;

        let dist_sq = d_temp * d_temp + d_moist * d_moist + d_erosion * d_erosion;

        if dist_sq > 0.8 {
            continue;
        }

        let weight = (-8.0 * dist_sq).exp() + 0.00001;

        if weight > dominant_weight {
            dominant_weight = weight;
            dominant_surface = recipe.surface;
        }

        total_weight += weight;
        blended_base += recipe.base_height * weight;
        blended_knoll_amp += recipe.knoll_amplitude * weight;
        blended_ridge_amp += recipe.ridge_amplitude * weight;
    }

    let inv_weight = 1.0 / total_weight;

    let base = blended_base * inv_weight;
    let knoll_amp = blended_knoll_amp * inv_weight;
    let ridge_amp = blended_ridge_amp * inv_weight;

    let rolling = fbm(world_x * 0.0004, world_z * 0.0004, 2, 0.5, 2.0, seed.wrapping_add(200)) * 6.0;
    let knoll = billow_knoll(world_x, world_z, seed) * knoll_amp;
    let ridge = fbm(world_x * 0.002, world_z * 0.002, 3, 0.5, 2.0, seed.wrapping_add(300)).abs() * ridge_amp;

    let final_height = (base + rolling + knoll + ridge).clamp(0.0, CHUNK_Y as f32) as usize;

    SampledTerrain {
        height: final_height,
        surface: dominant_surface,
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
// fn lerp(a: f32, b: f32, t: f32) -> f32 {
//     a + t * (b - a)
// }

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

pub fn billow_knoll(world_x: f32, world_z: f32, seed: u32) -> f32 {
    let raw = fbm(world_x * 0.008, world_z * 0.008, 3, 0.5, 2.0, seed);
    let billow = 1.0 - raw.abs();
    billow.powf(3.0)
}

// pub fn worley_f1(world_x: f32, world_z: f32, cell_size: f32, seed: u32) -> (f32, IVec2) {
//     let cell_x = (world_x / cell_size).floor() as i32;
//     let cell_z = (world_z / cell_size).floor() as i32;
// 
//     let mut best_dist_sq = f32::MAX;
//     let mut best_cell = IVec2::ZERO;
// 
//     for dz in -1..=1 {
//         for dx in -1..=1 {
//             let cx = cell_x + dx;
//             let cz = cell_z + dz;
//             let (jx, jz) = hash2(cx, cz, seed);
// 
//             let point_x = (cx as f32 + jx) * cell_size;
//             let point_z = (cz as f32 + jz) * cell_size;
// 
//             let dx_ = world_x - point_x;
//             let dz_ = world_z - point_z;
//             let dist_sq = dx_ * dx_ + dz_ * dz_;
// 
//             if dist_sq < best_dist_sq {
//                 best_dist_sq = dist_sq;
//                 best_cell = IVec2::new(cx, cz);
//             }
//         }
//     }
// 
//     (best_dist_sq.sqrt(), best_cell)
// }
// pub fn surface_material(world_x: f32, world_z: f32, seed: u32) -> VoxelType {
//     let temp = sample_temperature(world_x, world_z, seed);
//     if temp < -0.1 { VoxelType::Snow } else { VoxelType::Grass }
// }

pub fn sample_temperature(world_x: f32, world_z: f32, seed: u32) -> f32 {
    fbm(world_x * 0.00008, world_z * 0.00008, 3, 0.5, 2.0, seed.wrapping_add(1))
}
pub fn sample_moisture(world_x: f32, world_z: f32, seed: u32) -> f32 {
    fbm(world_x * 0.00008, world_z * 0.00008, 3, 0.5, 2.0, seed.wrapping_add(2))
}
pub fn sample_erosion(world_x: f32, world_z: f32, seed: u32) -> f32 {
    fbm(world_x * 0.0001, world_z * 0.0001, 3, 0.5, 2.0, seed.wrapping_add(3))
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

    ((total / max_amplitude) * 2.5).clamp(-1.0, 1.0)
}

pub fn sample_biome(world_x: f32, world_z: f32, seed: u32) -> (&'static BiomeRecipe, f32, f32) {
    let temp = sample_temperature(world_x, world_z, seed);
    let moisture = sample_moisture(world_x, world_z, seed);
    let erosion = sample_erosion(world_x, world_z, seed);

    let mut dominant_weight = -1.0;
    let mut dominant_recipe = &BIOME_RECIPES[0];

    for recipe in BIOME_RECIPES {
        let d_temp = temp - recipe.temp;
        let d_moist = moisture - recipe.moisture;
        let d_erosion = erosion - recipe.erosion;

        let dist_sq = d_temp * d_temp + d_moist * d_moist + d_erosion * d_erosion;

        let weight = (-8.0 * dist_sq).exp() + 0.00001;
        if weight > dominant_weight {
            dominant_weight = weight;
            dominant_recipe = recipe;
        }
    }
    (dominant_recipe, temp, moisture)
}