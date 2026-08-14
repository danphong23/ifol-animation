struct TransitionUniform {
    progress: f32,
    effect_type: f32, // 0: Liquid Warp, 1: Glitch
    direction_x: f32,
    direction_y: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_input_A: texture_2d<f32>;
@group(0) @binding(1) var s_input_A: sampler;
@group(0) @binding(2) var t_input_B: texture_2d<f32>;
@group(0) @binding(3) var s_input_B: sampler;

@group(1) @binding(0) var<uniform> fx: TransitionUniform;

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

// Pseudo-random function
fn hash(n: vec2<f32>) -> f32 {
    return fract(sin(dot(n, vec2<f32>(12.9898, 4.1414))) * 43758.5453);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color_A = vec4<f32>(0.0);
    var color_B = vec4<f32>(0.0);
    let p = fx.progress;

    if (fx.effect_type < 0.5) {
        // ---- LIQUID WARP ----
        let x_offset = sin(in.uv.y * 20.0 + p * 10.0) * 0.05 * sin(p * 3.1415);
        let y_offset = cos(in.uv.x * 20.0 + p * 10.0) * 0.05 * sin(p * 3.1415);
        
        let uv_A = in.uv + vec2<f32>(x_offset, y_offset) * p;
        let uv_B = in.uv + vec2<f32>(x_offset, y_offset) * (1.0 - p);

        color_A = textureSample(t_input_A, s_input_A, clamp(uv_A, vec2<f32>(0.0), vec2<f32>(1.0)));
        color_B = textureSample(t_input_B, s_input_B, clamp(uv_B, vec2<f32>(0.0), vec2<f32>(1.0)));
        
        // Soft wipe based on direction
        let dir = vec2<f32>(fx.direction_x, fx.direction_y);
        let proj = dot(in.uv - vec2<f32>(0.5), dir) + 0.5;
        let mix_factor = smoothstep(p - 0.2, p + 0.2, proj);

        return mix(color_A, color_B, mix_factor);

    } else {
        // ---- GLITCH ----
        // Create blocky distortion
        let block_y = floor(in.uv.y * 30.0) / 30.0;
        let shift = (hash(vec2<f32>(block_y, p)) - 0.5) * 0.3 * sin(p * 3.1415);
        
        let rgb_split = 0.02 * sin(p * 3.1415);

        // Sample with RGB split for A and B
        let r_A = textureSample(t_input_A, s_input_A, in.uv + vec2<f32>(shift + rgb_split, 0.0)).r;
        let g_A = textureSample(t_input_A, s_input_A, in.uv + vec2<f32>(shift, 0.0)).g;
        let b_A = textureSample(t_input_A, s_input_A, in.uv + vec2<f32>(shift - rgb_split, 0.0)).b;
        color_A = vec4<f32>(r_A, g_A, b_A, 1.0);

        let r_B = textureSample(t_input_B, s_input_B, in.uv + vec2<f32>(shift + rgb_split, 0.0)).r;
        let g_B = textureSample(t_input_B, s_input_B, in.uv + vec2<f32>(shift, 0.0)).g;
        let b_B = textureSample(t_input_B, s_input_B, in.uv + vec2<f32>(shift - rgb_split, 0.0)).b;
        color_B = vec4<f32>(r_B, g_B, b_B, 1.0);

        // Hard cuts based on random blocks
        let threshold = hash(vec2<f32>(floor(in.uv.x * 20.0), block_y));
        let mix_factor = step(1.0 - p, threshold);

        return mix(color_A, color_B, mix_factor);
    }
}
