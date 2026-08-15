struct GlassUniform {
    panel_center: vec2<f32>,
    panel_size: vec2<f32>, // half width, half height
    corner_radius: f32,
    blur_amount: f32,
    refraction_strength: f32,
    border_thickness: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: GlassUniform;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    // Calculate SDF distance to rounded rectangle panel
    let p = in.uv - u_params.panel_center;
    let dist = sdf_rounded_box(p, u_params.panel_size, u_params.corner_radius);
    
    // If outside the glass panel, render background normally
    if (dist > 0.01) {
        // Drop shadow behind glass panel
        let shadow_p = (in.uv - (u_params.panel_center + vec2<f32>(0.01, 0.02)));
        let shadow_dist = sdf_rounded_box(shadow_p, u_params.panel_size, u_params.corner_radius);
        let shadow_alpha = (1.0 - smoothstep(-0.02, 0.05, shadow_dist)) * 0.45;
        return vec4<f32>(mix(base_color.rgb, vec3<f32>(0.0), shadow_alpha), base_color.a);
    }
    
    // Inside glass panel:
    // 1. Calculate glass refraction offset based on distance gradient
    let normal_uv = normalize(p) * (smoothstep(0.0, -0.05, dist) * u_params.refraction_strength);
    let sample_uv = in.uv + normal_uv;

    // 2. Dual-radius Frosted Blur
    var blurred = vec4<f32>(0.0);
    var total_w = 0.0;
    let b_radius = u_params.blur_amount * 0.004;

    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * b_radius;
            let w = exp(-f32(x*x + y*y) / 4.0);
            blurred += textureSample(t_diffuse, s_diffuse, sample_uv + offset) * w;
            total_w += w;
        }
    }
    let frosted_backdrop = blurred / total_w;

    // 3. Glass material tinting (frosted milky glass)
    let glass_tint = vec3<f32>(0.92, 0.95, 1.0);
    let glass_body = mix(frosted_backdrop.rgb, glass_tint, 0.22);

    // 4. Specular Rim Light (Fresnel reflection along the top-left edges)
    let light_dir = normalize(vec2<f32>(-1.0, -1.0));
    let rim_factor = max(dot(normalize(p), light_dir), 0.0);
    let edge_specular = (1.0 - smoothstep(0.0, u_params.border_thickness, abs(dist))) * (0.5 + 0.5 * rim_factor);

    let final_glass = glass_body + vec3<f32>(edge_specular * 0.7);

    // 5. Anti-aliased transition at the boundary
    let edge_blend = 1.0 - smoothstep(-0.002, 0.002, dist);
    let result = mix(base_color.rgb, final_glass, edge_blend);

    return vec4<f32>(result, 1.0);
}
