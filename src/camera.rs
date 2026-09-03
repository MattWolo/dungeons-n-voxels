// use bevy::{
//     camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
//     prelude::*,
// };
//
// pub struct CameraPlugin;
//
// impl Plugin for CameraPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_plugins(FreeCameraPlugin)
//             .add_systems(Startup, spawn_camera)
//             .add_systems(Update, close_on_escape);
//     }
// }
//
// fn spawn_camera(mut commands: Commands) {
//     commands.spawn((
//         Camera3d::default(),
//         Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::X, Vec3::Y),
//         FreeCamera {
//             sensitivity: 0.2,
//             friction: 25.0,
//             walk_speed: 3.0,
//             run_speed: 9.0,
//             ..default()
//         },
//     ));
//
//     commands.spawn((
//         PointLight::default(),
//         Transform::from_xyz(1.8, 1.8, 1.8),
//     ));
//
//     commands.spawn((
//         DirectionalLight {
//             illuminance: 8000.0,
//             ..default()
//         },
//         Transform::from_xyz(0.0, 20.0, 0.0)
//             .looking_at(Vec3::ZERO, Vec3::Y),
//     ));
// }
//
// fn close_on_escape(
//     mut commands: Commands,
//     focused_windows: Query<(Entity, &Window)>,
//     input: Res<ButtonInput<KeyCode>>,
// ) {
//     for (window, focus) in focused_windows.iter() {
//         if !focus.focused {
//             continue;
//         }
//         if input.just_pressed(KeyCode::Escape) {
//             commands.entity(window).despawn();
//         }
//     }
// }