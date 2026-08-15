struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = vec2<f32>(0.0);
    var uv = vec2<f32>(0.0);
    if (vertex_index == 0u) { pos = vec2<f32>(-1.0, -1.0); uv = vec2<f32>(0.0, 1.0); }
    else if (vertex_index == 1u) { pos = vec2<f32>( 3.0, -1.0); uv = vec2<f32>(2.0, 1.0); }
    else if (vertex_index == 2u) { pos = vec2<f32>(-1.0,  3.0); uv = vec2<f32>(0.0, -1.0); }
    
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let scale = 12.0;
    let u = in.uv * scale;
    
    let i_u = floor(u);
    let f_u = fract(u);
    
    var min_dist = 1.0;
    var cell_point = vec2<f32>(0.0);
    
    for(var y = -1; y <= 1; y++) {
        for(var x = -1; x <= 1; x++) {
            let neighbor = vec2<f32>(f32(x), f32(y));
            
            // Random point inside the neighbor cell
            // (We can add time animation here if needed)
            let point = hash22(i_u + neighbor);
            
            let diff = neighbor + point - f_u;
            let dist = length(diff);
            
            if (dist < min_dist) {
                min_dist = dist;
                cell_point = point;
            }
        }
    }
    
    // Draw cells
    let color = vec3<f32>(min_dist * 0.8, min_dist * 0.9, min_dist * 1.0);
    
    // Add borders
    let border = smoothstep(0.0, 0.02, min_dist);
    let final_color = mix(vec3<f32>(0.0, 0.8, 1.0), color, border);
    
    return vec4<f32>(final_color, 1.0);
}
