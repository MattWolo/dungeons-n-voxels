pub mod humanoid_genome;

pub fn hash_property(
    seed: u64,
    property: u64,
) -> u64 {
    let mut x = seed ^ property.wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x^=x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

pub fn property_f32(
    seed: u64,
    property: u64,
) -> f32 {
    let value = hash_property(seed, property);
    let normalized = value as f64 / u64::MAX as f64;
    normalized as f32
}

pub fn range(
    value: f32,
    min: f32,
    max: f32,
) -> f32 {
    min + value * (max - min)
}