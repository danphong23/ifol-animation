struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0), vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0), vec2<f32>( 1.0, -1.0), vec2<f32>( 1.0,  1.0)
    );
    var out: VertexOutput;
    // Adjust X position by aspect ratio (600/800) so the quad is square on screen,
    // reducing the heavy horizontal stretch of the character.
    out.clip_position = vec4<f32>(pos[vi].x * 0.75, pos[vi].y, 0.0, 1.0);
    out.uv = pos[vi] * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

// Key color is green (0.0, 1.0, 0.0)
const key_color: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);
const threshold: f32 = 0.4; // Tolerance
const smoothing: f32 = 0.1;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    // Calculate distance between current pixel and key color in RGB space
    let diff = distance(color.rgb, key_color);
    
    // Smoothstep for anti-aliasing the edges
    let alpha = smoothstep(threshold - smoothing, threshold + smoothing, diff);
    
    // Multiply alpha by original alpha just in case
    return vec4<f32>(color.rgb, color.a * alpha);
}
