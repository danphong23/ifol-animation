struct CloudUniform {
    model_view: mat4x4<f32>,
    uv_bounds: vec4<f32>,         // min_u, min_v, max_u, max_v
    key_color_tol: vec4<f32>,     // key_r, key_g, key_b, tolerance
    params: vec4<f32>,            // smoothness, depth_softness, opacity, silver_rim_intensity
    lighting_pos: vec4<f32>,      // moon_x, moon_y, cloud_center_x, cloud_center_y
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) norm_uv: vec2<f32>,
    @location(2) world_xy: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> cloud: CloudUniform;

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
    let world_pos = cloud.model_view * vec4<f32>(quad_pos[vi], 0.0, 1.0);
    out.clip_position = world_pos;
    out.uv = mix(cloud.uv_bounds.xy, cloud.uv_bounds.zw, norm_uv[vi]);
    out.norm_uv = norm_uv[vi];
    out.world_xy = world_pos.xy;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let raw = textureSample(t_diffuse, s_diffuse, in.uv);

    let key_color = cloud.key_color_tol.xyz;
    let tolerance = cloud.key_color_tol.w;
    let smoothness = cloud.params.x;
    let depth_softness = cloud.params.y;
    let opacity = cloud.params.z;
    let silver_rim_intensity = cloud.params.w;
    let moon_pos = cloud.lighting_pos.xy;

    // Dynamic Chroma key removal
    let dist = distance(raw.rgb, key_color);
    let edge_low = tolerance - smoothness * 0.5;
    let edge_high = tolerance + smoothness * 0.5;

    if (dist < edge_low) {
        discard;
    }

    let alpha = smoothstep(edge_low, edge_high, dist) * opacity;

    // Green despill filter
    var base_color = raw.rgb;
    let max_rb = max(base_color.r, base_color.b);
    if (base_color.g > max_rb) {
        base_color.g = max_rb;
    }

    // Directional Moonlight Vector from Moon Source (moon_pos)
    let to_moon = normalize(moon_pos - in.world_xy);
    let dist_to_moon = length(moon_pos - in.world_xy);

    // Natural cloud contour orientation
    let local_dir = normalize(in.norm_uv - vec2<f32>(0.5, 0.5));
    let dist_from_center = length(in.norm_uv - vec2<f32>(0.5, 0.5));
    let is_contour_rim = smoothstep(0.20, 0.48, dist_from_center);

    // Moon facing factor (strongest on upper-left rims facing the moon)
    let moon_facing = max(dot(local_dir, to_moon), 0.0);

    // Silver lining highlight along the cloud's moon-lit edge
    let silver_lining = pow(moon_facing, 1.5) * is_contour_rim * silver_rim_intensity;
    let silver_color = vec3<f32>(0.94, 0.97, 1.25);

    // Proximity ambient illumination (clouds near the moon catch gentle moonlight glow)
    let proximity_boost = clamp(1.0 - dist_to_moon * 0.65, 0.0, 1.0) * 0.20;

    // Soft celestial lighting mix
    let lit_color = mix(base_color, silver_color, clamp(silver_lining + proximity_boost, 0.0, 1.0));

    // Shadow side (away from moon) retains rich anime blue tones
    let shadow_color = base_color * vec3<f32>(0.75, 0.80, 0.92);
    var final_lit_cloud = mix(shadow_color, lit_color, smoothstep(0.0, 0.6, moon_facing * is_contour_rim + 0.35));

    // Atmospheric distance haze
    let atmospheric_haze = vec3<f32>(0.015, 0.03, 0.08);
    let final_color = mix(final_lit_cloud, atmospheric_haze, depth_softness * 0.30);

    return vec4<f32>(final_color, alpha);
}
