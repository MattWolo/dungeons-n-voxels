use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::prelude::*;
use third_person_camera as tp_cam;
use crate::player::Player;

#[derive(Component)]
pub struct ControlsPlugin;

#[derive(Component)]
pub struct MainCamera;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(FreeCameraPlugin)
            .add_systems(Update, close_on_escape)
            .add_systems(Update, toggle_camera);
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
    if input.just_pressed(KeyCode::KeyT){
        let Ok((camera_entity, is_free_cam)) = camera_query.single() else {
            return;
        };
        let Ok(player_entity) = player_query.single() else{
            return;
        };

        if is_free_cam {
            commands
                .entity(camera_entity)
                .remove::<FreeCamera>()
                .insert((
                    tp_cam::ThirdPersonCamera::aimed_at(player_entity),
                    tp_cam::DampingFactor(5.0),
                    ));
        } else {
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