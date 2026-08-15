struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32,
) -> VertexOutput {
    let p = particles[in_instance_index];
    
    // A simple quad from 6 vertices
    var pos = vec2<f32>(0.0, 0.0);
    var uv = vec2<f32>(0.0, 0.0);
    
    switch in_vertex_index {
        case 0u: { pos = vec2<f32>(-1.0, -1.0); uv = vec2<f32>(-1.0, -1.0); }
        case 1u: { pos = vec2<f32>( 1.0, -1.0); uv = vec2<f32>( 1.0, -1.0); }
        case 2u: { pos = vec2<f32>(-1.0,  1.0); uv = vec2<f32>(-1.0,  1.0); }
        case 3u: { pos = vec2<f32>(-1.0,  1.0); uv = vec2<f32>(-1.0,  1.0); }
        case 4u: { pos = vec2<f32>( 1.0, -1.0); uv = vec2<f32>( 1.0, -1.0); }
        case 5u: { pos = vec2<f32>( 1.0,  1.0); uv = vec2<f32>( 1.0,  1.0); }
        default: {}
    }
    
    let radius = 5.0; // matching compute shader radius
    // Convert pos to screen space. Assuming 800x800 logical area (32 grid size * 25 cell size)
    let screen_pos = p.pos + pos * radius;
    
    // Map from 0..800 to -1..1
    let clip_pos = vec2<f32>(
        (screen_pos.x / 800.0) * 2.0 - 1.0,
        1.0 - (screen_pos.y / 800.0) * 2.0
    );
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(clip_pos, 0.5, 1.0);
    out.uv = uv;
    out.color = p.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv);
    if (dist > 1.0) {
        discard;
    }
    
    // Antialiased circle
    let alpha = 1.0 - smoothstep(0.8, 1.0, dist);
    
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
