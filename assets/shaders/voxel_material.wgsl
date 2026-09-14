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

fn hash12(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    let a = hash12(i);
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(a, b, u.x),
        mix(c, d, u.x),
        u.y
    );
}

fn fbm2(p: vec2<f32>) -> f32 {
    let n1 = value_noise(p);
    let n2 = value_noise(p * 2.03 + vec2<f32>(17.2, 8.3)) * 0.5;
    let n3 = value_noise(p * 4.11 + vec2<f32>(91.7, 42.6)) * 0.25;

    return (n1 + n2 + n3) / 1.75;
}

fn three_color_ramp(
    t: f32,
    dark: vec3<f32>,
    mid: vec3<f32>,
    bright: vec3<f32>,
) -> vec3<f32> {
    if (t < 0.5) {
        return mix(dark, mid, t * 2.0);
    }
    return mix(mid, bright, (t - 0.5) * 2.0);
}

fn terrain_patch_tint(
    base_rgb: vec3<f32>,
    world_pos: vec3<f32>,
    world_normal: vec3<f32>,
) -> vec3<f32> {
    let voxel_xz = floor(world_pos.xz + vec2<f32>(0.5, 0.5));

    //lower values = larger patches
    let macro_noise = fbm2(voxel_xz * 0.03);
    let detail_noise = fbm2(voxel_xz * 0.11 + vec2<f32>(31.7, 19.4));

    let patchSpot = clamp(macro_noise * 0.8 + detail_noise * 0.2, 0.0, 1.0);

    //quantize to stylized bands
    //let stepped = floor(patchSpot * 3.0) / 2.0;
    let stepped = smoothstep(0.2, 0.8, patchSpot);

    let max_c = max(base_rgb.r, max(base_rgb.g, base_rgb.b));
    let min_c = min(base_rgb.r, min(base_rgb.g, base_rgb.b));
    let saturation = max_c - min_c;

    //snow detector
    let whiteness = smoothstep(0.7, 1.0, min_c) * (1.0 - smoothstep(0.0, 0.2, saturation));

    //grass detector
    let green_strength = smoothstep(0.05, 0.25, base_rgb.g - max(base_rgb.r, base_rgb.b));

    //color ramps, soften or exaggerate tint
    let grass_dark = vec3<f32>(0.30, 0.96, 0.30);
    let grass_mid = vec3<f32>(1.00, 1.00, 1.00);
    let grass_bright = vec3<f32>(1.15, 1.30, 0.30);

    let snow_dark = vec3<f32>(0.30, 0.30, 1.00);
    let snow_mid = vec3<f32>(1.00, 1.00, 1.00);
    let snow_bright = vec3<f32>(1.92, 1.98, 2.07);

    let grass_tint =
        three_color_ramp(
            stepped,
            grass_dark,
            grass_mid,
            grass_bright,
        );

    let snow_tint =
        three_color_ramp(
            stepped,
            snow_dark,
            snow_mid,
            snow_bright,
        );

    let material_tint = mix(grass_tint, snow_tint, whiteness);

    let topness = smoothstep(0.55, 0.95, normalize(world_normal).y);

    let terrain_mask = max(whiteness, green_strength) * topness;

    return mix(
        vec3<f32>(1.0, 1.0, 1.0),
        material_tint,
        terrain_mask,
    );
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

    let patch_tint = terrain_patch_tint(
        pbr_input.material.base_color.rgb,
        in.world_position.xyz,
        in.world_normal,
    );

    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * patch_tint,
        pbr_input.material.base_color.a
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