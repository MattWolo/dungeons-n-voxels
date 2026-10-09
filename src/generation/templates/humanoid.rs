use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::DynamicSkinnedMeshBounds,
    gltf::{GltfMesh, GltfNode, GltfSkin},
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues,
        skinning::{
            SkinnedMesh, SkinnedMeshInverseBindposes,
        }},
    prelude::*};
use std::collections::HashMap;
use crate::generation::genomes::humanoid_genome::{scale_vertices_for_bone, HumanoidGenome};

const CHARACTER_VOXEL_SIZE: f32 = 0.1;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HumanoidBone {
    Root,
    Torso,
    Chest,
    Head,
    HandR,
    HandL,
    FootR,
    FootL,
}
#[derive(Component)]
pub struct GeneratedHumanoidPreview;
#[derive(Component)]
pub struct GeneratedHumanoidJoint {
    pub bone: HumanoidBone,
    pub bind_transform: Transform
}
pub struct HumanoidPart {
    bone: HumanoidBone,
    size: UVec3, //dimensions in voxel units
    offset: IVec3, //position relative to bone's pivot
    region: BodyRegion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyRegion {
    Torso,
    Chest,
    Head,
    Hand,
    Foot,
}
#[derive(Resource)]
pub struct HumanoidTemplate {
    pub vertices: Vec<TemplateVertex>,
    pub indices: Vec<u32>,
    pub joints: Vec<HumanoidJointTemplate>,
    pub inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    pub source_transform: Transform,
}
#[derive(Resource)]
pub struct HumanoidTemplateAsset {
    pub gltf: Handle<Gltf>,
}
#[derive(Clone, Copy, Debug)]
pub struct TemplateVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub bone: HumanoidBone,
    pub region: BodyRegion,
}

#[derive(Default)]
struct MeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}
#[derive(Clone, Debug)]
pub struct HumanoidJointTemplate {
    pub bone: HumanoidBone,
    pub name: String,
    pub local_transform: Transform,
    pub parent: Option<usize>,
    pub animation_path: Vec<String>,
}
pub struct GeneratedHumanoid {
    pub vertices: Vec<TemplateVertex>,
    pub indices: Vec<u32>,
}

pub fn build_humanoid_template(
    mut commands: Commands,
    template_asset: Option<Res<HumanoidTemplateAsset>>,
    gltfs: Res<Assets<Gltf>>,
    nodes: Res<Assets<GltfNode>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    skins: Res<Assets<GltfSkin>>,
    meshes: Res<Assets<Mesh>>,
) {
    let Some(template) = template_asset else {
        return;
    };
    let Some(gltf) = gltfs.get(&template.gltf) else {
        return;
    };
    let Some(root_handle) = gltf.named_nodes.get("root") else {
        error!("Humanoid template has no node named 'root");
        return;
    };
    info!("--- HUMANOID TEMPLATE HIERARCHY ---");
    let complete = visit_node_debug(
        root_handle,
        0,
        HumanoidBone::Root,
        None,
        &nodes,
    );

    if !complete {
        return;
    }
    info!("--- END HUMANOID TEMPLATE HIERARCHY ---");
    info!("--- SOURCE MESH / SKIN ---");

    let complete = inspect_source_mesh(
        root_handle,
        &nodes,
        &gltf_meshes,
        &skins,
        &meshes,
    );

    if !complete {
        return;
    }
    info!("-- END SOURCE MESH / SKIN ---");

    let humanoid_template =
        match extract_humanoid_template(
            root_handle,
            &nodes,
            &gltf_meshes,
            &skins,
            &meshes,
        ) {
            Ok(template) => template,
            Err(error) => {
                error!("Failed to extract humanoid template: {}", error);
                return;
            }
        };
    info!("Extracted humanoid template : \
    {} vertices, {} indices, {} triangles",
    humanoid_template.vertices.len(),
    humanoid_template.indices.len(),
    humanoid_template.indices.len() / 3,
    );

    let torso_count = humanoid_template
        .vertices.iter()
        .filter(|v| v.region == BodyRegion::Torso).count();
    let chest_count = humanoid_template
        .vertices.iter()
        .filter(|v| v.region == BodyRegion::Chest).count();
    let head_count = humanoid_template
        .vertices.iter()
        .filter(|v| v.region == BodyRegion::Head).count();
    let hand_count = humanoid_template
        .vertices.iter()
        .filter(|v| v.region == BodyRegion::Hand).count();
    let foot_count = humanoid_template
        .vertices.iter()
        .filter(|v| v.region == BodyRegion::Foot).count();
    info!("Regions: torso={} chest={} head={} hands={} feet={}",
        torso_count, chest_count, head_count, hand_count, foot_count,
    );

    commands.insert_resource(humanoid_template);
    commands.remove_resource::<HumanoidTemplateAsset>();
}

