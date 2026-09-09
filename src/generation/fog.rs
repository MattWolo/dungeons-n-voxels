use std::ops::DerefMut;
use bevy::asset::{Asset, Assets, Handle};
use bevy::camera::{Camera, Camera3d};
use bevy::camera::visibility::RenderLayers;
use bevy::image::Image;
use bevy::math::{vec3, Vec3};
use bevy::pbr::{Material, MeshMaterial3d};
use bevy::prelude::{default, Commands, Component, Name, Reflect, ResMut, TypePath, Query};
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy_inspector_egui::bevy_egui::{EguiGlobalSettings, PrimaryEguiContext};

#[derive(Clone, Copy, Debug, PartialEq, ShaderType, Component, Reflect)]
pub struct FogBindGroup {
    pub distance_start: f32,
    pub distance_end: f32,
}

impl Default for FogBindGroup {
    fn default() -> Self {
        Self {
            distance_start: 40.0,
            distance_end: 80.0,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct FogMaterial {
    #[uniform(0)]
    pub settings: FogBindGroup,
    #[texture(1, dimension = "2d")]
    #[sampler(2)]
    pub sky_texture: Handle<Image>,
}

impl Material for FogMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/fog.wgsl".into()
    }
}

pub fn setup_egui_render_layer(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
) {
    egui_global_settings.auto_create_primary_context = false;
    commands.spawn((
        Name::new("camera_egui_fix"),
        PrimaryEguiContext,
        Camera3d::default(),
        Camera {
            order: 3,
            ..default()
        },
        RenderLayers::none(),
    ));
}

pub fn force_material_update(
    mut materials: ResMut<Assets<FogMaterial>>,
    query: Query<&MeshMaterial3d<FogMaterial>>,
) {
    // If the sky state changed or the image was resized this frame:
    for handle in query.iter() {
        if let Some(mut _material) = materials.get_mut(handle) {
            // This operation *should* force Bevy to re-prepare the material's bind group
            // and re-evaluate its texture view dependency.
            // The actual bug is on the Camera's side, but this is the user workaround.
            let _ = _material.deref_mut();
        }
    }
}