struct FlareUniform {
    threshold: f32,
    streak_length: f32, // Horizontal spread
    intensity: f32,
    tint_color: vec4<f32>, // Anamorphic blue tint (e.g. 0.2, 0.6, 1.0, 1.0)
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: FlareUniform;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSampleLevel(t_diffuse, s_diffuse, in.uv, 0.0);
    
    // 1D Horizontal wide streak accumulation
    var streak = vec3<f32>(0.0);
    var total_weight = 0.0;
    let step = u_params.streak_length * 0.004;

    for (var i = -16; i <= 16; i++) {
        let offset_x = f32(i) * step;
        let sample_uv = in.uv + vec2<f32>(offset_x, 0.0);

        // Edge boundary clamping & smooth falloff to avoid hard edge smearing
        if (sample_uv.x >= 0.0 && sample_uv.x <= 1.0) {
            let edge_fade = smoothstep(0.0, 0.05, sample_uv.x) * smoothstep(1.0, 0.95, sample_uv.x);
            let s_color = textureSampleLevel(t_diffuse, s_diffuse, sample_uv, 0.0);
            
            // Extract bright highlights
            let lum = dot(s_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
            if (lum > u_params.threshold) {
                // Exponential decay weight
                let dist = abs(f32(i));
                let w = exp(-dist * 0.12) * edge_fade;
                
                // Anamorphic spectral dispersion (R, G, B slight offset)
                let dispersion = vec3<f32>(1.0 + f32(i)*0.01, 1.0, 1.0 - f32(i)*0.01);
                streak += (s_color.rgb - vec3<f32>(u_params.threshold)) * u_params.tint_color.rgb * dispersion * w;
                total_weight += w;
            }
        }
    }

    let final_streak = streak * u_params.intensity;
    
    // Additive mix over original frame
    let combined = base_color.rgb + final_streak;
    return vec4<f32>(combined, base_color.a);
}
