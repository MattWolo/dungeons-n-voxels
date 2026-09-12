use std::f32::consts::{FRAC_PI_2, TAU};
use bevy::camera::visibility::RenderLayers;
use bevy::light::light_consts::lux::{AMBIENT_DAYLIGHT, MOONLESS_NIGHT};
use bevy::prelude::*;
use bevy::mesh::primitives::PlaneMeshBuilder;
use bevy_sky_gradient::ambient_driver::{AmbientDriverPlugin, AmbientSettings};
use bevy_sky_gradient::aurora::{AuroraPlugin, AuroraSettings};
use bevy_sky_gradient::cycle::SkyCyclePlugin;
use bevy_sky_gradient::gradient_driver::GradientDriverPlugin;
use bevy_sky_gradient::plugin::{GradientTextureHandle, SkyPlugin, SkySettings};
use bevy_sky_gradient::prelude::{SunDriverPlugin, SunSettings, SkyTimeSettings};
use bevy_sky_gradient::sky_texture::{SkyTexturePlugin, SkyTexturePluginSettings};
use bevy_sky_gradient::sun::SunDriverTag;
use crate::controls::MainCamera;
use crate::generation::moon_material::{advance_lunar_clock, LunarClock, MoonDirection, MoonMaterial, MoonMaterialUniforms};
use crate::player::Player;
use crate::generation::voxel_material::{force_material_update};

#[derive(Component)]
pub struct Moon;

#[derive(Component)]
struct MoonSpotlight;

