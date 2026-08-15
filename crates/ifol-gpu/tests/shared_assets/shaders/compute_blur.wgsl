@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var dst_texture: texture_storage_2d<rgba8unorm, write>;

struct Params {
    dir: vec2<f32>,
    radius: i32,
    pad: i32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dim = textureDimensions(src_texture);
    if (global_id.x >= dim.x || global_id.y >= dim.y) {
        return;
    }
    
    let pos = vec2<i32>(global_id.xy);
    let dir = vec2<i32>(params.dir);
    let r = params.radius;
    
    var color = vec4<f32>(0.0);
    var total_weight = 0.0;
    
    let sigma = f32(r) / 2.0;
    let two_sigma_sq = max(2.0 * sigma * sigma, 0.001);
    
    for (var i = -r; i <= r; i++) {
        let offset_pos = pos + dir * i;
        let clamped_pos = clamp(offset_pos, vec2<i32>(0), vec2<i32>(dim) - vec2<i32>(1));
        
        let sample = textureLoad(src_texture, clamped_pos, 0);
        
        let weight = exp(-f32(i * i) / two_sigma_sq);
        color += sample * weight;
        total_weight += weight;
    }
    
    color = color / total_weight;
    textureStore(dst_texture, pos, color);
}
