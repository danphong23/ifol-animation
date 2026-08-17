struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct CRTUniform {
    curvature: vec2<f32>,
    scanline_intensity: f32,
    time: f32,
};

// Integer hash keeps VHS noise deterministic across shader backends.
fn hash_u32(value: u32) -> u32 {
    var h = value;
    h = h ^ (h >> 16u);
    h = h * 2146121005u;
    h = h ^ (h >> 15u);
    h = h * 2221713035u;
    h = h ^ (h >> 16u);
    return h;
}

fn hash01(value: u32) -> f32 {
    return f32(hash_u32(value)) / 4294967295.0;
}

@group(1) @binding(0) var<uniform> u_params: CRTUniform;

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
fn fs_main(
    @builtin(position) screen_position: vec4<f32>,
    @location(0) input_uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    var uv = input_uv;
    
    // Curve UVs
    uv = uv * 2.0 - 1.0;
    let offset = uv.yx / u_params.curvature;
    uv = uv + uv * offset * offset;
    uv = uv * 0.5 + 0.5;
    
    // Check bounds
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    
    // RGB split (Chromatic Aberration) slightly based on distance from center
    let dist = length(uv - vec2<f32>(0.5));
    let r_uv = uv + vec2<f32>(0.003 * dist, 0.0);
    let b_uv = uv - vec2<f32>(0.003 * dist, 0.0);
    
    let r = textureSampleLevel(t_diffuse, s_diffuse, r_uv, 0.0).r;
    let g = textureSampleLevel(t_diffuse, s_diffuse, uv, 0.0).g;
    let b = textureSampleLevel(t_diffuse, s_diffuse, b_uv, 0.0).b;
    
    var color = vec3<f32>(r, g, b);
    
    // Scanlines
    let scanline = sin(uv.y * 600.0 * 3.14159) * u_params.scanline_intensity;
    color -= vec3<f32>(scanline);
    
    // Vignette
    let vignette = smoothstep(0.8, 0.2, dist);
    color *= vignette;
    
    // Noise (simple fast hash)
    let pixel = vec2u(floor(screen_position.xy));
    let time_seed = u32(max(u_params.time, 0.0) * 1000.0);
    let noise = hash01(pixel.x ^ hash_u32(pixel.y + time_seed));
    color += vec3<f32>(noise * 0.05);
    
    return vec4<f32>(color, 1.0);
}
