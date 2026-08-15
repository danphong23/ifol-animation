struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = vec2<f32>(0.0);
    var uv = vec2<f32>(0.0);
    if (vertex_index == 0u) { pos = vec2<f32>(-1.0, -1.0); uv = vec2<f32>(0.0, 1.0); }
    else if (vertex_index == 1u) { pos = vec2<f32>( 3.0, -1.0); uv = vec2<f32>(2.0, 1.0); }
    else if (vertex_index == 2u) { pos = vec2<f32>(-1.0,  3.0); uv = vec2<f32>(0.0, -1.0); }
    
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, smp, in.uv);
}
