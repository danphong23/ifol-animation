struct DistortionUniform {
    transform: mat4x4<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    time: f32,
    amplitude: f32,
    frequency: f32,
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> config: DistortionUniform;

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
    
    var p = pos[vi];
    
    // Sway effect (Wind): Only move the top vertices (where p.y > 0)
    // The higher the Y, the more it sways
    let sway_factor = smoothstep(-1.0, 1.0, p.y);
    p.x += sin(config.time * config.frequency) * config.amplitude * sway_factor;
    
    out.clip_position = config.transform * vec4<f32>(p, 0.0, 1.0);
    out.uv = mix(config.uv_min, config.uv_max, uv[vi]);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex_color = textureSampleLevel(t_diffuse, s_diffuse, in.uv, 0.0);
    
    let max_rb = max(tex_color.r, tex_color.b);
    if (tex_color.g > max_rb * 1.1) {
        tex_color.g = max_rb;
        tex_color.a = 0.0;
    }
    
    if (tex_color.a < 0.1) {
        discard;
    }
    
    return tex_color;
}
