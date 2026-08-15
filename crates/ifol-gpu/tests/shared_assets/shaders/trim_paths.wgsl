struct TrimPathsUniform {
    center: vec2<f32>,
    half_size: vec2<f32>,
    corner_radius: f32,
    thickness: f32,
    dash_length: f32,
    gap_length: f32,
    dash_offset: f32,
    trim_start: f32, // 0.0 to 1.0
    trim_end: f32,   // 0.0 to 1.0
    stroke_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: TrimPathsUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[vi], 0.0, 1.0);
    out.uv = uv[vi];
    return out;
}

// SDF for Rounded Box
fn sdf_rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

// Approximate perimeter angle / parameter [0.0 to 1.0] around rounded rectangle
fn get_perimeter_t(p: vec2<f32>) -> f32 {
    let angle = atan2(p.y, p.x); // -PI to PI
    var norm_angle = (angle + 3.14159265359) / (2.0 * 3.14159265359);
    return norm_angle;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    let p = in.uv - u_params.center;
    let dist = sdf_rounded_box(p, u_params.half_size, u_params.corner_radius);
    
    // Distance to the stroke boundary
    let stroke_dist = abs(dist) - (u_params.thickness * 0.5);
    let is_on_stroke = 1.0 - smoothstep(-0.0015, 0.0015, stroke_dist);
    
    if (is_on_stroke <= 0.001) {
        return base_color;
    }

    // Calculate parameter t along the perimeter (0.0 to 1.0)
    let t = get_perimeter_t(p);

    // Check Trim Start and Trim End
    var trim_visible = 0.0;
    if (t >= u_params.trim_start && t <= u_params.trim_end) {
        trim_visible = 1.0;
    }

    // Check Dashes & Gaps
    let total_pattern = u_params.dash_length + u_params.gap_length;
    let pattern_pos = (t * 50.0 + u_params.dash_offset) % total_pattern;
    let is_dash = step(pattern_pos, u_params.dash_length);

    let stroke_alpha = is_on_stroke * is_dash * trim_visible * u_params.stroke_color.a;

    // Glowing stroke
    let glow_dist = abs(dist);
    let glow = exp(-glow_dist * 120.0) * 0.45 * trim_visible;

    let final_rgb = mix(base_color.rgb, u_params.stroke_color.rgb, stroke_alpha) + u_params.stroke_color.rgb * glow;
    return vec4<f32>(final_rgb, max(base_color.a, stroke_alpha));
}
