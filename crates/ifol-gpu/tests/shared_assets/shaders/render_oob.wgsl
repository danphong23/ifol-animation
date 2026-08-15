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

@group(0) @binding(0) var<storage, read> dst_buffer: array<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x = in.uv.x;
    let y = 1.0 - in.uv.y;

    // Total elements = 1024, Valid elements = 1000, Padding = 24
    let idx = u32(x * 1024.0);
    let val = dst_buffer[idx];

    // Background
    var col = vec3<f32>(0.03, 0.04, 0.07);

    // Boundary line at 1000/1024 = 0.9765
    let boundary = 1000.0 / 1024.0;
    if (abs(x - boundary) < 0.003) {
        return vec4<f32>(1.0, 0.2, 0.2, 1.0); // Red vertical line marking valid limit
    }

    if (x < boundary) {
        // Valid region (Green bars)
        let normalized_val = clamp((val - 1.0) / 10.0, 0.0, 1.0);
        if (y < normalized_val * 0.7 + 0.1) {
            col = vec3<f32>(0.1, 0.85, 0.4);
        }
    } else {
        // Out of bounds padding region (Untouched zeroes -> Purple indicators)
        if (abs(val) < 0.001) {
            if (y < 0.15) {
                col = vec3<f32>(0.6, 0.2, 0.9); // Untouched zero buffer
            }
        } else {
            col = vec3<f32>(1.0, 0.0, 0.0); // Error corrupt memory
        }
    }

    return vec4<f32>(col, 1.0);
}
