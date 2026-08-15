struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    color: vec4<f32>,
    life: f32,
    max_life: f32,
    size: f32,
    pad: f32,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let p = particles[instance_index];

    let quad_offsets = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );

    let offset = quad_offsets[vertex_index];
    
    // Crisp sub-pixel particle size with aspect ratio correction
    let p_size = vec2<f32>(0.0028, 0.0028 * (800.0 / 600.0));
    let world_pos = p.pos + offset * p_size;

    out.clip_position = vec4<f32>(world_pos.x, world_pos.y, 0.0, 1.0);
    out.color = p.color;
    out.uv = offset;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let r = length(in.uv);
    if (r > 1.0) {
        discard;
    }

    // High quality antialiased star point falloff
    let glow = exp(-r * r * 4.0);
    let alpha = glow * 0.6;

    return vec4<f32>(in.color.rgb, alpha);
}
