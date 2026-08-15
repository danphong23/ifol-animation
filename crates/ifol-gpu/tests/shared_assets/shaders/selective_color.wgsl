struct SelectiveColorUniform {
    target_hue: f32,    // 0.0 to 1.0 (e.g. 0.95 = Pink/Red, 0.33 = Green, 0.6 = Blue)
    tolerance: f32,     // Width of hue band
    softness: f32,      // Transition falloff
    saturation_boost: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: SelectiveColorUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[vi], 0.0, 1.0);
    out.uv = uv[vi];
    return out;
}

// Convert RGB to HSV
fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4<f32>(c.bg, K.wz), vec4<f32>(c.gb, K.xy), step(c.b, c.g));
    let q = mix(vec4<f32>(p.xyw, c.r), vec4<f32>(c.r, p.yzx), step(p.x, c.r));

    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

// Convert HSV to RGB
fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.uv);
    if (color.a < 0.01) {
        return color;
    }

    let hsv = rgb2hsv(color.rgb);
    
    // Circular Hue distance on 0.0 to 1.0 circle
    var hue_diff = abs(hsv.x - u_params.target_hue);
    if (hue_diff > 0.5) {
        hue_diff = 1.0 - hue_diff;
    }

    // Calculate mask: 1.0 if inside target hue band, 0.0 outside
    let inner_edge = u_params.tolerance;
    let outer_edge = u_params.tolerance + u_params.softness;
    let keep_factor = 1.0 - smoothstep(inner_edge, outer_edge, hue_diff);

    // Grayscale (Luminance)
    let lum = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let gray = vec3<f32>(lum);

    // Boost saturated target color
    let boosted_hsv = vec3<f32>(hsv.x, clamp(hsv.y * u_params.saturation_boost, 0.0, 1.0), hsv.z);
    let saturated_rgb = hsv2rgb(boosted_hsv);

    let final_rgb = mix(gray, saturated_rgb, keep_factor * step(0.1, hsv.y));
    return vec4<f32>(final_rgb, color.a);
}
