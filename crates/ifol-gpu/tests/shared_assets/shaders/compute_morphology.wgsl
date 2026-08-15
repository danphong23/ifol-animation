struct Params {
    radius: i32,
    mode: i32, // 0 = dilation, 1 = erosion
    _pad: vec2<i32>,
}
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(input_tex);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }
    
    let center_pos = vec2<i32>(global_id.xy);
    var result_color = vec4<f32>(0.0);
    
    if (params.mode == 0) { // Dilation (Max)
        var max_val = vec4<f32>(0.0);
        for (var dy = -params.radius; dy <= params.radius; dy++) {
            for (var dx = -params.radius; dx <= params.radius; dx++) {
                let p = center_pos + vec2<i32>(dx, dy);
                if (p.x >= 0 && p.x < i32(size.x) && p.y >= 0 && p.y < i32(size.y)) {
                    let val = textureLoad(input_tex, p, 0);
                    max_val = max(max_val, val);
                }
            }
        }
        result_color = max_val;
    } else { // Erosion (Min)
        var min_val = vec4<f32>(1.0);
        for (var dy = -params.radius; dy <= params.radius; dy++) {
            for (var dx = -params.radius; dx <= params.radius; dx++) {
                let p = center_pos + vec2<i32>(dx, dy);
                if (p.x >= 0 && p.x < i32(size.x) && p.y >= 0 && p.y < i32(size.y)) {
                    let val = textureLoad(input_tex, p, 0);
                    min_val = min(min_val, val);
                }
            }
        }
        result_color = min_val;
    }
    
    // Draw original as red in left half for comparison?
    // Let's just output it! We'll do a split screen in the test file using a render pass!
    textureStore(output_tex, center_pos, result_color);
}

@compute @workgroup_size(16, 16)
fn cs_gen_mask(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(output_tex);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }
    let p = vec2<f32>(global_id.xy) / vec2<f32>(size);
    let dist = distance(p, vec2<f32>(0.5, 0.5));
    
    var col = 0.0;
    // Ring
    if (dist > 0.2 && dist < 0.22) { col = 1.0; }
    // Small dots
    if (distance(p, vec2<f32>(0.2, 0.2)) < 0.02) { col = 1.0; }
    if (distance(p, vec2<f32>(0.8, 0.2)) < 0.02) { col = 1.0; }
    if (distance(p, vec2<f32>(0.5, 0.8)) < 0.02) { col = 1.0; }
    
    textureStore(output_tex, global_id.xy, vec4<f32>(col, col, col, 1.0));
}