pub fn build_template_debug_mesh(
    template: &HumanoidTemplate,
) -> Mesh {
    let positions: Vec<[f32; 3]> = template
        .vertices.iter().map(|vertex| {
        vertex.position.to_array()
    }).collect();
    let normals: Vec<[f32; 3]> = template
        .vertices.iter().map(|vertex| {
        vertex.normal.to_array()
    }).collect();
    let colors: Vec<[f32; 4]> = template
        .vertices.iter().map(|vertex| {
        region_debug_color(vertex.region)
    }).collect();

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(template.indices.clone()))
}

fn visit_node_debug(
    node_handle: &Handle<GltfNode>,
    depth: usize,
    inherited_bone: HumanoidBone,
    inherited_region: Option<BodyRegion>,
    nodes: &Assets<GltfNode>,
) -> bool {
    let Some(node) = nodes.get(node_handle) else {
        return false;
    };

    let bone = bone_from_name(&node.name).unwrap_or(inherited_bone);
    let region = region_from_name(&node.name).or(inherited_region);

    let indent = " ".repeat(depth);

    info!("{}{} | index={} bone={:?} region={:?} mesh={} animation_root={}",
        indent,
        node.name,
        node.index,
        bone,
        region,
        node.mesh.is_some(),
        node.is_animation_root,
    );

    for child in &node.children {
        let complete = visit_node_debug(
            child,
            depth + 1,
            bone,
            region,
            nodes,
        );
        if !complete {
            return false;
        }
    }
    true
}

fn inspect_source_mesh(
    node_handle: &Handle<GltfNode>,
    nodes: &Assets<GltfNode>,
    gltf_meshes: &Assets<GltfMesh>,
    skins: &Assets<GltfSkin>,
    meshes: &Assets<Mesh>,
) -> bool {
    let Some(node) = nodes.get(node_handle) else {
        return false;
    };

    info!(
        "Source node '{}' | mesh={} skin={}",
        node.name,
        node.mesh.is_some(),
        node.skin.is_some(),
    );
    if let Some(skin_handle) = &node.skin {
        let Some(skin) = skins.get(skin_handle) else {
            return false;
        };

        info!(
            "Skin '{}' contains {} joints:",
            skin.name,
            skin.joints.len(),
        );

        for (joint_index, joint_handle) in skin.joints.iter().enumerate() {
            let Some(joint_node) = nodes.get(joint_handle) else {
                return false;
            };
            info!("  joint[{joint_index}] = '{}'",
            joint_node.name
            );
        }
    } else {
        warn!("Source node '{}' has no GltfSkin",
        node.name
        );
    }
    let Some(gltf_mesh_handle) = &node.mesh else {
        warn!(
            "Source node '{}' has no GltfMesh",
            node.name
        );
        return true;
    };
    let Some(gltf_mesh) = gltf_meshes.get(gltf_mesh_handle)
    else {
        return false;
    };
    info!(
        "GltfMesh '{}' has {} primitive(s)",
        gltf_mesh.name,
        gltf_mesh.primitives.len(),
    );
    for primitive in &gltf_mesh.primitives {
        let Some(mesh) = meshes.get(&primitive.mesh)
        else {
            return false;
        };
        inspect_primitive_mesh(
            primitive.index,
            mesh,
        );
    }
    true
}

