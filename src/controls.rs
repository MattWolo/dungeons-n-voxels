use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::prelude::*;
use third_person_camera as tp_cam;
use crate::player::Player;

#[derive(Component)]
pub struct ControlsPlugin;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
struct Disabled;

const SPEED: f32 = 100.0;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(FreeCameraPlugin)
            .add_systems(Update, close_on_escape)
            .add_systems(Update, (toggle_camera, player_movement));
    }
}

fn close_on_escape(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }
        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}

fn toggle_camera(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    camera_query: Query<(Entity, Has<FreeCamera>), With<MainCamera>>,
    player_query: Query<Entity, With<Player>>,
) {
    if !input.just_pressed(KeyCode::KeyT) {
        return;
    }
    if let (Some((camera_entity, is_free_cam)), Some(player_entity)) = (
        camera_query.iter().next(),
        player_query.iter().next(),
    ) {
        if is_free_cam {
            commands.entity(player_entity).remove::<Disabled>();
            commands
                .entity(camera_entity)
                .remove::<FreeCamera>()
                .insert((
                    tp_cam::ThirdPersonCamera::aimed_at(player_entity),
                    tp_cam::DampingFactor(5.0),
                ));
        } else {
            commands.entity(player_entity).insert(Disabled);
            commands
                .entity(camera_entity)
                .remove::<tp_cam::ThirdPersonCamera>()
                .remove::<tp_cam::DampingFactor>()
                .insert(FreeCamera{
                    sensitivity: 0.2,
                    friction: 25.0,
                    walk_speed: 3.0,
                    run_speed: 9.0,
                    ..default()
                });
        }
    }
}

fn player_movement(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<Disabled>)>,
    camera_query: Query<&Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(mut player_transform) = player_query.single_mut() else {
        return;
    };
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let mut input_dir = Vec2::ZERO;

    if input.pressed(KeyCode::KeyW) {
        input_dir.y += 1.0;
    }
    if input_dir == Vec2::ZERO && input.pressed(KeyCode::KeyS) {
        input_dir.y -= 1.0;
    } else if input.pressed(KeyCode::KeyS) {
        input_dir.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyA) {
        input_dir.x -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) {
        input_dir.x += 1.0;
    }

    if input_dir == Vec2::ZERO {
        return;
    }

    let input_3d = Vec3::new(input_dir.x, 0.0, -input_dir.y);
    let world_dir = camera_transform.rotation * input_3d;
    let flat_dir = Vec3::new(world_dir.x, 0.0, world_dir.z).normalize_or_zero();

    player_transform.translation += flat_dir * SPEED * time.delta_secs();

    if flat_dir != Vec3::ZERO {
        player_transform.look_to(flat_dir, Vec3::Y);
    }
}