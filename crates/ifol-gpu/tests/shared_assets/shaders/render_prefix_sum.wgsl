@group(0) @binding(0) var<storage, read> prefix_data: array<u32, 256>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32) -> VertexOutput {
    // Visualize prefix sum array as a bar chart / spectrum
    let elem_idx = v_idx / 6u;
    let local_v = v_idx % 6u;
    
    let val = f32(prefix_data[elem_idx]);
    let max_val = f32(prefix_data[255]);
    let height = (val / max_val) * 1.5;
    
    let x_step = 2.0 / 256.0;
    let x0 = -1.0 + f32(elem_idx) * x_step;
    let x1 = x0 + x_step * 0.8;
    
    var pos = vec2<f32>(0.0);
    if (local_v == 0u) { pos = vec2<f32>(x0, -0.8); }
    else if (local_v == 1u) { pos = vec2<f32>(x1, -0.8); }
    else if (local_v == 2u) { pos = vec2<f32>(x1, -0.8 + height); }
    else if (local_v == 3u) { pos = vec2<f32>(x0, -0.8); }
    else if (local_v == 4u) { pos = vec2<f32>(x1, -0.8 + height); }
    else if (local_v == 5u) { pos = vec2<f32>(x0, -0.8 + height); }
    
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    
    let hue = f32(elem_idx) / 256.0;
    out.color = vec3<f32>(hue, 1.0 - hue, 0.8);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
