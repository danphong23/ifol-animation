@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> global_hist: array<atomic<u32>, 256>;

var<workgroup> local_hist: array<atomic<u32>, 256>;

@compute @workgroup_size(16, 16, 1)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_index) local_idx: u32,
) {
    // 1. Initialize local histogram
    if (local_idx < 256u) {
        atomicStore(&local_hist[local_idx], 0u);
    }
    workgroupBarrier();

    let gx = global_id.x;
    let gy = global_id.y;

    // 2. Sample pixel, compute luminance, and atomic increment local histogram
    if (gx < 800u && gy < 600u) {
        let color = textureLoad(input_tex, vec2<i32>(i32(gx), i32(gy)), 0).rgb;
        // Rec. 709 luma coefficients
        let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
        
        let bin_idx = clamp(u32(luma * 255.0), 0u, 255u);
        atomicAdd(&local_hist[bin_idx], 1u);
    }
    workgroupBarrier();

    // 3. Merge local histogram into global histogram
    if (local_idx < 256u) {
        let local_val = atomicLoad(&local_hist[local_idx]);
        if (local_val > 0u) {
            atomicAdd(&global_hist[local_idx], local_val);
        }
    }
}
