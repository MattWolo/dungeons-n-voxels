#import bevy_pbr::forward_io::VertexOutput

struct MoonMaterialUniforms {
    color: vec4<f32>,
    moon_dir: vec3<f32>,
    angular_radius: f32,
    softness: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> uniforms: MoonMaterialUniforms;

// Hash for 3D Noise
fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

// 3D Simplex-style Gradient Noise
fn noise3D(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let n000 = dot(hash33(i + vec3(0.,0.,0.)) - vec3(0.5), f - vec3(0.,0.,0.));
    let n100 = dot(hash33(i + vec3(1.,0.,0.)) - vec3(0.5), f - vec3(1.,0.,0.));
    let n010 = dot(hash33(i + vec3(0.,1.,0.)) - vec3(0.5), f - vec3(0.,1.,0.));
    let n110 = dot(hash33(i + vec3(1.,1.,0.)) - vec3(0.5), f - vec3(1.,1.,0.));
    let n001 = dot(hash33(i + vec3(0.,0.,1.)) - vec3(0.5), f - vec3(0.,0.,1.));
    let n101 = dot(hash33(i + vec3(1.,0.,1.)) - vec3(0.5), f - vec3(1.,0.,1.));
    let n011 = dot(hash33(i + vec3(0.,1.,1.)) - vec3(0.5), f - vec3(0.,1.,1.));
    let n111 = dot(hash33(i + vec3(1.,1.,1.)) - vec3(0.5), f - vec3(1.,1.,1.));

    let lx = mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y);
    let hx = mix(mix(n001, n101, u.x), mix(n011, n111, u.x), u.y);
    return mix(lx, hx, u.z) + 0.5;
}

fn fbm3D(p: vec3<f32>) -> f32 {
    var val = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i = 0; i < 4; i = i + 1) {
        val += amp * noise3D(pos);
        pos = pos * 2.04;
        amp *= 0.5;
    }
    return val;
}

// Shared "seas vs highlands" mask, driven by domain-warped fbm.
// Used for BOTH the terrain height and the surface albedo, so the dark
// maria patches actually line up with where the ground sits lower
// (previously these were computed two different ways and drifted apart).
fn get_maria_mask(p: vec3<f32>) -> f32 {
    let warp = vec3<f32>(
        fbm3D(p * 0.8 + vec3<f32>(0.0, 0.0, 0.0)),
        fbm3D(p * 0.8 + vec3<f32>(5.2, 1.3, 2.8)),
        fbm3D(p * 0.8 + vec3<f32>(1.7, 9.2, 3.4))
    );
    return smoothstep(0.42, 0.58, fbm3D(p * 0.6 + warp * 0.8));
}

// Individual crater profile with central peak, depression, and rim
fn single_crater(p: vec3<f32>, center: vec3<f32>, radius: f32) -> f32 {
    let d = length(p - center) / radius;
    if (d > 1.5) { return 0.0; }

    let pit = -smoothstep(0.85, 0.0, d) * 0.3;
    let central_peak = smoothstep(0.2, 0.0, d) * 0.09;

    // Sharp raised rim: a NARROW rise then a NARROW fall makes a thin,
    // crisp ridge (steep gradient = catches light as a hard edge) instead
    // of the old wide 0.6-1.3 dome, which spread the height change over
    // too much distance to ever look sharp.
    let rim_rise = smoothstep(0.80, 0.94, d);
    let rim_fall = smoothstep(1.12, 0.98, d);
    let rim = rim_rise * rim_fall * 0.55;

    return rim + pit + central_peak;
}

// Rays extending from major impact sites
fn crater_rays(p: vec3<f32>, center: vec3<f32>) -> f32 {
    let dir = normalize(p - center);
    let dist = length(p - center);
    let ray_noise = noise3D(dir * 12.0);
    return pow(ray_noise, 3.0) * exp(-dist * 1.2) * 0.4;
}

// Height used for COLOR ONLY - can stay rich/detailed since it never
// touches the surface normal, so extra fine noise here can't turn into
// visual static.
fn get_moon_color_height(p: vec3<f32>) -> f32 {
    let maria_mask = get_maria_mask(p);
    let highlands = fbm3D(p * 2.5) * 0.3;
    var h = mix(highlands, 0.05 + highlands * 0.2, maria_mask);

    let c1 = vec3<f32>(-0.3, -0.4, 0.8);
    let c2 = vec3<f32>(0.4, 0.2, 0.7);
    let c3 = vec3<f32>(-0.2, 0.5, 0.8);
    let c4 = vec3<f32>(0.15, -0.55, 0.75);

    h += single_crater(p, c1, 0.35);
    h += single_crater(p, c2, 0.25);
    h += single_crater(p, c3, 0.18);
    h += single_crater(p, c4, 0.14);
    h += crater_rays(p, c1);

    return h;
}

