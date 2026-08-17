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

// Integer hash keeps block selection deterministic across shader backends.
fn hash_u32(value: u32) -> u32 {
    var h = value;
    h = h ^ (h >> 16u);
    h = h * 2146121005u;
    h = h ^ (h >> 15u);
    h = h * 2221713035u;
    h = h ^ (h >> 16u);
    return h;
}

fn hash_block(x: u32, y: u32) -> f32 {
    let mixed = hash_u32(x ^ hash_u32(y * 374761393u));
    return f32(hash_u32(mixed)) / 4294967295.0;
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

        color_A = textureSampleLevel(t_input_A, s_input_A, clamp(uv_A, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
        color_B = textureSampleLevel(t_input_B, s_input_B, clamp(uv_B, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
        
        // Soft wipe based on direction
        let dir = vec2<f32>(fx.direction_x, fx.direction_y);
        let proj = dot(in.uv - vec2<f32>(0.5), dir) + 0.5;
        let mix_factor = smoothstep(p - 0.2, p + 0.2, proj);

        return mix(color_A, color_B, mix_factor);

    } else {
        // ---- GLITCH ----
        // Create blocky distortion
        let block_y_index = u32(floor(in.uv.y * 30.0));
        let shift = (hash_block(0u, block_y_index) - 0.5) * 0.3 * sin(p * 3.1415);
        
        let rgb_split = 0.02 * sin(p * 3.1415);

        // Sample with RGB split for A and B
        let r_A = textureSampleLevel(t_input_A, s_input_A, in.uv + vec2<f32>(shift + rgb_split, 0.0), 0.0).r;
        let g_A = textureSampleLevel(t_input_A, s_input_A, in.uv + vec2<f32>(shift, 0.0), 0.0).g;
        let b_A = textureSampleLevel(t_input_A, s_input_A, in.uv + vec2<f32>(shift - rgb_split, 0.0), 0.0).b;
        color_A = vec4<f32>(r_A, g_A, b_A, 1.0);

        let r_B = textureSampleLevel(t_input_B, s_input_B, in.uv + vec2<f32>(shift + rgb_split, 0.0), 0.0).r;
        let g_B = textureSampleLevel(t_input_B, s_input_B, in.uv + vec2<f32>(shift, 0.0), 0.0).g;
        let b_B = textureSampleLevel(t_input_B, s_input_B, in.uv + vec2<f32>(shift - rgb_split, 0.0), 0.0).b;
        color_B = vec4<f32>(r_B, g_B, b_B, 1.0);

        // Hard cuts based on random blocks
        let threshold = hash_block(u32(floor(in.uv.x * 20.0)), block_y_index);
        let mix_factor = step(1.0 - p, threshold);

        return mix(color_A, color_B, mix_factor);
    }
}
