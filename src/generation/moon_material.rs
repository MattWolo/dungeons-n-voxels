use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    render::render_resource::ShaderType
};
use bevy::shader::ShaderRef;


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

impl Default for MoonDirection {
    fn default() -> Self {
        Self {
            dir: Vec3::ZERO,
            night_progress: 0.0,
        }
    }
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