// Height used ONLY to build the shading normal. The base terrain term is
// deliberately much lower amplitude/frequency here than in the color
// height above - that's what stops the whole moon from shading like
// fuzzy static. The craters (which keep full strength) end up reading
// as the dominant relief feature instead of getting drowned out.
fn get_moon_bump_height(p: vec3<f32>) -> f32 {
    var h = fbm3D(p * 1.1) * 0.07;

    let c1 = vec3<f32>(-0.3, -0.4, 0.8);
    let c2 = vec3<f32>(0.4, 0.2, 0.7);
    let c3 = vec3<f32>(-0.2, 0.5, 0.8);
    let c4 = vec3<f32>(0.15, -0.55, 0.75);

    h += single_crater(p, c1, 0.35);
    h += single_crater(p, c2, 0.25);
    h += single_crater(p, c3, 0.18);
    h += single_crater(p, c4, 0.14);
    h += crater_rays(p, c1) * 0.6;

    return h;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p2d = (in.uv - vec2<f32>(0.5)) * 2.0;
    let dist = length(p2d);
    let moon_radius = 0.78; // Slightly reduced radius to leave room for the outer blue halo

    // Define Blue Luminance Palette
    let atmospheric_blue = vec3<f32>(0.35, 0.65, 0.80) * uniforms.color.rgb;

    // 1. Outer Atmospheric Blue Glow (Renders outside the moon disk)
    if (dist > moon_radius) {
        let glow_dist = (dist - moon_radius) / (1.0 - moon_radius);
        let glow_intensity = exp(-glow_dist * 4.5) * 0.35;
        let glow_rgb = atmospheric_blue * glow_intensity * 1.2;
        return vec4<f32>(glow_rgb, glow_intensity);
    }

    // 2. Sphere Surface Mapping
    let r = dist / moon_radius;
    let z = sqrt(max(0.0, 1.0 - r * r));
    let sphere_normal = vec3<f32>(p2d.x / moon_radius, -p2d.y / moon_radius, z);
    let sample_pos = sphere_normal * 2.2 + vec3<f32>(12.0, 5.0, 8.0);

    // 3. Surface Normal Perturbation - uses the dedicated bump-only height field
    let eps = 0.015;
    let hb_center = get_moon_bump_height(sample_pos);
    let hb_x = get_moon_bump_height(sample_pos + vec3<f32>(eps, 0.0, 0.0));
    let hb_y = get_moon_bump_height(sample_pos + vec3<f32>(0.0, eps, 0.0));
    let hb_z = get_moon_bump_height(sample_pos + vec3<f32>(0.0, 0.0, eps));

    let grad = vec3<f32>(hb_x - hb_center, hb_y - hb_center, hb_z - hb_center) / eps;
    let bumped_normal = normalize(sphere_normal - grad * 0.35);

    // 4. Base Surface Colors (Maria vs Highlands) - same mask as the terrain now
    let color_height = get_moon_color_height(sample_pos);
    let maria_val = get_maria_mask(sample_pos);
    let albedo_base = mix(vec3<f32>(0.85, 0.88, 0.92), vec3<f32>(0.28, 0.30, 0.34), maria_val);
    let final_albedo = albedo_base + vec3<f32>(color_height * 0.55);

    // 5. Directional Lighting with a crisper terminator band
    let light_dir = normalize(uniforms.moon_dir);
    let raw_ndotl = dot(bumped_normal, light_dir);
    let terminator = smoothstep(0.0, 0.28, raw_ndotl); // widen/narrow this range to taste
    let lighting = mix(0.14, 1.0, terminator);

    // 6. Inner Rim Scattering (Atmospheric blue tint around the edge)
    let rim_scatter = pow(1.0 - z, 4.0) * atmospheric_blue * 0.6;

    // Smooth anti-aliased edge
    let alpha = 1.0 - smoothstep(0.97, 1.0, r);

    // Final color composition (Surface + Rim Scattering)
    let surface_rgb = (uniforms.color.rgb * final_albedo * lighting) + rim_scatter;

    return vec4<f32>(surface_rgb * alpha, alpha);
}
