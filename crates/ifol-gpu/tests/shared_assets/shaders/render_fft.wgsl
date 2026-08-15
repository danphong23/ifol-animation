@group(0) @binding(0) var<storage, read> spectrum: array<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) val: f32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    let mag = spectrum[instance_index];
    
    let bar_w = 2.0 / 128.0;
    let x_offset = -1.0 + f32(instance_index) * bar_w;
    
    var pos = vec2<f32>(0.0);
    var uv = vec2<f32>(0.0);
    
    if (vertex_index == 0u) { pos = vec2<f32>(0.0, 0.0); uv = vec2<f32>(0.0, 0.0); }
    if (vertex_index == 1u) { pos = vec2<f32>(1.0, 0.0); uv = vec2<f32>(1.0, 0.0); }
    if (vertex_index == 2u) { pos = vec2<f32>(1.0, mag); uv = vec2<f32>(1.0, 1.0); }
    if (vertex_index == 3u) { pos = vec2<f32>(0.0, 0.0); uv = vec2<f32>(0.0, 0.0); }
    if (vertex_index == 4u) { pos = vec2<f32>(1.0, mag); uv = vec2<f32>(1.0, 1.0); }
    if (vertex_index == 5u) { pos = vec2<f32>(0.0, mag); uv = vec2<f32>(0.0, 1.0); }
    
    pos.x = x_offset + pos.x * bar_w * 0.9;
    pos.y = -1.0 + pos.y * 2.0;
    
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    out.val = mag;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = vec3<f32>(in.uv.y * 1.5, 0.5, 1.0 - in.uv.y);
    return vec4<f32>(color, 1.0);
}
