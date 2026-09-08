use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_sky_gradient::plugin::SkyboxMagnetTag;
use third_person_camera as tp_cam;
use third_person_camera::TargetOffset;
use crate::controls::MainCamera;

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App){
        app.add_plugins(tp_cam::ThirdPersonCameraPlugin::default())
            .add_systems(Startup, spawn_player);
    }
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let player =
        commands.spawn((
            Mesh3d(meshes.add(Mesh::from(Cuboid::from_length(2.0)))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(0.0, 45.0, 0.0),
            Player,
    )).id();

    let camera =commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            ..default()
        },
        SkyboxMagnetTag,
        Transform::default(),
        TargetOffset(Vec3::new(0.0, 3.0, 0.0)),
        tp_cam::ThirdPersonCamera::aimed_at(player),
        tp_cam::DampingFactor(5.0),
        MainCamera,
    )).id();
    commands.trigger(tp_cam::SetLocalCamera(camera));
}