use bevy::{
    core_pipeline::prepass::DepthPrepass,
    gltf::GltfAssetLabel,
    prelude::*,
    render::occlusion_culling::OcclusionCulling,
    world_serialization::WorldInstanceReady,
    animation::{
        AnimatedBy, AnimationTargetId,
    }
};
use bevy_sky_gradient::plugin::SkyboxMagnetTag;
use std::time::Duration;
use bevy::camera::visibility::DynamicSkinnedMeshBounds;
use bevy::ecs::system::entity_command::observe;
use bevy::mesh::skinning::SkinnedMesh;
use third_person_camera as tp_cam;
use third_person_camera::TargetOffset;
use crate::controls::MainCamera;
use crate::generation::genomes::humanoid_genome::HumanoidGenome;
use crate::generation::templates::humanoid::{
    build_humanoid_template,
    build_template_debug_mesh,
    HumanoidTemplate,
    HumanoidTemplateAsset,
    GeneratedHumanoidPreview,
    build_generated_skinned_mesh,
    GeneratedHumanoidJoint,
    HumanoidBone,
    GeneratedHumanoid,
    generate_humanoid
};

const HUMANOID_PATH: &str = "models/Humanoid_male_template.gltf";

#[derive(Component)]
pub struct Player;

#[derive(Component)]
struct PlayerVisual;

#[derive(Resource)]
struct HumanoidAnimationSet {
    graph: Handle<AnimationGraph>,
    idle: AnimationNodeIndex,
    walk: AnimationNodeIndex,
    idle_clip: Handle<AnimationClip>,
    walk_clip: Handle<AnimationClip>,
}

#[derive(Component)]
struct HumanoidAnimator {
    idle: AnimationNodeIndex,
    walk: AnimationNodeIndex,
}
#[derive(Component)]
struct HumanoidTemplatePreview;

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App){
        app.add_plugins(tp_cam::ThirdPersonCameraPlugin::default())
            .add_systems(Startup, spawn_player)
            .add_systems(Update, (
                test_player_animations,
                build_humanoid_template,
                spawn_skinned_humanoid_preview,
                //test_generated_skeleton,
            ));
    }
}

fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let idle_clip = asset_server.load(GltfAssetLabel::Animation(0).from_asset(HUMANOID_PATH));
    let walk_clip = asset_server.load(GltfAssetLabel::Animation(1).from_asset(HUMANOID_PATH));

    let (graph, animation_nodes) = AnimationGraph::from_clips([idle_clip.clone(), walk_clip.clone()]);
    let graph_handle = animation_graphs.add(graph);

    let animations = HumanoidAnimationSet {
        graph: graph_handle,
        idle: animation_nodes[0],
        walk: animation_nodes[1],
        idle_clip,
        walk_clip,
    };
    commands.insert_resource(animations);

    let player =
        commands.spawn((
            Name::new("Player"),
            Player,
            Transform::from_xyz(0.0, 65.0, 0.0),
            Visibility::default(),
    )).id();

    let humanoid_gltf: Handle<Gltf> = asset_server.load(HUMANOID_PATH);
    commands.insert_resource(HumanoidTemplateAsset {
        gltf: humanoid_gltf,
    });

    let visual = commands.spawn((
        Name::new("PlayerVisual"),
        PlayerVisual,
        Transform::IDENTITY, //if blockbench do not match scale, change this
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(HUMANOID_PATH))),
        )).observe(setup_player_animation).id();

    commands.entity(player).add_children(&[visual]);

    let camera =commands.spawn((
        Camera3d::default(),
        DepthPrepass,
        OcclusionCulling,
        Camera {
            order: 0,
            ..default()
        },
        MainCamera,
        SkyboxMagnetTag,
        Transform::default(),
        TargetOffset(Vec3::new(0.0, 3.0, 0.0)),
        tp_cam::ThirdPersonCamera::aimed_at(player),
        tp_cam::DampingFactor(5.0),
    )).id();
    commands.trigger(tp_cam::SetLocalCamera(camera));
}

fn setup_player_animation(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations: Res<HumanoidAnimationSet>,
    mut animation_players: Query<&mut AnimationPlayer>,
) {

    for descendant in children.iter_descendants(ready.entity) {
        let Ok(mut player) = animation_players.get_mut(descendant) else {
            continue;
        };

        let mut transitions = AnimationTransitions::new();

        transitions.play(
            &mut player,
            animations.idle,
            Duration::ZERO,
        ).repeat();

        commands.entity(descendant).insert((
            AnimationGraphHandle(animations.graph.clone()),
            transitions,
            HumanoidAnimator {
                idle: animations.idle,
                walk: animations.walk,
            },
        ));
    }
}

fn test_player_animations(
    keys: Res<ButtonInput<KeyCode>>,
    mut animators: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &HumanoidAnimator,
    )>,
) {
    let requested = if keys.just_pressed(KeyCode::Digit1) {
        Some(false)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(true)
    } else {
        None
    };

    let Some(walking) = requested else {
        return;
    };

    for (mut player, mut transitions, animations) in &mut animators {
        let animation = if walking {
            animations.walk
        } else {
            animations.idle
        };

        transitions.play(
            &mut player,
            animation,
            Duration::from_millis(150),
        ).repeat();
    }
}

