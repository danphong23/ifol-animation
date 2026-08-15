struct DataArray {
    elements: array<vec4<f32>>,
};

@group(0) @binding(0) var<storage, read> input_a: DataArray;
@group(0) @binding(1) var<storage, read> input_b: DataArray;
@group(0) @binding(2) var<storage, read_write> output_c: DataArray;

@compute @workgroup_size(64, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    // Guard against out of bounds
    let total_count = arrayLength(&input_a.elements);
    if (idx >= total_count) {
        return;
    }

    let a = input_a.elements[idx];
    let b = input_b.elements[idx];
    
    // Mathematical formula on GPU:
    // C[idx] = A[idx] * 2.0 + sin(B[idx]) * 1.5 + cos(f32(idx) * 0.01)
    let idx_f = f32(idx) * 0.01;
    let wave = cos(vec4<f32>(idx_f, idx_f + 0.5, idx_f + 1.0, idx_f + 1.5));
    
    output_c.elements[idx] = a * 2.0 + sin(b) * 1.5 + wave;
}
