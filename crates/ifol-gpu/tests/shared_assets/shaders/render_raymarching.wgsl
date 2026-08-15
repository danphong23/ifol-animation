struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = vec2<f32>(0.0);
    var uv = vec2<f32>(0.0);
    if (vertex_index == 0u) { pos = vec2<f32>(-1.0, -1.0); uv = vec2<f32>(-1.0, -1.0); }
    else if (vertex_index == 1u) { pos = vec2<f32>( 3.0, -1.0); uv = vec2<f32>( 3.0, -1.0); }
    else if (vertex_index == 2u) { pos = vec2<f32>(-1.0,  3.0); uv = vec2<f32>(-1.0,  3.0); }
    
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    return out;
}

// 3D Torus SDF
fn sdTorus(p: vec3<f32>, t: vec2<f32>) -> f32 {
    let q = vec2<f32>(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

// Scene SDF (Torus rotated)
fn map(p: vec3<f32>) -> f32 {
    // Rotation matrix
    let angle = 0.6;
    let c = cos(angle);
    let s = sin(angle);
    let rot_x = mat3x3<f32>(
        1.0, 0.0, 0.0,
        0.0, c, -s,
        0.0, s, c
    );
    let rot_y = mat3x3<f32>(
        c, 0.0, s,
        0.0, 1.0, 0.0,
        -s, 0.0, c
    );
    let rotated_p = rot_y * rot_x * p;
    return sdTorus(rotated_p, vec2<f32>(1.0, 0.35));
}

// Estimate normal
fn calcNormal(p: vec3<f32>) -> vec3<f32> {
    let e = 0.001;
    return normalize(vec3<f32>(
        map(p + vec3<f32>(e, 0.0, 0.0)) - map(p - vec3<f32>(e, 0.0, 0.0)),
        map(p + vec3<f32>(0.0, e, 0.0)) - map(p - vec3<f32>(0.0, e, 0.0)),
        map(p + vec3<f32>(0.0, 0.0, e)) - map(p - vec3<f32>(0.0, 0.0, e))
    ));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ray origin & Direction
    let ro = vec3<f32>(0.0, 0.0, -3.0);
    let rd = normalize(vec3<f32>(in.uv.x * 1.333, in.uv.y, 1.5));
    
    var t = 0.0;
    var hit = false;
    var p = vec3<f32>(0.0);
    
    // Raymarching loop
    for (var i = 0; i < 96; i++) {
        p = ro + rd * t;
        let d = map(p);
        if (d < 0.001) {
            hit = true;
            break;
        }
        if (t > 10.0) {
            break;
        }
        t += d;
    }
    
    let bg_color = vec3<f32>(0.1, 0.12, 0.18);
    
    if (!hit) {
        return vec4<f32>(bg_color, 1.0);
    }
    
    // Lighting calculation
    let n = calcNormal(p);
    let light_dir = normalize(vec3<f32>(1.0, 2.0, -1.0));
    
    let diff = max(dot(n, light_dir), 0.0);
    let ambient = 0.15;
    
    // Specular
    let view_dir = normalize(-rd);
    let half_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(n, half_dir), 0.0), 32.0);
    
    let base_color = vec3<f32>(0.9, 0.45, 0.15); // Vibrant Torus Orange
    let color = base_color * (diff + ambient) + vec3<f32>(1.0) * spec * 0.6;
    
    return vec4<f32>(color, 1.0);
}
