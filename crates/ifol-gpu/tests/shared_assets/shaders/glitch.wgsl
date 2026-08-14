struct GlitchUniform {
    transform: mat4x4<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    time: f32,
    intensity: f32, // Glitch intensity
    aberration: f32, // Chromatic aberration distance
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> config: GlitchUniform;

fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453);
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    
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
    
    let p = pos[vi];
    out.clip_position = config.transform * vec4<f32>(p, 0.0, 1.0);
    out.uv = mix(config.uv_min, config.uv_max, uv[vi]);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Generate blocky glitch offsets
    let block_y = floor(in.uv.y * 20.0);
    let glitch_noise = hash(block_y + config.time) * 2.0 - 1.0;
    
    var uv_offset = 0.0;
    if (abs(glitch_noise) > (1.0 - config.intensity * 0.5)) {
        uv_offset = glitch_noise * 0.05 * config.intensity;
    }
    
    // Base UV with glitch offset
    let base_uv = vec2<f32>(in.uv.x + uv_offset, in.uv.y);
    
    // Chromatic Aberration sampling
    let r_uv = base_uv + vec2<f32>(config.aberration, 0.0);
    let g_uv = base_uv;
    let b_uv = base_uv - vec2<f32>(config.aberration, 0.0);
    
    let color_r = textureSampleLevel(t_diffuse, s_diffuse, r_uv, 0.0);
    let color_g = textureSampleLevel(t_diffuse, s_diffuse, g_uv, 0.0);
    let color_b = textureSampleLevel(t_diffuse, s_diffuse, b_uv, 0.0);
    
    var out_rgba = vec4<f32>(color_r.r, color_g.g, color_b.b, color_g.a);
    
    // Despill chroma key on the green screen.
    // Notice since we split channels, green despill should primarily look at the center UV (color_g)
    let max_rb = max(color_g.r, color_g.b);
    if (color_g.g > max_rb * 1.1) {
        out_rgba.g = max_rb;
        out_rgba.a = 0.0; // Hard mask out green bg
    }
    
    // If the base pixel is transparent, discard
    if (out_rgba.a < 0.1) {
        discard;
    }
    
    return out_rgba;
}
