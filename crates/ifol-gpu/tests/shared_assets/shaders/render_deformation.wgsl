struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Input pos is [-1, 1], we can just pass it directly or add a perspective transform if needed
    out.clip_pos = vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // We can use UV to draw a wireframe or just color it
    let uv = in.uv * 64.0;
    let grid = abs(fract(uv - 0.5) - 0.5);
    let line = min(grid.x, grid.y);
    
    // Draw grid lines
    if (line < 0.05) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
    
    return in.color;
}
