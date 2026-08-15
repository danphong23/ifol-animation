struct Particle {
    pos: vec2<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var quad_offsets = array<vec2<f32>, 6>(
        vec2<f32>(-0.015, -0.015),
        vec2<f32>( 0.015, -0.015),
        vec2<f32>(-0.015,  0.015),
        vec2<f32>(-0.015,  0.015),
        vec2<f32>( 0.015, -0.015),
        vec2<f32>( 0.015,  0.015)
    );

    let p = particles[instance_index];
    let offset = quad_offsets[vertex_index];
    let final_pos = p.pos + offset;

    var out: VertexOutput;
    out.position = vec4<f32>(final_pos, 0.0, 1.0);
    out.color = p.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
