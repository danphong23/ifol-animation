// Parallel Exclusive Scan (Blelloch Algorithm) on GPU Workgroup
@group(0) @binding(0) var<storage, read_write> data: array<u32, 256>;

var<workgroup> temp: array<u32, 256>;

@compute @workgroup_size(256)
fn cs_main(@builtin(local_invocation_id) local_id: vec3<u32>) {
    let tid = local_id.x;
    
    // Tải dữ liệu vào Shared Memory
    temp[tid] = data[tid];
    workgroupBarrier();
    
    // Up-sweep (Reduce) Phase
    var offset = 1u;
    for (var d = 128u; d > 0u; d >>= 1u) {
        if (tid < d) {
            let ai = offset * (2u * tid + 1u) - 1u;
            let bi = offset * (2u * tid + 2u) - 1u;
            temp[bi] += temp[ai];
        }
        offset *= 2u;
        workgroupBarrier();
    }
    
    // Clear last element for Exclusive Scan
    if (tid == 0u) {
        temp[255] = 0u;
    }
    workgroupBarrier();
    
    // Down-sweep Phase
    for (var d = 1u; d < 256u; d *= 2u) {
        offset >>= 1u;
        workgroupBarrier();
        if (tid < d) {
            let ai = offset * (2u * tid + 1u) - 1u;
            let bi = offset * (2u * tid + 2u) - 1u;
            let t = temp[ai];
            temp[ai] = temp[bi];
            temp[bi] += t;
        }
    }
    workgroupBarrier();
    
    // Ghi kết quả Exclusive Scan ra Storage Buffer
    data[tid] = temp[tid];
}
