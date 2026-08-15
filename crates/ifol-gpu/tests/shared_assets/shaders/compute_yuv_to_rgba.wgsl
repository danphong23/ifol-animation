@group(0) @binding(0) var y_tex: texture_2d<f32>;
@group(0) @binding(1) var u_tex: texture_2d<f32>;
@group(0) @binding(2) var v_tex: texture_2d<f32>;
@group(0) @binding(3) var output_tex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(output_tex);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }
    
    let pos = vec2<i32>(global_id.xy);
    let uv_pos = pos / 2; // integer division for 4:2:0 subsampling
    
    let y = textureLoad(y_tex, pos, 0).r;
    let u = textureLoad(u_tex, uv_pos, 0).r - 0.5;
    let v = textureLoad(v_tex, uv_pos, 0).r - 0.5;
    
    // BT.601 conversion
    let r = y + 1.402 * v;
    let g = y - 0.344136 * u - 0.714136 * v;
    let b = y + 1.772 * u;
    
    let rgb = vec3<f32>(r, g, b);
    textureStore(output_tex, pos, vec4<f32>(rgb, 1.0));
}
