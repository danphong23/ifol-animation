struct ReplaceUniform {
    transform: mat4x4<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    target_hsv: vec4<f32>,
    replace_hsv: vec4<f32>,
    tolerance: f32,
    smoothness: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> config: ReplaceUniform;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
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
    
    let p = pos[in_vertex_index];
    out.clip_position = config.transform * vec4<f32>(p, 0.0, 1.0);
    out.uv = mix(config.uv_min, config.uv_max, uv[in_vertex_index]);
    
    return out;
}

// Convert RGB to HSV
fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = select(vec4<f32>(c.bg, K.wz), vec4<f32>(c.gb, K.xy), c.b < c.g);
    let q = select(vec4<f32>(p.xyw, c.r), vec4<f32>(c.r, p.yzx), p.x < c.r);
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
    let tex_color = textureSampleLevel(t_diffuse, s_diffuse, in.uv, 0.0);
    if (tex_color.a < 0.05) {
        discard;
    }
    
    let color_hsv = rgb2hsv(tex_color.rgb);
    
    var hue_diff = abs(color_hsv.x - config.target_hsv.x);
    if (hue_diff > 0.5) {
        hue_diff = 1.0 - hue_diff;
    }
    
    let sat_diff = abs(color_hsv.y - config.target_hsv.y);
    // Ignore value difference to capture shadows and highlights
    let color_dist = hue_diff + sat_diff * 0.5;
    
    let replace_weight = 1.0 - smoothstep(config.tolerance - config.smoothness, config.tolerance + config.smoothness, color_dist);
    
    // Shift the hue and saturation, but keep original value (shading)
    var new_hsv = color_hsv;
    new_hsv.x = fract(color_hsv.x + (config.replace_hsv.x - config.target_hsv.x) + 1.0);
    
    // For saturation, we could do a direct replacement or an offset. 
    // Direct replacement can look flat if the original had saturation gradients.
    // Let's just scale it relative to target.
    new_hsv.y = clamp(color_hsv.y * (config.replace_hsv.y / (config.target_hsv.y + 0.001)), 0.0, 1.0);
    
    let replaced_rgb = hsv2rgb(new_hsv);
    let final_rgb = mix(tex_color.rgb, replaced_rgb, replace_weight);
    
    // Add green chroma key logic as a bonus so it looks nice!
    var out_rgba = vec4<f32>(final_rgb, tex_color.a);
    let max_rb = max(out_rgba.r, out_rgba.b);
    if (out_rgba.g > max_rb * 1.1) {
        out_rgba.g = max_rb;
        out_rgba.a = 0.0;
    }
    
    return out_rgba;
}
