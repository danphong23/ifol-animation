// Workgroup Shared Memory 2D Fast Blur (16x16 workgroup with 4-pixel apron -> 24x24 tile)

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

// 24x24 On-Chip Shared Memory Tile
var<workgroup> s_tile: array<array<vec4<f32>, 24>, 24>;

@compute @workgroup_size(16, 16, 1)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let in_dims = textureDimensions(input_tex);
    let in_w = i32(in_dims.x);
    let in_h = i32(in_dims.y);

    let base_x = i32(workgroup_id.x * 16u) - 4;
    let base_y = i32(workgroup_id.y * 16u) - 4;

    let lx = local_id.x;
    let ly = local_id.y;
    let linear_id = ly * 16u + lx; // 0..255

    // Cooperative tile loading: 256 threads load 576 (24x24) pixels into shared memory
    // Each thread loads 2 or 3 pixels
    for (var i = linear_id; i < 576u; i = i + 256u) {
        let ty = i / 24u;
        let tx = i % 24u;

        let gx = clamp(base_x + i32(tx), 0, in_w - 1);
        let gy = clamp(base_y + i32(ty), 0, in_h - 1);

        s_tile[ty][tx] = textureLoad(input_tex, vec2<i32>(gx, gy), 0);
    }

    // Synchronize all threads until the shared memory tile is fully populated
    workgroupBarrier();

    let gx = global_id.x;
    let gy = global_id.y;

    if (gx >= 800u || gy >= 600u) {
        return;
    }

    // Center divider golden marker (x == 398..402)
    if (gx >= 398u && gx <= 402u) {
        textureStore(output_tex, vec2<u32>(gx, gy), vec4<f32>(1.0, 0.8, 0.2, 1.0));
        return;
    }

    // LEFT HALF: Original sharp image (1:1 mapping)
    if (gx < 400u) {
        let src_col = textureLoad(input_tex, vec2<i32>(i32(gx), i32(gy)), 0);
        textureStore(output_tex, vec2<u32>(gx, gy), src_col);
        return;
    }

    // RIGHT HALF: 9x9 Gaussian Filter from Shared Memory
    let tile_cx = lx + 4u;
    let tile_cy = ly + 4u;

    var accum = vec4<f32>(0.0);
    var weight_sum = 0.0;

    // 1D Gaussian kernel weights for radius 4: [0.05, 0.09, 0.12, 0.15, 0.18, 0.15, 0.12, 0.09, 0.05]
    let weights = array<f32, 9>(0.05, 0.09, 0.12, 0.15, 0.18, 0.15, 0.12, 0.09, 0.05);

    for (var dy = -4; dy <= 4; dy = dy + 1) {
        let wy = weights[dy + 4];
        for (var dx = -4; dx <= 4; dx = dx + 1) {
            let wx = weights[dx + 4];
            let w = wx * wy;

            let sample_col = s_tile[u32(i32(tile_cy) + dy)][u32(i32(tile_cx) + dx)];
            accum = accum + sample_col * w;
            weight_sum = weight_sum + w;
        }
    }

    let blurred_col = accum / weight_sum;
    textureStore(output_tex, vec2<u32>(gx, gy), blurred_col);
}
