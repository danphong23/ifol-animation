@group(0) @binding(0) var<storage, read> input_buf: array<f32>;

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
    let total_elements = 1000000.0;
    let target_idx = 543210.0;
    let target_x = target_idx / total_elements; // Normalized X coordinate of MAX element (~0.5432)

    let uv = in.uv;

    // 1. Vertical Target Radar Scan Beam (Gold Line) at Target Index
    let beam_dist = abs(uv.x - target_x);
    var beam_color = vec3<f32>(0.0);
    if (beam_dist < 0.0015) {
        beam_color = vec3<f32>(1.0, 0.85, 0.2); // Golden Target Laser Beam
    } else if (beam_dist < 0.008) {
        beam_color = vec3<f32>(1.0, 0.85, 0.2) * (1.0 - beam_dist / 0.008) * 0.3;
    }

    // 2. Sample local data sample from input buffer
    let sampled_idx = u32(clamp(uv.x * total_elements, 0.0, total_elements - 1.0));
    let val = input_buf[sampled_idx];

    // 3. Render 1M Point Cloud Base Layer (Greenish Noise at Bottom)
    var cloud_color = vec3<f32>(0.0);
    let normalized_val = val / 9999.5; // [0.0 .. 1.0]

    if (val > 1000.0) {
        // TARGET MAX ELEMENT (Star Glow at Top)
        let star_pos = vec2<f32>(target_x, 0.85);
        let dist = length(uv - star_pos);
        if (dist < 0.04) {
            let glow = (1.0 - dist / 0.04);
            cloud_color = vec3<f32>(1.0, 0.9, 0.3) * glow * 2.0; // Glowing Yellow Star
        }
    } else {
        // Normal 999,999 Background Elements (Greenish Wave Base)
        let sample_height = clamp(val / 100.0 * 0.3, 0.02, 0.3);
        if (uv.y < sample_height) {
            cloud_color = vec3<f32>(0.1, 0.5 + uv.y, 0.2);
        }
    }

    let final_color = cloud_color + beam_color + vec3<f32>(0.02, 0.03, 0.05);
    return vec4<f32>(final_color, 1.0);
}