fn inspect_primitive_mesh(
    primitive_index: usize,
    mesh: &Mesh,
) {
    let vertex_count = mesh.count_vertices();
    let index_count = mesh.indices().map(|indices| indices.len()).unwrap_or(0);

    info!(
        "Primitive {primitive_index}: \"
        vertices={vertex_count}, \
        indices={index_count}"
    );

    info!(
        "Position        = {}",
        mesh.contains_attribute(Mesh::ATTRIBUTE_POSITION)
    );
    info!(
        "  NORMAL        = {}",
        mesh.contains_attribute(
            Mesh::ATTRIBUTE_NORMAL
        )
    );

    info!(
        "  UV_0          = {}",
        mesh.contains_attribute(
            Mesh::ATTRIBUTE_UV_0
        )
    );

    info!(
        "  COLOR         = {}",
        mesh.contains_attribute(
            Mesh::ATTRIBUTE_COLOR
        )
    );

    info!(
        "  JOINT_INDEX   = {}",
        mesh.contains_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX
        )
    );

    info!(
        "  JOINT_WEIGHT  = {}",
        mesh.contains_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT
        )
    );

    inspect_skinning_attributes(mesh);
}

fn inspect_skinning_attributes(mesh: &Mesh) {
    let Some(joint_attribute) =
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX) else {
        warn!("No joint index attribute");
        return;
    };
    let Some(weight_attribute) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
    else {
        warn!("No joint weight attribute");
        return;
    };
    let VertexAttributeValues::Uint16x4(joint_indices) = joint_attribute
    else {
        warn!("invalid indices format");
        return;
    };

    let VertexAttributeValues::Float32x4(joint_weights) = weight_attribute
    else {
        warn!("invalid weight format");
        return;
    };
    info!(" skinning vertices = {}", joint_indices.len());

    let mut rigid_vertices = 0usize;
    for weights in joint_weights {
        let non_zero = weights.iter().filter(|weight| **weight > 0.0001).count();
        let sum: f32 = weights.iter().sum();
        if non_zero == 1 && (sum - 1.0).abs() < 0.001 {
            rigid_vertices += 1;
        }
    }

    info!(" rigid vertices = {}/{}", rigid_vertices, joint_weights.len(),);

    for i in 0..joint_indices.len().min(12) {
        info!(" vertex[{i}] joints={:?} weights={:?}", joint_indices[i], joint_weights[i]);
    }
}

fn bone_from_name(name: &str) -> Option<HumanoidBone> {
    match name {
        "root" => Some(HumanoidBone::Root),
        "torso" => Some(HumanoidBone::Torso),
        "chest" => Some(HumanoidBone::Chest),
        "head" => Some(HumanoidBone::Head),
        "right_hand" => Some(HumanoidBone::HandR),
        "left_hand" => Some(HumanoidBone::HandL),
        "right_foot" => Some(HumanoidBone::FootR),
        "left_foot" => Some(HumanoidBone::FootL),
        _ => None,
    }
}
fn region_from_name(name: &str) -> Option<BodyRegion> {
    match name {
        "torso" => Some(BodyRegion::Torso),
        "chest" => Some(BodyRegion::Chest),
        "head" => Some(BodyRegion::Head),
        "left_hand" | "right_hand" => Some(BodyRegion::Hand),
        "left_foot" | "right_foot" => Some(BodyRegion::Foot),
        _ => None
    }
}

