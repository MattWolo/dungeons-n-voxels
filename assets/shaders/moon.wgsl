#import bevy_pbr::forward_io::VertexOutput

struct MoonMaterialUniforms {
    color: vec4<f32>,
    moon_dir: vec3<f32>,
    angular_radius: f32,
    softness: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: MoonMaterialUniforms;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv - vec2<f32>(0.5, 0.5));
    let radius = 0.45;

    // Hard/masked cutoff at the edge
    if (dist > radius) {
        discard;
    }

    // Solid opaque color (alpha = 1.0) blocks all background stars
    return vec4<f32>(uniforms.color.rgb, 1.0);
}