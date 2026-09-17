use bevy::camera::visibility::RenderLayers;
use bevy::light::light_consts::lux::{AMBIENT_DAYLIGHT};
use bevy::prelude::*;
use bevy_sky_gradient::ambient_driver::{AmbientDriverPlugin};
use bevy_sky_gradient::aurora::{AuroraPlugin, AuroraSettings};
use bevy_sky_gradient::cycle::SkyCyclePlugin;
use bevy_sky_gradient::gradient_driver::GradientDriverPlugin;
use bevy_sky_gradient::plugin::{SkyPlugin, SkySettings};
use bevy_sky_gradient::prelude::{SunDriverPlugin, SunSettings, SkyTimeSettings};
use bevy_sky_gradient::sky_texture::SkyTexturePluginSettings;
use crate::generation::moon_generation::{apply_moon_direction, spawn_moon, update_moon_direction, update_moon_material_phase, update_moon_phase_light};
use crate::generation::moon_material::{advance_lunar_clock, LunarClock, MoonDirection, MoonMaterial};
use crate::generation::voxel_material::{force_material_update};

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {

        app
            .insert_resource(SkySettings {
                ..default()
            })
            .insert_resource(LunarClock{..default()})
            .insert_resource(SkyTexturePluginSettings {
                ..default()
            })
            .init_resource::<MoonDirection>()
            .add_plugins(MaterialPlugin::<MoonMaterial>::default())
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