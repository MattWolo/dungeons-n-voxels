use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
pub enum LodLevel {
    Full,
    Far,
}
pub const FULL_DETAIL_DISTANCE: i32 = 16;

pub fn lod_for_distance_sq(distance_sq: i32) -> LodLevel {
    if distance_sq <= FULL_DETAIL_DISTANCE * FULL_DETAIL_DISTANCE {
        LodLevel::Full
    } else {
        LodLevel::Far
    }
}