fn region_from_bone(bone: HumanoidBone) -> Option<BodyRegion> {
    match bone {
        HumanoidBone::Root => None,
        HumanoidBone::Torso => Some(BodyRegion::Torso),
        HumanoidBone::Chest => Some(BodyRegion::Chest),
        HumanoidBone::Head => Some(BodyRegion::Head),
        HumanoidBone::HandL | HumanoidBone::HandR => Some(BodyRegion::Hand),
        HumanoidBone::FootL | HumanoidBone::FootR => Some(BodyRegion::Foot)
    }
}

pub fn joint_index_for_bone(
    template: &HumanoidTemplate,
    bone: HumanoidBone,
) -> usize {
    template.joints.iter().position(|joint| {
        joint.bone == bone
    }).expect("Humanoid bone missing from template")
}

fn rigid_joint_index(
    joints: [u16; 4],
    weights: [f32; 4],
) -> Result<usize, String> {
    let mut active_slot = None;
    for slot in 0..4{
        if weights[slot] > 0.0001 {
            if active_slot.is_some() {
                return Err(format!("Vertex has more than one joints: \
                joints={joints:?}, weights={weights:?}"));
            }
            active_slot = Some(slot);
        }
    }
    let Some(slot) = active_slot else {
        return Err(format!("Vertex has no joints: \
        joints={joints:?}, weights={weights:?}"
        ));
    };
    if (weights[slot] - 1.0).abs() > 0.001 {
        return Err(format!("Expected rigid weight 1.0, got {}", weights[slot]));
    }
    Ok(joints[slot] as usize)
}

fn extract_humanoid_template(
    root_handle: &Handle<GltfNode>,
    nodes: &Assets<GltfNode>,
    gltf_meshes: &Assets<GltfMesh>,
    skins: &Assets<GltfSkin>,
    meshes: &Assets<Mesh>,
) -> Result<HumanoidTemplate, String> {
    let root = nodes.get(root_handle).ok_or("Root GltfNode is not loaded")?;
    let skin_handle = root.skin.as_ref().ok_or("Root node has no skins")?;
    let skin = skins.get(skin_handle).ok_or("GltfSkin is not loaded")?;
    let animation_root = find_animation_root(root_handle, nodes).ok_or("Could not find Gltf animation root")?;
    let animation_root_node = nodes.get(&animation_root).ok_or("Animation root node is not loaded")?;
    info!("Gltf animation root is '{}' index={}", animation_root_node.name, animation_root_node.index);
    let mut animation_paths = HashMap::new();
    collect_animation_paths(&animation_root, nodes, &mut Vec::new(), &mut animation_paths)?;
    let joints = extract_joint_templates(skin, nodes, &animation_paths)?;
    let gltf_mesh_handle = root.mesh.as_ref().ok_or("Root node has no mesh")?;
    let gltf_mesh = gltf_meshes.get(gltf_mesh_handle).ok_or("GltfMesh is not loaded")?;

    let mut joint_to_bone = Vec::<HumanoidBone>::with_capacity(
        skin.joints.len()
    );
    for joint_handle in &skin.joints {
        let joint_node = nodes.get(joint_handle).ok_or("Skin joint node is not loaded")?;

        let bone = bone_from_name(&joint_node.name)
            .ok_or_else(|| {
                format!("Unknown humanoid joint '{}'", joint_node.name)
            })?;
        joint_to_bone.push(bone);
    }
    info!("Resolved {} GLTF joints", joint_to_bone.len());

    let mut template = HumanoidTemplate {
        vertices: Vec::new(),
        indices: Vec::new(),
        joints,
        inverse_bindposes: skin.inverse_bind_matrices.clone(),
        source_transform: root.transform.clone(),
    };
    
    for (index, joint) in
        template.joints.iter().enumerate()
    {
        info!("template joint[{index}] {:?} '{}' parent={:?}", joint.bone, joint.name, joint.parent,);
    }

    //humanoid template has one primitive, gltfMesh can have multiple
    for primitive in &gltf_mesh.primitives {
        let source_mesh = meshes.get(&primitive.mesh).ok_or("Primitive Mesh is not loaded")?;
        append_primitive_to_template(
            source_mesh,
            &joint_to_bone,
            &mut template,
        )?;
    }
    Ok(template)
}

