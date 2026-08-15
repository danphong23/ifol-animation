var<workgroup> local_histogram: array<atomic<u32>, 256>;

@group(0) @binding(0) var<storage, read_write> global_histogram: array<atomic<u32>, 256>;

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_index) local_idx: u32) {
    // 1. Clear local workgroup memory
    atomicStore(&local_histogram[local_idx], 0u);
    workgroupBarrier();

    // 2. Compute bin index for 102,400 threads
    let thread_idx = global_id.x;
    if (thread_idx < 102400u) {
        let f = f32(thread_idx) * 0.01;
        let wave = sin(f) * 0.5 + 0.5; // [0.0 .. 1.0]
        let bin = u32(clamp(wave * 255.0, 0.0, 255.0));

        // Atomic Add on local Workgroup Shared Memory
        atomicAdd(&local_histogram[bin], 1u);
    }

    workgroupBarrier();

    // 3. Reduce local histogram into global storage buffer
    let local_count = atomicLoad(&local_histogram[local_idx]);
    if (local_count > 0u) {
        atomicAdd(&global_histogram[local_idx], local_count);
    }
}
