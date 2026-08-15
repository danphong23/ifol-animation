// Compute Vertex Wave Simulation (TC102)
// Calculates dynamic vertex positions & normal colors into a Storage Buffer

struct Vertex {
    pos: vec4f,
    color: vec4f,
};

struct SimulationParams {
    time: f32,
    grid_size: u32,
    wave_frequency: f32,
    wave_amplitude: f32,
};

@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(1) var<uniform> params: SimulationParams;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) gid: vec3u) {
    let index = gid.x;
    let total_vertices = params.grid_size * params.grid_size;
    if index >= total_vertices {
        return;
    }
    
    let gx = index % params.grid_size;
    let gz = index / params.grid_size;
    
    let u = f32(gx) / f32(params.grid_size - 1u) * 2.0 - 1.0;
    let v = f32(gz) / f32(params.grid_size - 1u) * 2.0 - 1.0;
    
    let dist = sqrt(u * u + v * v);
    let y = sin(dist * params.wave_frequency - params.time * 3.0) * params.wave_amplitude * (1.0 - dist * 0.5);
    
    // Wave normal shading color
    let r = (y / params.wave_amplitude) * 0.5 + 0.5;
    let g = sin(u * 3.0 + params.time) * 0.3 + 0.5;
    let b = cos(v * 3.0 + params.time) * 0.5 + 0.5;
    
    vertices[index].pos = vec4f(u * 0.8, y * 0.5, v * 0.8, 1.0);
    vertices[index].color = vec4f(r, g, b, 1.0);
}
