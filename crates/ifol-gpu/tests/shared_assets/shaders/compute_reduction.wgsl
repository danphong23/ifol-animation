var<workgroup> local_max: array<f32, 256>;

@group(0) @binding(0) var<storage, read> input_buf: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_buf: array<f32>;

@compute @workgroup_size(256)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let tid = local_id.x;
    let gid = global_id.x;

    // Load into workgroup shared memory
    local_max[tid] = input_buf[gid];
    workgroupBarrier();

    // Tree-based Parallel Reduction in Workgroup Shared Memory
    for (var s = 128u; s > 0u; s >>= 1u) {
        if (tid < s) {
            local_max[tid] = max(local_max[tid], local_max[tid + s]);
        }
        workgroupBarrier();
    }

    // Workgroup leader writes reduced max value to output buffer slot
    if (tid == 0u) {
        output_buf[workgroup_id.x] = local_max[0];
    }
}
