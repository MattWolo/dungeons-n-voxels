use bevy::{
    pbr::{MaterialExtension,
        MaterialExtensionKey,
        MaterialExtensionPipeline,
        StandardMaterial, },
    render::{mesh::{
            Mesh,
            MeshVertexBufferLayoutRef,
        }, render_resource::{
            RenderPipelineDescriptor,
        },
    },
    shader::ShaderRef
};
use std::ops::DerefMut;
use bevy::asset::{Asset, Assets, Handle};
use bevy::camera::{Camera, Camera3d};
use bevy::camera::visibility::RenderLayers;
use bevy::image::Image;
use bevy::math::{vec3, Vec3};
use bevy::pbr::{Material, MeshMaterial3d, MeshPipelineKey};
use bevy::prelude::{default, Commands, Component, Name, Reflect, ResMut, TypePath, Query};
use bevy::render::render_resource::{AsBindGroup, ShaderType, SpecializedMeshPipelineError, VertexAttribute};
use bevy_inspector_egui::bevy_egui::{EguiGlobalSettings, PrimaryEguiContext};
use crate::generation::ChunkMaterialHandle;
use crate::generation::mesh::ATTRIBUTE_AO;

const SHADER_PATH: &str = "shaders/voxel_material.wgsl";
#[derive(Clone, Copy, Debug, PartialEq, ShaderType, Component, Reflect)]
pub struct VoxelMaterialSettings {
    pub distance_start: f32,
    pub distance_end: f32,
}

impl Default for VoxelMaterialSettings {
    fn default() -> Self {
        Self {
            distance_start: 40.0,
            distance_end: 80.0,
        }
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct VoxelMaterialExtension {
    #[uniform(100)]
    pub settings: VoxelMaterialSettings,
    #[texture(101)]
    #[sampler(102)]
    pub sky_texture: Handle<Image>,
}

impl MaterialExtension for VoxelMaterialExtension {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }

    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let Some(index) = layout
            .0
            .attribute_ids()
            .iter()
            .position(|id| *id == ATTRIBUTE_AO.id)
        else {
            return Ok(());
        };

        let layout_attribute = &layout.0.layout().attributes[index];

        descriptor.vertex.buffers[0].attributes.push(VertexAttribute {
            format: layout_attribute.format,
            offset: layout_attribute.offset,
            shader_location: 8,
        });

        Ok(())
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
    mut materials: ResMut<Assets<ChunkMaterialHandle>>,
    query: Query<&MeshMaterial3d<ChunkMaterialHandle>>,
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