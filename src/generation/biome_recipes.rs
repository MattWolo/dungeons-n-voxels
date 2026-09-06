use crate::generation::voxel_type::VoxelType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BiomeCategory {
    GreenForest,
    Snowy,
    Desert,
}

#[derive(Clone, Copy, Debug)]
pub struct BiomeRecipe {
    pub name: &'static str,
    pub temp: f32,
    pub moisture: f32,
    pub erosion: f32,
    pub base_height: f32,
    pub knoll_amplitude: f32,
    pub ridge_amplitude: f32,
    pub surface: VoxelType,
    pub category: BiomeCategory,
}

pub const BIOME_RECIPES: &[BiomeRecipe] = &[
    BiomeRecipe {
        name: "Plains",
        temp: 0.2,
        moisture: -0.1,
        erosion: 0.5,
        base_height: 5.0,
        knoll_amplitude: 3.0,
        ridge_amplitude: 2.0,
        surface: VoxelType::Grass,
        category: BiomeCategory::GreenForest,
    },
    BiomeRecipe {
        name: "Hills",
        temp: 0.2,
        moisture: -0.1,
        erosion: -0.5,
        base_height: 15.0,
        knoll_amplitude: 32.0,
        ridge_amplitude: 16.0,
        surface: VoxelType::Grass,
        category: BiomeCategory::GreenForest,
    },
    BiomeRecipe {
        name: "SnowyPlains",
        temp: -0.2,
        moisture: 0.1,
        erosion: 0.5,
        base_height: 5.0,
        knoll_amplitude: 2.0,
        ridge_amplitude: 2.0,
        surface: VoxelType::Snow,
        category: BiomeCategory::Snowy,
    },
    BiomeRecipe {
        name: "SnowyHills",
        temp: -0.2,
        moisture: 0.1,
        erosion: -0.5,
        base_height: 15.0,
        knoll_amplitude: 32.0,
        ridge_amplitude: 16.0,
        surface: VoxelType::Snow,
        category: BiomeCategory::Snowy,
    },
    BiomeRecipe {
        name: "Desert",
        temp: 0.6,
        moisture: 0.0,
        erosion: 0.5,
        base_height: 5.0,
        knoll_amplitude: 2.0,
        ridge_amplitude: 2.0,
        surface: VoxelType::Sand,
        category: BiomeCategory::Desert,
    },
];

pub struct SampledTerrain {
    pub height: usize,
    pub surface: VoxelType,
}