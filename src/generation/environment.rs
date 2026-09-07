use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_sky_gradient::ambient_driver::AmbientDriverPlugin;
use bevy_sky_gradient::aurora::{AuroraPlugin, AuroraSettings};
use bevy_sky_gradient::cycle::SkyCyclePlugin;
use bevy_sky_gradient::gradient_driver::GradientDriverPlugin;
use bevy_sky_gradient::plugin::{SkyPlugin, SkySettings};
use bevy_sky_gradient::prelude::{SunDriverPlugin, SunSettings, SkyTimeSettings};
use bevy_sky_gradient::sky_texture::{FullSkyCameraTag, SkyTexturePlugin, SkyTexturePluginSettings};
use crate::controls::MainCamera;

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(SkySettings {
                spawn_default_skybox: false,
                camera_gradient_order: -2,
                skybox_gradient_render_layer: RenderLayers::layer(8),
                ..default()
            })
            .insert_resource(SkyTexturePluginSettings {
                sky_render_layer: RenderLayers::layer(8),
                full_sky_camera_order: -2,
                final_camera_order: -1,
            })
            .add_plugins(SkyTexturePlugin::default())
            .add_systems(Update, sync_sky_camera_with_main)
            .add_plugins(
                SkyPlugin::builder()
                    .set_sun_driver(SunDriverPlugin {
                        spawn_default_sun_light: false,
                        sun_settings: SunSettings {
                            sun_color: vec4(1.0, 1.0, 0.0, 1.0),
                            ..default()
                        },
                    })
                    .set_aurora(AuroraPlugin {
                        aurora_settings: AuroraSettings {
                            render_texture_percent: 1.0,
                            camera_render_layers: RenderLayers::none(),
                            camera_order: -2,
                        },
                    })
                    .set_cycle(SkyCyclePlugin {
                        sky_time_settings: SkyTimeSettings {
                            day_time_sec: 30.0,
                            night_time_sec: 30.0,
                            sunrise_time_sec: 6.0,
                            sunset_time_sec: 6.0,
                        },
                        sky_time: Default::default(),
                    })
                    .set_gradient_driver(GradientDriverPlugin::default())
                    .set_ambient_driver(AmbientDriverPlugin::default())
                    .build()
            );
    }
}

fn sync_sky_camera_with_main(
    main_camera: Query<&Transform, With<MainCamera>>,
    mut sky_camera: Query<&mut Transform, (With<FullSkyCameraTag>, Without<MainCamera>)>,
) {
    if let (Ok(main_transform), Ok(mut sky_transform)) = (main_camera.single(), sky_camera.single_mut()) {
        sky_transform.rotation = main_transform.rotation;
    }
}