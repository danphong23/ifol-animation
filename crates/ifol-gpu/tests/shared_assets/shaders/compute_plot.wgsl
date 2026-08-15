struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var quad_pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var norm_uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(quad_pos[vi], 0.0, 1.0);
    out.uv = norm_uv[vi];
    return out;
}

struct DataArray {
    elements: array<vec4<f32>>,
};

@group(0) @binding(0) var<storage, read> input_a: DataArray;
@group(0) @binding(1) var<storage, read> input_b: DataArray;
@group(0) @binding(2) var<storage, read> output_c: DataArray;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Blueprint Grid Background with coordinates
    let grid_x = abs(fract(in.uv.x * 16.0 - 0.5) - 0.5) / fwidth(in.uv.x * 16.0);
    let grid_y = abs(fract(in.uv.y * 12.0 - 0.5) - 0.5) / fwidth(in.uv.y * 12.0);
    let grid_line = min(grid_x, grid_y);
    var col = mix(vec3<f32>(0.07, 0.10, 0.16), vec3<f32>(0.02, 0.03, 0.06), smoothstep(0.0, 1.0, grid_line));

    // Zero-Axis line (Center Y = 0.5)
    let axis_dist = abs(in.uv.y - 0.5) * 600.0;
    col += vec3<f32>(0.15, 0.22, 0.35) * smoothstep(1.5, 0.0, axis_dist);

    // Header label banner background (Top 10% of screen)
    if (in.uv.y < 0.08) {
        col = mix(col, vec3<f32>(0.05, 0.08, 0.14), 0.85);
        let border_dist = abs(in.uv.y - 0.08) * 600.0;
        col += vec3<f32>(0.2, 0.4, 0.7) * smoothstep(1.5, 0.0, border_dist);
    }

    let total_elements = arrayLength(&output_c.elements);
    let sample_idx = u32(clamp(in.uv.x * f32(total_elements), 0.0, f32(total_elements - 1u)));

    // 2. INPUT A: Raw Linear Ramp (Yellow-Amber Dotted/Dashed Line)
    let val_a = input_a.elements[sample_idx].x;
    let y_a = 0.5 - (val_a / 12.0);
    let dist_a = abs(in.uv.y - y_a) * 600.0;
    let dash_a = step(0.4, fract(in.uv.x * 50.0)); // Dashed pattern for input
    let col_input_a = vec3<f32>(1.0, 0.75, 0.2) * smoothstep(2.0, 0.0, dist_a) * dash_a;

    // 3. INPUT B: High-frequency Sine Wave (Orange-Red Thin Line)
    let val_b = sin(input_b.elements[sample_idx].x) * 1.5;
    let y_b = 0.5 - (val_b / 12.0);
    let dist_b = abs(in.uv.y - y_b) * 600.0;
    let col_input_b = vec3<f32>(0.9, 0.35, 0.15) * smoothstep(1.5, 0.0, dist_b) * 0.7;

    // 4. OUTPUT C: Computed Composite Waveform (Bright Neon Cyan Glow Line)
    let val_c = output_c.elements[sample_idx].x;
    let y_c = 0.5 - (val_c / 12.0);
    let dist_c = abs(in.uv.y - y_c) * 600.0;
    let glow_c = exp(-dist_c * 0.25) * vec3<f32>(0.0, 0.9, 1.0) * 0.9;
    let core_c = smoothstep(2.0, 0.0, dist_c) * vec3<f32>(0.9, 1.0, 1.0);

    // 5. Legend Indicator Boxes in Header
    // Box 1: [--- Input A: Linear Ramp] (x: 0.05..0.28)
    if (in.uv.y > 0.02 && in.uv.y < 0.06 && in.uv.x > 0.04 && in.uv.x < 0.28) {
        col = mix(col, vec3<f32>(1.0, 0.75, 0.2), 0.25);
    }
    // Box 2: [--- Input B: Sine Oscillation] (x: 0.32..0.58)
    if (in.uv.y > 0.02 && in.uv.y < 0.06 && in.uv.x > 0.32 && in.uv.x < 0.58) {
        col = mix(col, vec3<f32>(0.9, 0.35, 0.15), 0.25);
    }
    // Box 3: [=== Output C: GPU Computed Wave] (x: 0.62..0.94)
    if (in.uv.y > 0.02 && in.uv.y < 0.06 && in.uv.x > 0.62 && in.uv.x < 0.94) {
        col = mix(col, vec3<f32>(0.0, 0.9, 1.0), 0.35);
    }

    col += col_input_a + col_input_b + glow_c + core_c;
    return vec4<f32>(col, 1.0);
}
