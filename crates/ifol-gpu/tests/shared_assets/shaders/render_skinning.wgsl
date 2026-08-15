struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Vertex {
    pos: vec4<f32>,
    uv: vec2<f32>,
    pad: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> vertices: array<Vertex>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let v = vertices[vertex_index];
    var out: VertexOutput;
    
    // Scale down a bit and apply simple perspective
    var p = v.pos;
    p.x -= 0.5;
    p.y -= 0.5;
    
    let z_factor = 1.0 + p.z * 0.5;
    
    out.position = vec4<f32>(p.x * 1.5 / z_factor, p.y * 1.5 / z_factor, p.z, 1.0);
    out.uv = v.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Checkerboard pattern
    let size = 10.0;
    let grid = floor(in.uv * size);
    let c = (grid.x + grid.y) % 2.0;
    
    let color = mix(vec3<f32>(0.2, 0.4, 0.8), vec3<f32>(0.9, 0.9, 0.9), c);
    return vec4<f32>(color, 1.0);
}
