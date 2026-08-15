struct Particle {
    pos: vec2<f32>,
    radius: f32,
    _pad: f32,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

// We will draw each particle as a quad (6 vertices).
// instance_index corresponds to particle idx.
// vertex_index corresponds to one of the 6 vertices of the quad.
@vertex
fn vs_main(
    @builtin(vertex_index) v_idx: u32,
    @builtin(instance_index) i_idx: u32
) -> VertexOutput {
    let p = particles[i_idx];
    
    // Quad vertices around origin
    var quad_pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );
    
    let local_pos = quad_pos[v_idx];
    let world_pos = p.pos + local_pos * p.radius;
    
    var out: VertexOutput;
    out.clip_pos = vec4<f32>(world_pos, 0.0, 1.0);
    out.uv = local_pos; // -1 to 1 range
    out.color = vec4<f32>(0.2, 0.8, 0.3, 1.0); // Bright green
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Draw as circle
    let dist = length(in.uv);
    if (dist > 1.0) {
        discard;
    }
    
    return in.color;
}
