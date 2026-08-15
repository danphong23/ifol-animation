@group(0) @binding(0) var<storage, read> dst_buffer: array<f32>;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let total_slots = 320.0;
    let valid_count = 301.0;
    let slot_index = floor(in.uv.x * total_slots);

    let val = dst_buffer[u32(slot_index)];

    // Vertical boundary line at index 301 (Red Line)
    let boundary_x = valid_count / total_slots;
    if (abs(in.uv.x - boundary_x) < 0.002) {
        return vec4<f32>(1.0, 0.2, 0.2, 1.0); // Red Boundary Guard Line
    }

    // Normalized Bar height (expected values range around 0.5 to 455.0)
    let bar_height = clamp(val / 460.0, 0.05, 0.95);

    if (in.uv.y < bar_height) {
        if (slot_index < valid_count) {
            // Valid elements: Vibrant Cyan-to-Green Bar Chart
            let t = slot_index / valid_count;
            return vec4<f32>(0.1, 0.8 * t + 0.2, 0.9 - 0.5 * t, 1.0);
        } else {
            // Padding slots: Red Alert if modified (should be 0 height)
            return vec4<f32>(0.9, 0.1, 0.1, 1.0);
        }
    }

    // Background dark grid pattern
    let grid = step(0.98, fract(in.uv.x * 32.0)) * 0.05;
    return vec4<f32>(0.03 + grid, 0.04 + grid, 0.07 + grid, 1.0);
}
