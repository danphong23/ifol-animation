struct Params {
    valid_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<storage, read> src_buffer: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst_buffer: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.valid_count) {
        return;
    }
    // Multiply by 3.0 and add 0.5
    dst_buffer[idx] = src_buffer[idx] * 3.0 + 0.5;
}
