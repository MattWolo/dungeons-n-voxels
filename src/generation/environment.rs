use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::mesh::primitives::PlaneMeshBuilder;
use bevy_sky_gradient::ambient_driver::AmbientDriverPlugin;
use bevy_sky_gradient::aurora::{AuroraPlugin, AuroraSettings};
use bevy_sky_gradient::cycle::SkyCyclePlugin;
use bevy_sky_gradient::gradient_driver::GradientDriverPlugin;
use bevy_sky_gradient::plugin::{SkyPlugin, SkySettings};
use bevy_sky_gradient::prelude::{SunDriverPlugin, SunSettings, SkyTimeSettings};
use bevy_sky_gradient::sky_texture::{FullSkyCameraTag, SkyTexturePlugin, SkyTexturePluginSettings};
use bevy_sky_gradient::sun::SunDriverTag;
use crate::controls::MainCamera;
use crate::generation::moon_material::{MoonDirection, MoonMaterial, MoonMaterialUniforms};
#[derive(Component)]
pub struct Moon;

pub fn spawn_moon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MoonMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(
            PlaneMeshBuilder::new(Dir3::Z, Vec2::new(80.0, 80.0)).build()
        )),
        MeshMaterial3d(materials.add(MoonMaterial {
            uniforms: MoonMaterialUniforms {
                moon_dir: Vec3::ZERO,
                color: LinearRgba::new(3.0, 3.0, 3.2, 1.0),
                angular_radius: 0.01,
                softness: 0.03,
            }
        })),
        Transform::from_translation(Vec3::Z * 500.0),
        RenderLayers::default(),
        Moon,
        ));
    println!("MoonSpawned");
}

fn update_moon_direction(
    sun_query: Query<&Transform, With<SunDriverTag>>,
    mut moon_dir: ResMut<MoonDirection>,
) {
    if let Ok(sun_transform) = sun_query.single() {
        moon_dir.0 = sun_transform.forward().normalize();
    }
    println!("Moon dir {}", moon_dir.0);
}

fn apply_moon_direction(
    moon_dir: Res<MoonDirection>,
    main_camera_query: Query<&Transform, With<MainCamera>>,
    mut materials: ResMut<Assets<MoonMaterial>>,
    mut moon_query: Query<(&mut Transform, &MeshMaterial3d<MoonMaterial>), (With<Moon>, Without<MainCamera>)>,
) {
    let Ok(camera_transform) = main_camera_query.single() else { return };
    let Ok((mut moon_transform, material_handle)) = moon_query.single_mut() else { return };

    let distance = 900.0;

    moon_transform.translation = camera_transform.translation + (moon_dir.0 * distance);

    moon_transform.rotation = Quat::from_rotation_arc(Vec3::Z, -moon_dir.0);

    if let Some(mut material) = materials.get_mut(&material_handle.0) {
        material.uniforms.moon_dir = moon_dir.0;
    }
}

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(SkySettings {
                spawn_default_skybox: true,
                camera_gradient_order: -2,
                skybox_gradient_render_layer: RenderLayers::layer(8),
                ..default()
            })
            .insert_resource(SkyTexturePluginSettings {
                sky_render_layer: RenderLayers::layer(8),
                full_sky_camera_order: -2,
                final_camera_order: -1,
            })
            .init_resource::<MoonDirection>()
            .add_plugins(MaterialPlugin::<MoonMaterial>::default())
            .add_plugins(SkyTexturePlugin::default())
            .add_systems(Update, sync_sky_camera_with_main)
            .add_plugins(
                SkyPlugin::builder()
                    .set_sun_driver(SunDriverPlugin {
                        spawn_default_sun_light: true,
                        sun_settings: SunSettings {
                            illuminance: 1000.0,
                            sun_strength: 0.6,
                            sun_color: vec4(1.0, 1.0, 0.0, 1.0),
                            ..default()
                        },
                    })
                    .set_aurora(AuroraPlugin {
                        aurora_settings: AuroraSettings {
                            render_texture_percent: 0.0,
                            camera_render_layers: RenderLayers::none(),
                            camera_order: -2,
                        },
                    })
                    .set_cycle(SkyCyclePlugin {
                        sky_time_settings: SkyTimeSettings {
                            day_time_sec: 20.0,
                            night_time_sec: 20.0,
                            sunrise_time_sec: 6.0,
                            sunset_time_sec: 6.0,
                        },
                        sky_time: Default::default(),
                    })
                    .set_gradient_driver(GradientDriverPlugin::default())
                    .set_ambient_driver(AmbientDriverPlugin::default())
                    .build()
            )
            .add_systems(Startup, spawn_moon)
            .add_systems(Update, (
                update_moon_direction,
                apply_moon_direction.after(update_moon_direction),
            ));
    }
}

fn sync_sky_camera_with_main(
    main_camera: Query<(&Transform, &Projection), With<MainCamera>>,
    mut sky_camera: Query<(&mut Transform, &mut Projection), (With<FullSkyCameraTag>, Without<MainCamera>)>,
) {
    if let Ok((main_transform, main_proj)) = main_camera.single() {
        if let Ok((mut sky_transform, mut sky_proj)) = sky_camera.single_mut() {
            sky_transform.rotation = main_transform.rotation;
            *sky_proj = main_proj.clone();
        }
    }
}