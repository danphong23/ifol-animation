@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= 800u || y >= 600u) {
        return;
    }

    let in_dims = textureDimensions(input_tex);
    let in_w = i32(in_dims.x);
    let in_h = i32(in_dims.y);

    // Center divider cyan line (x == 398..402)
    if (x >= 398u && x <= 402u) {
        textureStore(output_tex, vec2<u32>(x, y), vec4<f32>(0.0, 0.95, 1.0, 1.0));
        return;
    }

    // LEFT HALF: Original unprocessed image
    if (x < 400u) {
        let src_x = clamp(i32((f32(x) / 400.0) * f32(in_w)), 0, in_w - 1);
        let src_y = clamp(i32((f32(y) / 600.0) * f32(in_h)), 0, in_h - 1);
        let src_col = textureLoad(input_tex, vec2<i32>(src_x, src_y), 0);
        textureStore(output_tex, vec2<u32>(x, y), src_col);
        return;
    }

    // RIGHT HALF: Compute Shader Processed (Sobel Edge Detection + High Contrast Color Saturation)
    let center_x = clamp(i32((f32(x - 400u) / 400.0) * f32(in_w)), 0, in_w - 1);
    let center_y = clamp(i32((f32(y) / 600.0) * f32(in_h)), 0, in_h - 1);

    var gx = vec3<f32>(0.0);
    var gy = vec3<f32>(0.0);

    let kx = array<f32, 9>(
        -1.0, 0.0, 1.0,
        -2.0, 0.0, 2.0,
        -1.0, 0.0, 1.0
    );
    let ky = array<f32, 9>(
        -1.0, -2.0, -1.0,
         0.0,  0.0,  0.0,
         1.0,  2.0,  1.0
    );

    var idx = 0;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let sample_x = clamp(center_x + dx * 2, 0, in_w - 1);
            let sample_y = clamp(center_y + dy * 2, 0, in_h - 1);
            let sample_c = textureLoad(input_tex, vec2<i32>(sample_x, sample_y), 0).rgb;
            gx = gx + sample_c * kx[idx];
            gy = gy + sample_c * ky[idx];
            idx = idx + 1;
        }
    }

    let edge_mag = sqrt(gx * gx + gy * gy);
    let edge_val = length(edge_mag);

    let orig_color = textureLoad(input_tex, vec2<i32>(center_x, center_y), 0).rgb;
    
    // Inverted Neon Edge Stylization
    let stylized_edge = mix(
        vec3<f32>(0.05, 0.08, 0.15), // Deep dark cyber-blue background
        vec3<f32>(1.0, 0.25, 0.6) * 1.6 + orig_color * 0.9, // Glowing Magenta-Gold edges
        smoothstep(0.12, 0.5, edge_val)
    );

    textureStore(output_tex, vec2<u32>(x, y), vec4<f32>(stylized_edge, 1.0));
}