fn extract_joint_templates(
    skin: &GltfSkin,
    nodes: &Assets<GltfNode>,
    animation_paths: &HashMap<AssetId<GltfNode>, Vec<String>>
) -> Result<Vec<HumanoidJointTemplate>, String> {
    let joint_index_by_node: HashMap<_, usize> =
        skin.joints.iter().enumerate().map(|(index, handle)| {
            (handle.id(), index)
        }).collect();
    let mut joints = Vec::with_capacity(skin.joints.len());
    for joint_handle in &skin.joints {
        let joint_node = nodes.get(joint_handle).ok_or("Joint GlthNode is not laoded")?;
        let bone = bone_from_name(&joint_node.name).ok_or_else(|| {
            format!("Unknown humanoid joint '{}'", joint_node.name)
        })?;

        let animation_path = animation_paths.get(&joint_handle.id()).ok_or_else(|| {
            format!("Joint '{}' was not found below animation root", joint_node.name)
        })?.clone();

        info!("Joint '{}' animation path: {}", joint_node.name, animation_path.join("/"));

        joints.push(HumanoidJointTemplate {
            bone,
            name: joint_node.name.clone(),
            local_transform: joint_node.transform.clone(),
            parent: None,
            animation_path,
        });
    }
    for (parent_index, parent_handle) in skin.joints.iter().enumerate() {
        let parent_node = nodes.get(parent_handle).ok_or("Parent GlthNode is not loaded")?;

        for child_handle in &parent_node.children {
            let Some(&child_index) = joint_index_by_node.get(&child_handle.id())
            else {
                continue;
            };
            joints[child_index].parent = Some(parent_index);
        }
    }
    Ok(joints)
}

fn append_primitive_to_template(
    mesh: &Mesh,
    joint_to_bone: &[HumanoidBone],
    template: &mut HumanoidTemplate,
) -> Result<(), String> {
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(values)) => values,
        _ => {
            return Err("Mesh Position is not float 32x3".into());
        }
    };

    let normals = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(values)) => values,
        _ => {
            return Err("Mesh Normal is not float 32x3".into());
        }
    };

    let joint_indices = match mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX) {
        Some(VertexAttributeValues::Uint16x4(values)) => values,
        _ => {
            return Err("Mesh joint Index is not UInt 16x4".into());
        }
    };

    let joint_weights = match mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT) {
        Some(VertexAttributeValues::Float32x4(values)) => values,
        _ => {
            return Err("Mesh joint Weight is not float 32x4".into());
        }
    };

    let vertex_count = positions.len();
    if normals.len() != vertex_count
        || joint_indices.len() != vertex_count
        || joint_weights.len() != vertex_count {
        return Err("Humanoid mesh vertex attribute lengths differ".into());
    }

    let base_vertex = template.vertices.len() as u32;
    for i in 0..vertex_count {
        let gltf_joint_index = rigid_joint_index(joint_indices[i], joint_weights[i])?;
        let bone = *joint_to_bone.get(gltf_joint_index).ok_or_else(|| {
            format!("Joint index {} out of bonds", gltf_joint_index)
        })?;
        let region = region_from_bone(bone).ok_or_else(|| {
            format!("Visible vertex is bount to {:?}, which has no body region", bone)
        })?;

        template.vertices.push(
            TemplateVertex{
                position: Vec3::from_array(positions[i]),
                normal: Vec3::from_array(normals[i]),
                bone,
                region,
            },
        );
    }

    //copy triangle indices

    if let Some(indices) = mesh.indices() {
        template.indices.extend(indices.iter().map(|index| {
            base_vertex + index as u32
        }));
    } else {
        template.indices.extend(
            (0..vertex_count as u32).map(|index| {
                base_vertex + index
            }));
    }
    Ok(())
}

