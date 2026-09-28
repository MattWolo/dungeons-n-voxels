#define VERTEX_COLORS
#define VERTEX_OUTPUT_INSTANCE_INDEX

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

const VOXEL_GRASS: u32 = 1u;
const VOXEL_SNOW: u32 = 2u;
const VOXEL_SAND: u32 = 3u;

fn voxel_color(voxel_id: u32) -> vec4<f32> {
    switch voxel_id {
        case VOXEL_GRASS: {
            return vec4<f32>(
                0.2,
                1.0,
                0.2,
                1.0,
            );
        }
        case VOXEL_SNOW: {
            return vec4<f32>(
                0.72,
                0.74,
                0.78,
                1.0,
            );
        }
        case VOXEL_SAND: {
            return vec4<f32>(
                1.0,
                1.0,
                0.5,
                1.0,
            );
        }
        default: {
            return vec4<f32>(
                1.0,
                0.0,
                1.0,
                1.0,
            );
        }
    }
}

fn is_grass(voxel_id: u32) -> f32 {
    if (voxel_id == VOXEL_GRASS) {
        return 1.0;
    }
    return 0.0;
}

fn is_snow(voxel_id: u32) -> f32 {
    if (voxel_id == VOXEL_SNOW) {
        return 1.0;
    }
    return 0.0;
}

fn is_sand(voxel_id: u32) -> f32 {
    if (voxel_id == VOXEL_SAND) {
        return 1.0;
    }
    return 0.0;
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

fn terrain_noise(p: vec2<f32>) -> f32 {
    let warp_x = fbm2(
        p * 0.018 + vec2<f32>(17.4, 91.7)
    );

    let warp_y = fbm2(
        p * 0.018 + vec2<f32>(63.1, 24.8)
    );

    let warp = (vec2<f32>(warp_x, warp_y) * 2.0 - vec2<f32>(1.0)) * 10.0;
    let warped_p = p + warp;

    let macro_noise = fbm2(warped_p * 0.03);
    let detail_noise = fbm2(warped_p * 0.11 + vec2<f32>(31.7, 19.4));

    return clamp(macro_noise * 0.85 + detail_noise * 0.15, 0.0, 1.0);
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
    voxel_id: u32,
    world_pos: vec3<f32>,
    world_normal: vec3<f32>,
) -> vec3<f32> {
    let patchSpot = terrain_noise(world_pos.xz);

    //quantize to stylized bands
    //let stepped = floor(patchSpot * 3.0) / 2.0;
    let stepped = smoothstep(0.2, 0.8, patchSpot);

    let grass_mask = is_grass(voxel_id);
    let snow_mask = is_snow(voxel_id);

    //color ramps, soften or exaggerate tint
    let grass_dark = vec3<f32>(0.30, 0.96, 0.30);
    let grass_mid = vec3<f32>(1.00, 1.00, 1.00);
    let grass_bright = vec3<f32>(1.00, 1.00, 0.55);

    let snow_dark = vec3<f32>(0.65, 0.65, 0.65);
    let snow_mid = vec3<f32>(0.82, 0.83, 0.85);
    let snow_bright = vec3<f32>(0.90, 0.90, 1.00);

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

    let material_tint = mix(grass_tint, snow_tint, snow_mask);

    let topness = smoothstep(0.55, 0.95, normalize(world_normal).y);

    let terrain_mask = max(grass_mask, snow_mask) * topness;

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
    @location(8) voxel_data: u32,
}

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

    let voxel_id = in.voxel_data & 0xffu;
    let ao_occlusion = (in.voxel_data >> 8u) & 0x3u;
    let ao = 1.0 - f32(ao_occlusion) / 3.0;

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

    // Temporary data carrier:
    // R = AO
    // G = voxel ID / 255
    // B = unused
    // A = always opaque

    out.color = vec4<f32>(ao, f32(voxel_id) / 255.0, 0.0, 1.0);

    out.instance_index = in.instance_index;

    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {

    let voxel_id = u32(round(in.color.g * 255.0));
    let ao = clamp(in.color.r, 0.0, 1.0);
    let base_color = voxel_color(voxel_id);

    var pbr_vertex = in;
    pbr_vertex.color = vec4<f32>(1.0);

    var pbr_input = pbr_input_from_standard_material(
        pbr_vertex,
        is_front,
    );

    let patch_tint = terrain_patch_tint(
        voxel_id,
        in.world_position.xyz,
        in.world_normal,
    );

    pbr_input.material.base_color = vec4<f32>(base_color.rgb * patch_tint, 1.0);

    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.metallic = 1.0;
    pbr_input.material.reflectance = vec3<f32>(0.0);

    var out: FragmentOutput;

    out.color = apply_pbr_lighting(pbr_input);

    let ao_visibility = pow(ao, 1.15); //1.0 - fully visible, 0.0 - not
    let ao_amount = 1.0 - ao_visibility;

    let default_ao_tint = vec3<f32>(0.28, 0.28, 0.28);
    let grass_ao_tint = vec3<f32>(0.24, 0.32, 0.20);
    let snow_ao_tint = vec3<f32>(0.32, 0.45, 0.77);
    let sand_ao_tint = vec3<f32>(0.77, 0.45, 0.32);

    var ao_tint = default_ao_tint;

    if (voxel_id == VOXEL_GRASS) {
        ao_tint = grass_ao_tint;
    }

    if (voxel_id == VOXEL_SNOW) {
        ao_tint = snow_ao_tint;
    }

    if (voxel_id == VOXEL_SAND) {
        ao_tint = sand_ao_tint;
    }

    let ao_multiplier = mix(vec3<f32>(1.0), ao_tint, ao_amount);

    out.color = vec4<f32>(out.color.rgb * ao_multiplier, out.color.a);

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
        1.0
    );

    return out;
}