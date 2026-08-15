struct Particle {
    pos: vec2<f32>,
    depth: f32,
    _pad: f32,
    color: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32,
) -> VertexOutput {
    let p = particles[in_instance_index];
    
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    
    let base_pos = pos[in_vertex_index];
    let size = 0.2;
    let offset = base_pos * size;
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(p.pos + offset, 0.5, 1.0);
    out.color = p.color;
    out.uv = base_pos * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = distance(in.uv, vec2<f32>(0.5, 0.5));
    if dist > 0.5 {
        discard;
    }
    
    let alpha = smoothstep(0.5, 0.3, dist);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
