@group(0) @binding(0) var storage_tex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(storage_tex);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    let uv = vec2<f32>(global_id.xy) / vec2<f32>(dims);
    let dist = length(uv - vec2<f32>(0.5, 0.5));

    // Dynamic wave pattern
    let wave = sin(dist * 30.0) * 0.5 + 0.5;
    let r = wave;
    let g = sin(uv.x * 20.0) * 0.5 + 0.5;
    let b = cos(uv.y * 20.0) * 0.5 + 0.5;

    textureStore(storage_tex, vec2<i32>(global_id.xy), vec4<f32>(r, g, b, 1.0));
}
