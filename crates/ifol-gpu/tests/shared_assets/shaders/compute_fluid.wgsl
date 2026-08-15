struct Params {
    time: f32,
    dt: f32,
    width: u32,
    height: u32,
}

@group(0) @binding(0) var density_in: texture_2d<f32>;
@group(0) @binding(1) var density_out: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dim = vec2<u32>(params.width, params.height);
    if (global_id.x >= dim.x || global_id.y >= dim.y) {
        return;
    }
    
    let pos = vec2<i32>(global_id.xy);
    let uv = vec2<f32>(global_id.xy) / vec2<f32>(dim);
    
    // Multi-center swirl vortex field
    let c1 = vec2<f32>(0.35 + 0.15 * sin(params.time * 2.0), 0.5 + 0.15 * cos(params.time * 2.0));
    let c2 = vec2<f32>(0.65 - 0.15 * cos(params.time * 1.5), 0.5 - 0.15 * sin(params.time * 1.5));
    
    let d1 = uv - c1;
    let d2 = uv - c2;
    
    let r1 = length(d1);
    let r2 = length(d2);
    
    // Rich fluid density pattern
    var color = vec4<f32>(0.08, 0.08, 0.12, 1.0);
    
    let swirl1 = smoothstep(0.3, 0.0, r1) * sin(r1 * 30.0 - params.time * 5.0);
    let swirl2 = smoothstep(0.3, 0.0, r2) * cos(r2 * 25.0 + params.time * 4.0);
    
    color += vec4<f32>(1.0, 0.4, 0.1, 1.0) * max(swirl1, 0.0);
    color += vec4<f32>(0.1, 0.7, 1.0, 1.0) * max(swirl2, 0.0);
    
    // Dynamic fluid smoke trails
    let wave = sin(uv.x * 20.0 + params.time * 3.0) * 0.05 + sin(uv.y * 15.0 - params.time * 2.0) * 0.05;
    let trail = smoothstep(0.08, 0.0, abs(uv.y - 0.5 + wave));
    color += vec4<f32>(0.8, 0.2, 0.8, 1.0) * trail * 0.6;
    
    textureStore(density_out, pos, color);
}
