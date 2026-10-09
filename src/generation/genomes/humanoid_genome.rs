use bevy::prelude::*;
use crate::generation::genomes::{property_f32, range};
use crate::generation::templates::humanoid::{joint_index_for_bone, HumanoidBone, HumanoidTemplate, TemplateVertex, joint_bind_matrix};

#[derive(Clone, Debug)]
pub struct HumanoidGenome {
    pub seed: u64,
    pub chest_width: f32,
    pub chest_height: f32,
    pub chest_depth: f32,

    pub head_scale: f32,
    pub hand_scale: f32,
    pub foot_scale: f32,
}

impl HumanoidGenome {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            seed,
            chest_width: range(
                property_f32(seed, 1),
                0.85,
                2.20,
            ),
            chest_height: range(
                property_f32(seed, 2),
                0.95,
                2.08,
            ),
            chest_depth: range(
                property_f32(seed, 3),
                0.90,
                1.15,
            ),
            head_scale: range(
                property_f32(seed, 4),
                0.70,
                2.10,
            ),
            hand_scale: range(
                property_f32(seed, 5),
                0.90,
                1.12,
            ),
            foot_scale: range(
                property_f32(seed, 6),
                0.92,
                1.12,
            )
        }
    }
}

//move it to genome.rs later
pub fn scale_vertices_for_bone(
    template: &HumanoidTemplate,
    vertices: &mut [TemplateVertex],
    bone: HumanoidBone,
    scale: Vec3,
) {
    let joint_index = joint_index_for_bone(template,bone);
    let bind = joint_bind_matrix(template, joint_index);
    let inverse_bind = bind.inverse();
    
    for vertex in vertices {
        if vertex.bone != bone {
            continue;
        }
        let local_position = inverse_bind.transform_point3(vertex.position);
        let scaled = local_position * scale;
        vertex.position = bind.transform_point3(scaled);
    }
}