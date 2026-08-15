struct Vertex {
    pos: vec2<f32>,
    uv: vec2<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<storage, read> in_vertices: array<Vertex>;
@group(0) @binding(1) var<storage, read_write> out_vertices: array<Vertex>;

struct Uniforms {
    time_and_pad: vec4<f32>,
};
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

@compute @workgroup_size(64, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_vertices = arrayLength(&in_vertices);
    
    if (idx >= total_vertices) {
        return;
    }

    var v = in_vertices[idx];
    
    // Apply a ripple/twist deformation
    let dist = length(v.pos);
    let angle = atan2(v.pos.y, v.pos.x);
    let time = uniforms.time_and_pad.x;
    
    // Twist based on distance and time
    let twist_amount = sin(dist * 5.0 - time * 3.0) * 0.2;
    let new_angle = angle + twist_amount;
    
    v.pos = vec2<f32>(cos(new_angle) * dist, sin(new_angle) * dist);
    
    // Pulse the color based on deformation
    v.color = vec4<f32>(
        0.5 + 0.5 * sin(time + dist * 10.0),
        0.5 + 0.5 * cos(time * 1.2 + dist * 8.0),
        1.0,
        1.0
    );

    out_vertices[idx] = v;
}
