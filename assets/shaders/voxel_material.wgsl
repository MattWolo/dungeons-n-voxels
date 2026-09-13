#import bevy_pbr::{
    mesh_view_bindings::view,
    utils::coords_to_viewport_uv,

    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{
        alpha_discard,
        apply_pbr_lighting,
        main_pass_post_lighting_processing,
    },

    forward_io::{
        VertexOutput,
        FragmentOutput,
    },
}

#import bevy_pbr::mesh_functions::{
    get_world_from_local,
    mesh_position_local_to_world,
    mesh_position_local_to_clip,
    mesh_normal_local_to_world,
}

struct VertexInput {
    @builtin(instance_index) instance_index: u32,

    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(5) color: vec4<f32>,
    @location(8) ao: f32,
};

struct FogSettings {
    distance_start: f32,
    distance_end: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> fog_settings: FogSettings;

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var sky_gradient_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var sky_gradient_sampler: sampler;

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = get_world_from_local(in.instance_index);

    out.world_position = mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(in.position, 1.0),
    );

    out.position = mesh_position_local_to_clip(
        world_from_local,
        vec4<f32>(in.position, 1.0),
    );

    out.world_normal = mesh_normal_local_to_world(
        in.normal,
        in.instance_index,
    );

    let base_rgb = in.color.rgb;

    //Measure color brightness and saturation to detect white blocks
    let max_c = max(base_rgb.r, max(base_rgb.g, base_rgb.b));
    let min_c = min(base_rgb.r, min(base_rgb.g, base_rgb.b));
    let saturation = max_c - min_c;

    // Whiteness = 1.0 for snow/white, 0.0 for saturated colors like grass
    let whiteness = smoothstep(0.7, 1.0, min_c) * (1.0 - smoothstep(0.0, 0.2, saturation));

    //shadow targets
    let default_shadow = base_rgb * 0.15;
    let snow_blue_shadow = vec3<f32>(0.25, 0.25, 1.25);

    //shadow color based on whiteness
    let shadow_color = mix(default_shadow, snow_blue_shadow, whiteness);

    //AO value (e.g., pow exponent alters shadow sharpness/falloff)
    let ao_curved = pow(clamp(in.ao, 0.0, 1.0), 1.2);

    out.color = vec4<f32>(
        mix(shadow_color, base_rgb, ao_curved),
        in.color.a,
    );

    out.instance_index = in.instance_index;

    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {

    var pbr_input = pbr_input_from_standard_material(
        in,
        is_front,
    );

    pbr_input.material.perceptual_roughness = 1.0;

    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

    var out: FragmentOutput;

    out.color = apply_pbr_lighting(pbr_input);

    out.color = main_pass_post_lighting_processing(
        pbr_input,
        out.color,
    );

    let screen_uv = coords_to_viewport_uv(
        in.position.xy,
        view.viewport,
    );

    let sky_color = textureSample(
        sky_gradient_texture,
        sky_gradient_sampler,
        screen_uv,
    ).rgb;

    let camera_distance = distance(
        in.world_position.xyz,
        view.world_position.xyz,
    );

    let fog_factor = smoothstep(
        fog_settings.distance_start,
        fog_settings.distance_end,
        camera_distance,
    );

    out.color = vec4<f32>(
        mix(out.color.rgb, sky_color, fog_factor),
        out.color.a,
    );

    return out;
}