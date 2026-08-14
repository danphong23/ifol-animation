struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn hash(n: u32) -> f32 {
    var x = n * 1103515245u + 12345u;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return f32(x) / 4294967295.0;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VertexOutput {
    var quad_pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.006,  0.008),
        vec2<f32>(-0.006, -0.008),
        vec2<f32>( 0.006,  0.008),
        vec2<f32>( 0.006,  0.008),
        vec2<f32>(-0.006, -0.008),
        vec2<f32>( 0.006, -0.008)
    );

    let seed = ii * 3u;
    let rand_x = hash(seed) * 1.96 - 0.98;
    let rand_y = hash(seed + 1u) * 1.96 - 0.98;
    let rand_scale = 0.5 + hash(seed + 2u) * 1.2;

    var out: VertexOutput;
    let p = vec2<f32>(rand_x, rand_y) + quad_pos[vi] * rand_scale;
    out.clip_position = vec4<f32>(p, 0.4, 1.0);
    
    // Glowing stardust colors (golden yellow, cyan, soft white)
    let color_type = ii % 3u;
    if (color_type == 0u) {
        out.color = vec4<f32>(1.0, 0.9, 0.6, 0.85); // Golden
    } else if (color_type == 1u) {
        out.color = vec4<f32>(0.6, 0.9, 1.0, 0.85); // Cyan
    } else {
        out.color = vec4<f32>(1.0, 1.0, 1.0, 0.95); // White spark
    }
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
