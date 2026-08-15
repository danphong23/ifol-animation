struct Particle {
    pos: vec4<f32>,
    old_pos: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32) -> VertexOutput {
    let size = 16u;
    let quads_per_row = size - 1u;
    
    let quad_idx = v_idx / 6u;
    let quad_v = v_idx % 6u;
    
    let quad_x = quad_idx % quads_per_row;
    let quad_y = quad_idx / quads_per_row;
    
    var x = quad_x;
    var y = quad_y;
    
    // Quad vertices: 0=(0,0), 1=(1,0), 2=(1,1)   3=(0,0), 4=(1,1), 5=(0,1)
    if (quad_v == 1u || quad_v == 2u || quad_v == 4u) { x += 1u; }
    if (quad_v == 2u || quad_v == 4u || quad_v == 5u) { y += 1u; }
    
    let particle_idx = y * size + x;
    let p = particles[particle_idx].pos;
    
    var out: VertexOutput;
    
    // Simple view-projection
    var pos = p;
    pos.x -= 0.5;
    pos.y -= 0.5;
    let z_factor = 1.0 + pos.z * 0.5;
    
    out.position = vec4<f32>(pos.x * 1.5 / z_factor, pos.y * 1.5 / z_factor, pos.z, 1.0);
    out.uv = vec2<f32>(f32(x) / f32(quads_per_row), f32(y) / f32(quads_per_row));
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Red flag color with some shading
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    
    // Checkerboard
    let grid = floor(in.uv * 10.0);
    let c = (grid.x + grid.y) % 2.0;
    let base_color = mix(vec3<f32>(0.8, 0.1, 0.1), vec3<f32>(0.9, 0.2, 0.2), c);
    
    return vec4<f32>(base_color, 1.0);
}