fn find_animation_root(
    node_handle: &Handle<GltfNode>,
    nodes: &Assets<GltfNode>,
) -> Option<Handle<GltfNode>> {
    let node = nodes.get(node_handle)?;
    if node.is_animation_root {
        return Some(node_handle.clone());
    }
    for child in &node.children {
        if let Some(found) = find_animation_root(child, nodes) {
            return Some(found);
        }
    }
    None
}

fn collect_animation_paths(
    node_handle: &Handle<GltfNode>,
    nodes: &Assets<GltfNode>,
    current_path: &mut Vec<String>,
    paths: &mut HashMap<AssetId<GltfNode>, Vec<String>>,
) -> Result<(), String> {
    let node = nodes.get(node_handle).ok_or("GltfNode missing while building animation paths")?;
    current_path.push(node.name.clone());
    paths.insert(node_handle.id(), current_path.clone());
    for child in &node.children {
        collect_animation_paths(
            child,
            nodes,
            current_path,
            paths,
        )?;
    }
    current_path.pop();
    Ok(())
}


fn push_face(
    buffers: &mut MeshBuffers,
    corners: [Vec3; 4],
    normal: Vec3,
    color: [f32; 4],
) {
    let base = buffers.positions.len() as u32;

    for corner in corners {
        buffers.positions.push(corner.to_array());
        buffers.normals.push(normal.to_array());
        buffers.colors.push(color);
    }
    buffers.uvs.extend_from_slice(&[
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ]);

    buffers.indices.extend_from_slice(&[
        base,
        base + 1,
        base + 2,

        base,
        base + 2,
        base + 3,
    ]);
}

//FRONT (-Z)
fn push_cuboid(
    buffers: &mut MeshBuffers,
    center: Vec3,
    size: Vec3,
    color: [f32; 4],
) {
    let half = size * 0.5;
    let min = center - half;
    let max = center + half;

    push_face(
        buffers,
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, min.y, min.z),
        ],
        Vec3::NEG_Z,
        color,
    );

    // BACK (+Z)
    push_face(
        buffers,
        [
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ],
        Vec3::Z,
        color,
    );

    // LEFT (-X)
    push_face(
        buffers,
        [
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, min.z),
        ],
        Vec3::NEG_X,
        color,
    );

    // RIGHT (+X)
    push_face(
        buffers,
        [
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, min.y, max.z),
        ],
        Vec3::X,
        color,
    );

    // TOP (+Y)
    push_face(
        buffers,
        [
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, max.y, min.z),
        ],
        Vec3::Y,
        color,
    );

    // BOTTOM (-Y)
    push_face(
        buffers,
        [
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, min.y, max.z),
        ],
        Vec3::NEG_Y,
        color,
    );
}

fn region_debug_color(region: BodyRegion) -> [f32; 4] {
    match region {
        BodyRegion::Torso => [0.65, 0.65, 0.65, 1.0],
        BodyRegion::Chest => [1.00, 0.20, 0.20, 1.0],
        BodyRegion::Head => [0.20, 0.40, 1.00, 1.0],
        BodyRegion::Hand => [0.20, 1.00, 0.30, 1.0],
        BodyRegion::Foot => [1.00, 0.85, 0.15, 1.0],
        _ => unreachable!()
    }
}

fn build_bone_to_joint_map(template: &HumanoidTemplate) -> HashMap<HumanoidBone, u16> {
    template.joints.iter().enumerate().map(|(index, joint)| {
        (joint.bone, index as u16)
    }).collect()
}

