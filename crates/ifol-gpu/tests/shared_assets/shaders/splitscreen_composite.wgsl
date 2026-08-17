struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_left: texture_2d<f32>;
@group(0) @binding(1) var s_left: sampler;

@group(1) @binding(0) var t_right: texture_2d<f32>;
@group(1) @binding(1) var s_right: sampler;

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
    // WebGPU requires textureSample to remain in uniform control flow. Sample
    // both viewports first, then choose the visible half arithmetically.
    let left_uv = vec2<f32>(in.uv.x * 2.0, in.uv.y);
    let right_uv = vec2<f32>((in.uv.x - 0.5) * 2.0, in.uv.y);
    let left_color = textureSample(t_left, s_left, left_uv);
    let right_color = textureSample(t_right, s_right, right_uv);

    // Dividing line at center (uv.x = 0.5)
    let dist_to_divider = abs(in.uv.x - 0.5);
    
    // Glowing neon divider line (width: 2px / 0.003 in UV)
    if (dist_to_divider < 0.0025) {
        let line_glow = 1.0 - dist_to_divider / 0.0025;
        return vec4<f32>(0.3, 0.8, 1.0, 1.0) * line_glow + vec4<f32>(1.0, 1.0, 1.0, 1.0) * pow(line_glow, 3.0);
    }

    // Select Left Viewport for x < 0.5 and Right Viewport otherwise.
    return select(right_color, left_color, in.uv.x < 0.5);
}
