// Glowing Orb & Pulse Shader for TC105

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = array<vec2f, 4>(
        vec2f(-1.0,  1.0),
        vec2f(-1.0, -1.0),
        vec2f( 1.0,  1.0),
        vec2f( 1.0, -1.0)
    );
    out.position = vec4f(pos[in_vertex_index], 0.0, 1.0);
    out.uv = pos[in_vertex_index] * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let uv = in.uv;
    
    // Glowing central core orb + satellite rings
    let center = vec2f(0.5, 0.5);
    let dist = distance(uv, center);
    
    let core = exp(-dist * 12.0) * 1.5;
    let ring = exp(-abs(dist - 0.25) * 40.0) * 0.8;
    let sat_dist = distance(uv, vec2f(0.65, 0.35));
    let satellite = exp(-sat_dist * 25.0) * 1.2;
    
    let intensity = core + ring + satellite;
    let col = vec3f(intensity * 1.0, intensity * 0.4, intensity * 0.9);
    
    return vec4f(col, clamp(intensity, 0.0, 1.0));
}
