// Fallback Error Checkerboard Shader (TC100)
// Renders high-contrast Magenta/Black debug grid when a node fails validation

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
    
    // 16x16 debug checkerboard
    let check_x = floor(uv.x * 16.0);
    let check_y = floor(uv.y * 16.0);
    let pattern = (i32(check_x) + i32(check_y)) % 2;
    
    // Magenta (#FF00FF) and Dark Gray (#181818)
    let magenta = vec3f(1.0, 0.0, 0.85);
    let dark = vec3f(0.08, 0.08, 0.1);
    
    let color = select(dark, magenta, pattern == 0);
    
    // Warning hazard stripes around border
    let border_dist = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    if border_dist < 0.05 {
        let stripe = floor((uv.x + uv.y) * 40.0) % 2.0;
        let border_col = select(vec3f(1.0, 0.8, 0.0), vec3f(0.0), stripe == 0.0);
        return vec4f(border_col, 1.0);
    }
    
    return vec4f(color, 1.0);
}