fn test_generated_skeleton(
    keys: Res<ButtonInput<KeyCode>>,
    mut joints: Query<(&GeneratedHumanoidJoint, &mut Transform)>
) {
    if keys.just_pressed(KeyCode::Digit3) {
        for (joint, mut transform) in &mut joints {
            *transform = joint.bind_transform.clone();
        }
    }
    if keys.just_pressed(KeyCode::Digit4) {
        for (joint, mut transform) in &mut joints {
            if joint.bone == HumanoidBone::Chest {
                transform.rotation = joint.bind_transform.rotation * Quat::from_rotation_y(0.45);
            }
        }
    }
    if keys.just_pressed(KeyCode::Digit5) {
        for (joint, mut transform) in &mut joints {
            if joint.bone == HumanoidBone::Head {
                transform.rotation = joint.bind_transform.rotation * Quat::from_rotation_y(0.6);
            }
        }
    }
}

fn spawn_humanoid_template_preview(
    mut commands: Commands,
    template: Option<Res<HumanoidTemplate>>,
    player_query: Query<Entity, With<Player>>,
    existing_preview: Query<(), With<HumanoidTemplatePreview>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(template) = template else {
        return;
    };
    if !existing_preview.is_empty() {
        return;
    }
    let Ok(player) = player_query.single() else {
        return;
    };
    let mesh = build_template_debug_mesh(&template);
    let mesh_handle = meshes.add(mesh);
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        ..default()
    });
    let mut transform = template.source_transform.clone();
    transform.translation.x += 4.0;
    let preview = commands.spawn((
        Name::new("HumanoidPreview"),
        HumanoidTemplatePreview,
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        transform,
    )).id();

    commands.entity(player).add_child(preview);
}

fn spawn_skinned_humanoid_preview(
    mut commands: Commands,
    template: Option<Res<HumanoidTemplate>>,
    player_query: Query<Entity, With<Player>>,
    existing: Query<(), With<GeneratedHumanoidPreview>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    animations: Res<HumanoidAnimationSet>,
    clips: Res<Assets<AnimationClip>>
) {
    let Some(template) = template else { return; };
    if !existing.is_empty() { return; };
    let Ok(player) = player_query.single() else { return; };
    let genome = HumanoidGenome::from_seed(12345235);

    let generated = generate_humanoid(&template,&genome);
    let mesh_handle = meshes.add(build_generated_skinned_mesh(&template, &generated));
    let material_handle = materials.add(
        StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            ..default()
        }
    );
    let mut mesh_transform = template.source_transform.clone();
    mesh_transform.translation.x += 4.0;
    let mesh_entity = commands.spawn((
        Name::new("GeneratedSkinnedHumanoid"),
        GeneratedHumanoidPreview,
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        mesh_transform,
        )).id();

    let mut animation_player = AnimationPlayer::default();
    let mut transitions = AnimationTransitions::new();
    transitions.play(
        &mut animation_player,
        animations.idle,
        Duration::ZERO,
    ).repeat();

    let idle_clip = clips.get(&animations.idle_clip);
    let walk_clip = clips.get(&animations.walk_clip);

    let mut joint_entities = Vec::with_capacity(template.joints.len());
    for (joint_index, joint) in template.joints.iter().enumerate() {
        let animation_target = animation_target_for_joint(&template, joint_index);

        let idle_has_curve = idle_clip.is_some_and(|clip| {
            clip.curves_for_target(animation_target).is_some()
        });
        let walke_has_curve = walk_clip.is_some_and(|clip| {
            clip.curves_for_target(animation_target).is_some()
        });
        info!("joint '{}' -> idle={} walk={}", joint.name, idle_has_curve, walke_has_curve);

        let entity = commands.spawn((
            Name::new(joint.name.clone()),
            joint.local_transform.clone(),
            GeneratedHumanoidJoint {
                bone: joint.bone,
                bind_transform: joint.local_transform.clone(),
            },
            animation_target,
            AnimatedBy(mesh_entity),
            )).id();
        joint_entities.push(entity);
    }
    for(index, joint) in template.joints.iter().enumerate() {
        let entity = joint_entities[index];
        match joint.parent {
            Some(parent_index) => {
                let parent = joint_entities[parent_index];
                commands.entity(parent).add_child(entity);
            }
            None => {
                commands.entity(mesh_entity).add_child(entity);
            }
        }
    }
    commands.entity(mesh_entity).insert((
        animation_player,
        AnimationGraphHandle(animations.graph.clone()),
        transitions,
        HumanoidAnimator {
            idle: animations.idle,
            walk: animations.walk,
        },
        SkinnedMesh {
            inverse_bindposes: template.inverse_bindposes.clone(),
            joints: joint_entities,
        },
        DynamicSkinnedMeshBounds,
        ));
    commands.entity(player).add_child(mesh_entity);
}

fn animation_target_for_joint(
    template: &HumanoidTemplate,
    joint_index: usize,
) -> AnimationTargetId {
    let joint = &template.joints[joint_index];

    let names: Vec<Name> = joint.animation_path.iter().map(|name| {
        Name::new(name.clone())
    }).collect();
    AnimationTargetId::from_names(names.iter())
}