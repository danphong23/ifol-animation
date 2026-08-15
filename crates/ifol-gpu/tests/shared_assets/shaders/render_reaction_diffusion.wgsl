@group(0) @binding(0) var rd_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((in_vertex_index << 1u) & 2u);
    let y = f32(in_vertex_index & 2u);
    out.clip_pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let rd_val = textureSampleLevel(rd_tex, samp, in.uv, 0.0);
    // a is rd_val.r, b is rd_val.g
    // We can map the difference to a nice color map.
    // Reaction diffusion usually looks good if we visualize (A - B) or just B.
    let b = rd_val.g;
    
    // Color mapping: dark blue to cyan to pink
    let col1 = vec3<f32>(0.05, 0.05, 0.2); // Background (low B)
    let col2 = vec3<f32>(0.0, 0.8, 0.8);   // Mid (medium B)
    let col3 = vec3<f32>(1.0, 0.2, 0.5);   // High (high B)
    
    var final_col = vec3<f32>(0.0);
    let t = clamp(b * 3.0, 0.0, 1.0); // Boost contrast of B
    if (t < 0.5) {
        final_col = mix(col1, col2, t * 2.0);
    } else {
        final_col = mix(col2, col3, (t - 0.5) * 2.0);
    }

    return vec4<f32>(final_col, 1.0);
}
