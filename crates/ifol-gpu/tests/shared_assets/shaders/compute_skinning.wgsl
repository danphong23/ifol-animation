struct Vertex {
    pos: vec4<f32>,
    uv: vec2<f32>,
    pad: vec2<f32>,
}

struct Params {
    time: f32,
    count: u32,
    pad1: u32,
    pad2: u32,
}

@group(0) @binding(0) var<storage, read> in_vertices: array<Vertex>;
@group(0) @binding(1) var<storage, read_write> out_vertices: array<Vertex>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.count) {
        return;
    }
    
    var v = in_vertices[index];
    
    // Waving animation (simulated wind/skinning)
    let wave = sin(v.pos.x * 5.0 - params.time * 5.0) * v.uv.x * 0.5;
    v.pos.y += wave;
    v.pos.z += cos(v.pos.x * 3.0 - params.time * 4.0) * v.uv.x * 0.2;
    
    out_vertices[index] = v;
}
