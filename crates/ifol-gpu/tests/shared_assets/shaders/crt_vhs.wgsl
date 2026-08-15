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
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;
    
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
    
    let r = textureSample(t_diffuse, s_diffuse, r_uv).r;
    let g = textureSample(t_diffuse, s_diffuse, uv).g;
    let b = textureSample(t_diffuse, s_diffuse, b_uv).b;
    
    var color = vec3<f32>(r, g, b);
    
    // Scanlines
    let scanline = sin(uv.y * 600.0 * 3.14159) * u_params.scanline_intensity;
    color -= vec3<f32>(scanline);
    
    // Vignette
    let vignette = smoothstep(0.8, 0.2, dist);
    color *= vignette;
    
    // Noise (simple fast hash)
    let noise = fract(sin(dot(uv + vec2<f32>(u_params.time), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    color += vec3<f32>(noise * 0.05);
    
    return vec4<f32>(color, 1.0);
}
