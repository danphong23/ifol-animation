# Báo cáo: TC93_STORAGE_TEXTURE_3D - 3D Storage Texture & Voxel Density Field Raymarching

Đây là báo cáo tổng hợp chi tiết kỹ thuật bài kiểm thử **3D Storage Texture Write (`texture_storage_3d`)** và **3D Volume Raymarching** cho TC93.

---

## 1. Môi trường & Thông số Thực thi Desktop (Tauri/wgpu)

- **Kích thước Thể Tích Voxel 3D:** $64 \times 64 \times 64$ ($262,144$ voxels)
- **Định dạng Texture 3D:** `wgpu::TextureFormat::Rgba8Unorm`
- **Cấu hình Workgroup Compute:** `[8, 8, 4]` (256 invocations / workgroup - Tuân thủ tuyệt đối giới hạn WebGPU)
- **Cấu hình Dispatch Workgroups:** `[8, 8, 16]` (Tổng $8 \times 8 \times 16 = 1,024$ workgroups $= 262,144$ luồng GPU)
- **Thời gian Thực thi Cold Start:** 11.63ms
- **Thời gian Thực thi Warm/Cached:** 5.12ms

### Kết quả Ảnh Render (Thực tế Direct Readback):

<img src="../outputs/desktop/tc93_storage_texture_3d.png" alt="TC93 Desktop Render" />

---

## 2. Phân Tích Kiến Trúc 3D Voxel Pipeline & Shader Code

Mô phỏng sương mù thể tích (Voxel Fog), khói 3D và các trường khoảng cách 3D (3D SDF Volume) đòi hỏi khả năng ghi trực tiếp từ Compute Shader vào Texture 3 chiều.

### 2.1 Compute Shader Ghi Dữ Liệu Thể Tích 3D (`compute_3d_voxel.wgsl`):
```wgsl
@group(0) @binding(0) var voxel_tex: texture_storage_3d<rgba8unorm, write>;

@compute @workgroup_size(8, 8, 4)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(voxel_tex);
    if (global_id.x >= dims.x || global_id.y >= dims.y || global_id.z >= dims.z) { return; }

    let pos = vec3<f32>(global_id) / vec3<f32>(dims);
    let dist = length(pos - vec3<f32>(0.5));

    // Trường mật độ 3D kết hợp sóng nhiễu 3D Procedural
    let density = smoothstep(0.45, 0.0, dist) * (sin(pos.x * 20.0) * cos(pos.y * 20.0) * 0.5 + 0.5);

    textureStore(voxel_tex, vec3<i32>(global_id), vec4<f32>(pos * density, density));
}
```

### 2.2 Fragment Shader Raymarching Khối 3D (`render_3d_voxel.wgsl`):
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ray_origin = vec3<f32>(in.uv.x, in.uv.y, 0.0);
    let ray_dir = vec3<f32>(0.0, 0.0, 1.0);
    var accumulated_color = vec4<f32>(0.0);

    // Front-to-back Volume Raymarching qua 64 bước lấy mẫu
    for (var i = 0; i < 64; i++) {
        let p = ray_origin + ray_dir * (f32(i) / 64.0);
        let sample_val = textureSampleLevel(voxel_tex, voxel_sampler, p, 0.0);
        let alpha = sample_val.a * 0.1;
        accumulated_color += vec4<f32>(sample_val.rgb * alpha, alpha);
        if (accumulated_color.a >= 0.98) { break; }
    }
    return vec4<f32>(accumulated_color.rgb, 1.0);
}
```

---

## 3. Xác Thực Trực Quan AI & Độ Hoàn Thiện

- **Xác minh không gian 3D:** Khối cầu mật độ 3D hiển thị dạng đám mây sương mù phát sáng có chiều sâu chân thực khi tia raymarch xuyên qua mảng $64 \times 64 \times 64$ voxels.
- **Tối ưu hóa Workgroup Limit:** Chuyển đổi từ `[8, 8, 8]` (512 invocations) sang `[8, 8, 4]` (256 invocations) đảm bảo tương thích 100% với WebGPU Standard Limit `max_compute_invocations_per_workgroup = 256`.
- **Trạng thái:** **PASSED (Xử lý 3D Storage Texture & Raymarching hoàn hảo)**

---

## 4. Tương Thích Đa Nền Tảng (Cross-Platform & WebGPU Compatibility)

- **WebGPU Feature Flag:** Cần đảm bảo thiết bị hỗ trợ `bgra8unorm-storage` hoặc `rgba8unorm` cho storage texture.