pub fn spawn_moon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MoonMaterial>>,
) {
    let moon_direction = Vec3::new(0.6, 0.3, 0.75).normalize();
    commands.spawn((
        Mesh3d(meshes.add(
            PlaneMeshBuilder::new(Dir3::Z, Vec2::new(60.0, 60.0)).build()
        )),
        MeshMaterial3d(materials.add(MoonMaterial {
            uniforms: MoonMaterialUniforms {
                moon_dir: moon_direction,
                color: LinearRgba::new(0.8, 1.1, 2.0, 1.0),
                angular_radius: 0.01,
                softness: 0.01,
            }
        })),
        Transform::from_translation(Vec3::Z * 500.0),
        RenderLayers::default(),
        Moon,
        ));

        commands.spawn((
            DirectionalLight {
                color: Color::srgb(0.8, 1.1, 2.0),
                illuminance: MOONLESS_NIGHT,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_translation(moon_direction * 100.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    commands.spawn((
        SpotLight {
            color: Color::srgb(0.6, 0.6, 1.2),
            intensity: 0.0,
            range: 150.0,
            outer_angle: 0.8,
            inner_angle: 0.5,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0)
            .looking_at(Vec3::ZERO, Vec3::Z),
        MoonSpotlight,
    ));
}

fn update_moon_direction(
    sun_query: Query<&Transform, With<SunDriverTag>>,
    mut moon_dir: ResMut<MoonDirection>,
) {
    if let Ok(sun_transform) = sun_query.single() {
        let dir = sun_transform.forward().normalize();
        moon_dir.dir = dir;

        if dir.y >= 0.0 {
            let height = dir.y.clamp(0.0, 1.0);
            if dir.z >= 0.0 {
                // Sunset (0.0) -> Midnight (0.5)
                moon_dir.night_progress = 0.5 * height;
            } else {
                // Midnight (0.5) -> Sunrise (1.0)
                moon_dir.night_progress = 0.5 + 0.5 * (1.0 - height);
            }
        } else {
            moon_dir.night_progress = 0.0;
        }
    }
}

fn update_moon_material_phase(
    lunar: Res<LunarClock>,
    moon_material_q: Query<&MeshMaterial3d<MoonMaterial>>,
    mut materials: ResMut<Assets<MoonMaterial>>,
) {
    let angle = lunar.phase() * TAU;
    let phase_dir = Vec3::new(angle.sin(), 0.15, -angle.cos()).normalize();

    for handle in &moon_material_q {
        if let Some(mut mat) = materials.get_mut(handle) {
            mat.uniforms.moon_dir = phase_dir;
        }
    }
}

fn update_moon_phase_light(
    moon_direction: Res<MoonDirection>,
    lunar: Res<LunarClock>,
    player_q: Query<&Transform, (With<Player>, Without<MoonSpotlight>)>,
    mut spot_q: Query<(&mut SpotLight, &mut Transform), (With<MoonSpotlight>, Without<Player>)>,
) {
    let Ok(player_transform) = player_q.single() else { return; };

    let progress = moon_direction.night_progress;
    let start_threshold = 0.99;
    let target_threshold = 0.5;
    let fade_start = 0.49;
    let fade_end = 0.15;
    let t_in = ((progress - start_threshold) / (target_threshold - start_threshold)).clamp(0.0, 1.0);
    let t_out = 1.0 - ((progress - fade_start) / (fade_end - fade_start)).clamp(0.0, 1.0);
    let t = t_in.min(t_out);
    let night_eased = t * t * (3.0 - 2.0 * t);
    let raw_progress = (lunar.illumination() - 0.85) / (0.98 - 0.85);
    let full_moon_gate = SmoothStepCurve.sample_clamped(raw_progress);
    let eased = night_eased * full_moon_gate;
    for (mut spot, mut transform) in &mut spot_q {
        spot.intensity = eased * 20_000_000.0;
        spot.color = Color::LinearRgba(LinearRgba::new(
            0.20 + 0.20 * eased,
            0.20 + 0.35 * eased,
            1.0 + 0.60 * eased,
            1.0,
        ));
        let anchor = player_transform.translation + Vec3::new(0.0, 20.0, 0.0);
        *transform = Transform::from_translation(anchor)
            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2));
    }
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

    moon_transform.translation = camera_transform.translation + (moon_dir.dir * distance);

    moon_transform.rotation = Quat::from_rotation_arc(Vec3::Z, -moon_dir.dir);

    if let Some(mut material) = materials.get_mut(&material_handle.0) {
        material.uniforms.moon_dir = moon_dir.dir;
    }
}

// fn setup(
//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut fog_materials: ResMut<Assets<FogMaterial>>,
//     gradient_texture: Res<GradientTextureHandle>,
// ) {
//     commands.spawn((
//         Mesh3d(meshes.add(Circle::new(90.0))),
//         MeshMaterial3d(fog_materials.add(FogMaterial {
//             settings: FogBindGroup {
//                 color: vec3(0.0, 1.0, 0.0),
//                 ..default()
//             },
//             sky_texture: gradient_texture.render_target.clone(),
//         })),
//         Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
//         ));
//
//     let mut rng = rand::rng();
//     for _ in 0..100 {
//         let x = rng.random_range(-80.0..80.0);
//         let z = rng.random_range(-80.0..80.0);
//         let scale = rng.random_range(1.0..8.0);
//
//         commands.spawn((
//             Mesh3d(meshes.add(Cuboid::default())),
//             MeshMaterial3d(fog_materials.add(FogMaterial {
//                 settings: FogBindGroup {
//                     color: vec3(1.0, 0.0, 0.0),
//                     ..default()
//                 },
//                 sky_texture: gradient_texture.render_target.clone(),
//             })),
//             Transform::from_xyz(x, scale * 0.5, z).with_scale(Vec3::splat(scale)),
//             ));
//     }
// }

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {

        app
            .insert_resource(SkySettings {
                // spawn_default_skybox: true,
                // camera_gradient_order: -3,
                // skybox_gradient_render_layer: RenderLayers::layer(6),
                ..default()
            })
            .insert_resource(LunarClock{..default()})
            .insert_resource(SkyTexturePluginSettings {
                // sky_render_layer: RenderLayers::layer(8),
                // full_sky_camera_order: -2,
                // final_camera_order: -1,
                ..default()
            })
            .init_resource::<MoonDirection>()
            .add_plugins(MaterialPlugin::<MoonMaterial>::default())
            //.add_plugins(SkyTexturePlugin::default())
            //.add_plugins((EguiPlugin::default(), WorldInspectorPlugin::default()))
            .add_plugins(
                SkyPlugin::builder()
                    .with_render_sky_to_texture()
                    .set_sun_driver(SunDriverPlugin {
                        spawn_default_sun_light: true,
                        sun_settings: SunSettings {
                            illuminance: AMBIENT_DAYLIGHT,
                            sun_strength: 1.5,
                            sun_sharpness: 400.0,
                            sun_color: vec4(1.0, 1.0, 0.5, 1.0),
                            ..default()
                        },
                    })
                    .set_aurora(AuroraPlugin {
                        aurora_settings: AuroraSettings {
                            render_texture_percent: 0.0,
                            camera_render_layers: RenderLayers::none(),
                            ..default()
                        },
                    })
                        .set_cycle(SkyCyclePlugin {
                        sky_time_settings: SkyTimeSettings {
                            day_time_sec: 10.0,
                            night_time_sec: 30.0,
                            sunrise_time_sec: 50.0,
                            sunset_time_sec: 50.0,
                        },
                        sky_time: Default::default(),
                    })
                    .set_gradient_driver(GradientDriverPlugin::default())
                    .set_ambient_driver(AmbientDriverPlugin::default())
                    .build()
            )
            .add_systems(Startup, (
                spawn_moon,
                //setup,
                //setup_egui_render_layer
            ))
            .add_systems(Update, force_material_update)
            .add_systems(Update, (
                advance_lunar_clock,
                update_moon_direction,
                apply_moon_direction,
                update_moon_material_phase,
                update_moon_phase_light,
                ).chain());
    }
}