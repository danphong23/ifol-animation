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

fn dot2(v: vec2<f32>) -> f32 { return dot(v,v); }

fn sdBezier(pos: vec2<f32>, A: vec2<f32>, B: vec2<f32>, C: vec2<f32>) -> f32 {
    let a = B - A;
    let b = A - 2.0*B + C;
    let c = a * 2.0;
    let d = A - pos;

    let kk = 1.0 / dot(b,b);
    let kx = kk * dot(a,b);
    let ky = kk * (2.0*dot(a,a)+dot(d,b)) / 3.0;
    let kz = kk * dot(d,a);

    var res = 0.0;
    let p = ky - kx*kx;
    let p3 = p*p*p;
    let q = kx*(2.0*kx*kx - 3.0*ky) + kz;
    let h = q*q + 4.0*p3;

    if(h >= 0.0) {
        let h_sqrt = sqrt(h);
        let x = (vec2<f32>(h_sqrt, -h_sqrt) - q) / 2.0;
        let uv = sign(x)*pow(abs(x), vec2<f32>(1.0/3.0));
        let t = clamp(uv.x+uv.y-kx, 0.0, 1.0);
        res = dot2(d + (c + b*t)*t);
    } else {
        let z = sqrt(-p);
        let v = acos( q/(p*z*2.0) ) / 3.0;
        let m = cos(v);
        let n = sin(v)*1.732050808;
        let t = clamp( vec3<f32>(m + m, -n - m, n - m) * z - kx, vec3<f32>(0.0), vec3<f32>(1.0) );
        res = min( dot2(d+(c+b*t.x)*t.x), dot2(d+(c+b*t.y)*t.y) );
        res = min(res, dot2(d+(c+b*t.z)*t.z));
    }
    return sqrt(res);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv;
    
    // 3 Control points of the Bezier Curve
    let p0 = vec2<f32>(0.1, 0.8);
    let p1 = vec2<f32>(0.9, 0.9);
    let p2 = vec2<f32>(0.5, 0.2);
    
    let d = sdBezier(p, p0, p1, p2);
    
    let thickness = 0.03;
    let alpha = 1.0 - smoothstep(thickness - 0.005, thickness + 0.005, d);
    
    let bg = vec3<f32>(0.1, 0.1, 0.15);
    let fg = vec3<f32>(1.0, 0.4, 0.2);
    
    // add control points visualize
    let d0 = length(p - p0);
    let d1 = length(p - p1);
    let d2 = length(p - p2);
    let d_pts = min(min(d0, d1), d2);
    
    var color = mix(bg, fg, alpha);
    if (d_pts < 0.02) {
        color = vec3<f32>(0.2, 0.8, 1.0); // blue points
    }
    
    // add bounding triangle visualize
    let d_l1 = sdBezier(p, p0, mix(p0,p1,0.5), p1); // wait, actually just straight line to P1
    
    return vec4<f32>(color, 1.0);
}
