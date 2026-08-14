struct MoonUniform {
    model_view: mat4x4<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    key_color: vec3<f32>,
    tolerance: f32,
    smoothness: f32,
    noise_strength: f32,
    glow_intensity: f32,
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) norm_uv: vec2<f32>,
};

@group(0) @binding(0) var t_props: texture_2d<f32>;
@group(0) @binding(1) var s_props: sampler;

@group(1) @binding(0) var t_noise: texture_2d<f32>;
@group(1) @binding(1) var s_noise: sampler;

@group(2) @binding(0) var<uniform> moon: MoonUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var quad_pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var norm_uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    var out: VertexOutput;
    let world_pos = moon.model_view * vec4<f32>(quad_pos[vi], 0.0, 1.0);
    out.clip_position = world_pos;
    out.uv = mix(moon.uv_min, moon.uv_max, norm_uv[vi]);
    out.norm_uv = norm_uv[vi];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let raw_color = textureSample(t_props, s_props, in.uv);

    // Dynamic Chroma Key with Despill
    let dist = distance(raw_color.rgb, moon.key_color);
    let edge_low = moon.tolerance - moon.smoothness * 0.5;
    let edge_high = moon.tolerance + moon.smoothness * 0.5;

    if (dist < edge_low) {
        discard;
    }

    let alpha = smoothstep(edge_low, edge_high, dist);

    // Large-scale organic domain-warped lunar maria (Sea of Serenity, Ocean of Storms)
    let uv = in.norm_uv;
    let warp_x = textureSample(t_noise, s_noise, uv * 0.5 + vec2<f32>(0.15, 0.25)).r;
    let warp_y = textureSample(t_noise, s_noise, uv * 0.5 + vec2<f32>(0.45, 0.65)).r;
    let warp = vec2<f32>(warp_x - 0.5, warp_y - 0.5) * 0.40;

    let macro_noise = textureSample(t_noise, s_noise, uv * 0.70 + warp).r;
    let micro_noise = textureSample(t_noise, s_noise, uv * 2.0 + warp * 0.5).r;
    let combined_lunar = macro_noise * 0.70 + micro_noise * 0.30;

    // Organic Maria pattern
    let maria_mask = smoothstep(0.30, 0.70, combined_lunar);

    // Balanced Contrast Palette:
    // Dark Maria: Mysterious slate-blue basalt plains [0.38, 0.45, 0.60]
    // Luminous Highlands: Radiant glowing pearl white [1.02, 1.06, 1.25]
    let dark_maria = vec3<f32>(0.38, 0.45, 0.60);
    let bright_highland = vec3<f32>(1.02, 1.06, 1.25);
    var surface = mix(dark_maria, bright_highland, maria_mask);

    // 3D Spherical Normal Lighting - Emissive Moon
    let centered = (in.norm_uv - vec2<f32>(0.5, 0.5)) * 2.0;
    let r_sq = dot(centered, centered);
    if (r_sq < 1.0) {
        let z = sqrt(1.0 - r_sq);
        let sphere_shade = 0.85 + 0.15 * z;
        surface = surface * sphere_shade;
    }

    // Outer rim corona glow (radiates light outward)
    let radial_dist = length(centered);
    let rim_halo = smoothstep(0.50, 0.98, radial_dist) * 0.35;
    surface = surface + vec3<f32>(0.75, 0.90, 1.30) * rim_halo;

    return vec4<f32>(surface * moon.glow_intensity, alpha);
}
