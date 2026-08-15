@group(0) @binding(0) var voxel_tex: texture_storage_3d<rgba8unorm, write>;

@compute @workgroup_size(8, 8, 4)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(voxel_tex);
    if (global_id.x >= dims.x || global_id.y >= dims.y || global_id.z >= dims.z) {
        return;
    }

    let pos = vec3<f32>(global_id) / vec3<f32>(dims); // [0..1]
    let center = vec3<f32>(0.5, 0.5, 0.5);
    let dist = length(pos - center);

    // 3D Sphere density field + noise wave
    let density = smoothstep(0.45, 0.0, dist) * (sin(pos.x * 20.0) * cos(pos.y * 20.0) * 0.5 + 0.5);

    let color = vec4<f32>(
        pos.x * density,
        pos.y * density,
        pos.z * density,
        density
    );

    textureStore(voxel_tex, vec3<i32>(global_id), color);
}
