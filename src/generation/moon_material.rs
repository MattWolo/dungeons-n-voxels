use std::f32::consts::TAU;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    render::render_resource::ShaderType
};
use bevy::shader::ShaderRef;
use bevy_sky_gradient::sun::SunDriverTag;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MoonMaterial {
    #[uniform(0)]
    pub uniforms: MoonMaterialUniforms,
}

#[derive(Resource)]
pub struct MoonDirection {
    pub dir: Vec3,
    pub night_progress: f32,
}

#[derive(Resource)]
pub struct LunarClock {
    pub day_index: u32,
    pub cycle_days: u32,
    pub was_night: bool,
}

impl Default for MoonDirection {
    fn default() -> Self {
        Self {
            dir: Vec3::ZERO,
            night_progress: 0.0,
        }
    }
}
impl Default for LunarClock {
    fn default() -> Self {
        Self {
            day_index: 0,
            cycle_days: 8,
            was_night: false,
        }
    }
}

impl LunarClock {
    pub fn phase(&self) -> f32 {
        (self.day_index % self.cycle_days) as f32 / self.cycle_days as f32
    }

    pub fn illumination(&self) -> f32 {
        0.5 - 0.5 * (self.phase() * TAU).cos()
    }
}

pub fn advance_lunar_clock(
    sun_query: Query<&Transform, With<SunDriverTag>>,
    mut lunar: ResMut<LunarClock>,
) {
    let Ok(sun_transform) = sun_query.single() else { return; };
    let is_night = sun_transform.forward().y >= 0.0;

    if is_night && !lunar.was_night {
        lunar.day_index += 1;
    }
    lunar.was_night = is_night;
}

#[derive(ShaderType, Debug, Clone)]
pub struct MoonMaterialUniforms {
    pub color: LinearRgba,
    pub moon_dir: Vec3,
    pub angular_radius: f32,
    pub softness: f32,
}
impl Material for MoonMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/moon.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

#[derive(ShaderType, Debug, Clone, Copy, Reflect)]
pub struct MoonSettings {
    pub moon_color: LinearRgba,
    pub moon_dir: Vec3,
    pub angular_radius: f32,
    pub softness: f32,
}

impl Default for MoonSettings {
    fn default() -> Self {
        Self {
            moon_color: LinearRgba::new(2.5, 2.5, 2.7, 1.0),
            moon_dir: Vec3::new(0.0, 0.707, -0.707),
            angular_radius: 0.025,
            softness: 0.003,
        }
    }
}