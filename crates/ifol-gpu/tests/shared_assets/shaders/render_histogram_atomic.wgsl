struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );

    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = uv[vertex_index];
    return out;
}

@group(0) @binding(0) var<storage, read> histogram: array<u32, 256>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let bin_idx = u32(in.uv.x * 256.0);
    let count = f32(histogram[bin_idx]);
    let norm_height = count / 1000.0; // Normalized height

    let y = 1.0 - in.uv.y;
    var col = vec3<f32>(0.02, 0.03, 0.05);

    if (y < norm_height * 0.8 + 0.05) {
        // Color gradient based on bin index
        let hue = f32(bin_idx) / 256.0;
        let r = sin(hue * 6.28) * 0.5 + 0.5;
        let g = sin(hue * 6.28 + 2.09) * 0.5 + 0.5;
        let b = sin(hue * 6.28 + 4.18) * 0.5 + 0.5;
        col = vec3<f32>(r, g, b);
    }

    return vec4<f32>(col, 1.0);
}
