// Source Pattern Generator for Texture Copy Testing (TC101)
// Renders high-frequency multi-color geometric pattern to test pixel-perfect DMA blit

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
    
    // Concentric rings & diagonal color gradients
    let center = vec2f(0.5);
    let dist = distance(uv, center);
    let rings = sin(dist * 50.0) * 0.5 + 0.5;
    
    let r = uv.x * 0.8 + rings * 0.2;
    let g = uv.y * 0.8 + (1.0 - rings) * 0.2;
    let b = sin(uv.x * 10.0 + uv.y * 10.0) * 0.5 + 0.5;
    
    // Grid markers every 10%
    let grid = step(0.95, fract(uv.x * 10.0)) + step(0.95, fract(uv.y * 10.0));
    let color = mix(vec3f(r, g, b), vec3f(1.0, 1.0, 1.0), min(grid, 1.0) * 0.5);
    
    return vec4f(color, 1.0);
}
