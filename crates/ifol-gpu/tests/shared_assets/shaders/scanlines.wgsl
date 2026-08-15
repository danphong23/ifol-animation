struct ScanlineUniform {
    lines_count: f32, // number of scanlines
    speed: f32,       // speed of movement
    time: f32,        // elapsed time
    opacity: f32,     // opacity of lines
    color: vec4<f32>, // tint of lines
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: ScanlineUniform;

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
    let base_color = textureSample(t_diffuse, s_diffuse, in.uv);
    if (base_color.a < 0.01) {
        return base_color;
    }
    
    // Sine wave based on Y coordinate and time
    let scan_val = sin(in.uv.y * u_params.lines_count + u_params.time * u_params.speed);
    
    // Remap sine wave from [-1, 1] to [0, 1]
    let normalized_scan = (scan_val + 1.0) * 0.5;
    
    // Apply tint
    let scanline_color = u_params.color.rgb * normalized_scan;
    
    // Blend with original color
    let final_rgb = mix(base_color.rgb, base_color.rgb + scanline_color, u_params.opacity);
    
    // Slight transparency modulation for hologram effect
    let hologram_alpha = base_color.a * (1.0 - (1.0 - normalized_scan) * u_params.opacity * 0.5);
    
    return vec4<f32>(final_rgb, hologram_alpha);
}