pub fn build_generated_skinned_mesh(
    template: &HumanoidTemplate,
    generated: &GeneratedHumanoid,
) -> Mesh {
    let bone_to_joint = build_bone_to_joint_map(template);
    let positions: Vec<[f32; 3]> = generated.vertices.iter().map(|vertex| {
        vertex.position.to_array()
    }).collect();
    let normals: Vec<[f32; 3]> = generated.vertices.iter().map(|vertex| {
        vertex.normal.to_array()
    }).collect();
    let colors: Vec<[f32; 4]> = generated.vertices.iter().map(|vertex| {
        region_debug_color(vertex.region)
    }).collect();
    let joint_indices: Vec<[u16; 4]> = generated.vertices.iter().map(|vertex| {
        let joint = *bone_to_joint.get(&vertex.bone).expect("Vertex bone missing from skeleton");
        [joint, 0, 0, 0]
    }).collect();
    let joint_weights: Vec<[f32; 4]> = vec![[1.0, 0.0, 0.0, 0.0]; generated.vertices.len()];

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(joint_indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights)
        .with_inserted_indices(Indices::U32(generated.indices.clone()))
        .with_generated_skinned_mesh_bounds().expect("Failed to generate skinned mesh")
}

pub fn generate_humanoid(
    template: &HumanoidTemplate,
    genome: &HumanoidGenome,
) -> GeneratedHumanoid {
    let mut vertices = template.vertices.clone();
    scale_vertices_for_bone(
        template,
        &mut vertices,
        HumanoidBone::Chest,
        Vec3::new(
          genome.chest_width,
          genome.chest_height,
          genome.chest_depth,
        ),
    );
    GeneratedHumanoid {
        vertices,
        indices: template.indices.clone(),
    }
}

// for genome
pub fn joint_bind_matrix(
    template: &HumanoidTemplate,
    joint_index: usize,
) -> Mat4 {
    let joint = &template.joints[joint_index];
    let local = joint.local_transform.to_matrix();
    match joint.parent {
        Some(parent) => {
            joint_bind_matrix(template, parent) * local
        }
        None => local,
    }
}

pub fn build_humanoid_mesh(
    parts: &[HumanoidPart],
    voxel_size: f32,
) -> Mesh {
    let mut buffers = MeshBuffers::default();

    for part in parts {
        let center = part.offset.as_vec3() * voxel_size;
        let size = part.size.as_vec3() * voxel_size;
        let color = region_debug_color(part.region);
        push_cuboid(&mut buffers, center, size, color);
    }
    info!(
    "Generated humanoid: {} vertices, {} indices, {} triangles",
    buffers.positions.len(),
    buffers.indices.len(),
    buffers.indices.len() / 3,
);
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default(),)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, buffers.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, buffers.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, buffers.colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, buffers.uvs)
        .with_inserted_indices(Indices::U32(buffers.indices))
}

pub fn basic_male_humanoid() -> Vec<HumanoidPart> {
    vec![
        HumanoidPart {
            bone: HumanoidBone::Torso,
            size: UVec3::new(8, 10, 4),
            offset: IVec3::new(0, 9, 0),
            region: BodyRegion::Torso,
        },
        HumanoidPart {
            bone: HumanoidBone::Head,
            size: UVec3::new(6, 6, 6),
            offset: IVec3::new(0, 17, 0),
            region: BodyRegion::Head,
        },
        HumanoidPart {
            bone: HumanoidBone::HandL,
            size: UVec3::new(3, 6, 3),
            offset: IVec3::new(-6, 9, 0),
            region: BodyRegion::Hand,
        },
        HumanoidPart {
            bone: HumanoidBone::HandR,
            size: UVec3::new(3, 6, 3),
            offset: IVec3::new(6, 9, 0),
            region: BodyRegion::Hand,
        },
        HumanoidPart {
            bone: HumanoidBone::FootL,
            size: UVec3::new(3, 4, 4),
            offset: IVec3::new(-2, 2, 0),
            region: BodyRegion::Foot,
        },
        HumanoidPart {
            bone: HumanoidBone::FootR,
            size: UVec3::new(3, 4, 4),
            offset: IVec3::new(2, 2, 0),
            region: BodyRegion::Foot,
        },
    ]
}
