@group(0) @binding(0) var tex_in: texture_2d<f32>;
@group(0) @binding(1) var tex_out: texture_storage_2d<rgba8unorm, write>;

// Gray-Scott parameters
const Da: f32 = 0.2097;
const Db: f32 = 0.105;
const feed: f32 = 0.037;
const k: f32 = 0.060;
const dt: f32 = 1.0;

@compute @workgroup_size(16, 16, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(tex_in);
    let gx = i32(global_id.x);
    let gy = i32(global_id.y);

    if (gx >= i32(dims.x) || gy >= i32(dims.y)) {
        return;
    }

    // Read 3x3 neighborhood
    var val = textureLoad(tex_in, vec2<i32>(gx, gy), 0).rg;
    let a = val.r;
    let b = val.g;

    var sum = vec2<f32>(0.0, 0.0);
    // Convolution weights for Laplacian
    // 0.05  0.2  0.05
    // 0.2  -1.0  0.2
    // 0.05  0.2  0.05

    sum += textureLoad(tex_in, vec2<i32>(gx - 1, gy - 1), 0).rg * 0.05;
    sum += textureLoad(tex_in, vec2<i32>(gx, gy - 1), 0).rg * 0.2;
    sum += textureLoad(tex_in, vec2<i32>(gx + 1, gy - 1), 0).rg * 0.05;
    sum += textureLoad(tex_in, vec2<i32>(gx - 1, gy), 0).rg * 0.2;
    sum += textureLoad(tex_in, vec2<i32>(gx, gy), 0).rg * -1.0;
    sum += textureLoad(tex_in, vec2<i32>(gx + 1, gy), 0).rg * 0.2;
    sum += textureLoad(tex_in, vec2<i32>(gx - 1, gy + 1), 0).rg * 0.05;
    sum += textureLoad(tex_in, vec2<i32>(gx, gy + 1), 0).rg * 0.2;
    sum += textureLoad(tex_in, vec2<i32>(gx + 1, gy + 1), 0).rg * 0.05;

    let laplacian = sum;

    // Reaction-Diffusion equations
    let abb = a * b * b;
    let new_a = a + (Da * laplacian.r - abb + feed * (1.0 - a)) * dt;
    let new_b = b + (Db * laplacian.g + abb - (k + feed) * b) * dt;

    // Output
    textureStore(tex_out, vec2<i32>(gx, gy), vec4<f32>(clamp(new_a, 0.0, 1.0), clamp(new_b, 0.0, 1.0), 0.0, 1.0));
}
