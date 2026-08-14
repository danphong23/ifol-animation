struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local_pos: vec2<f32>, // Used for SDF shape
};

struct SpriteUniform {
    transform: mat4x4<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    key_color: vec3<f32>,
    tolerance: f32,
    smoothness: f32,
    opacity: f32,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> config: SpriteUniform;

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
    out.local_pos = p; // -1 to 1 local coordinate
    
    // Apply transform (Scale + Translation)
    out.clip_position = config.transform * vec4<f32>(p, 0.0, 1.0);
    
    // Map UV coordinates
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

// SDF Rounded Rectangle
fn sd_round_rect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex_color = textureSampleLevel(t_diffuse, s_diffuse, in.uv, 0.0);
    
    let target_hsv = rgb2hsv(config.key_color);
    let color_hsv = rgb2hsv(tex_color.rgb);
    
    var hue_diff = abs(color_hsv.x - target_hsv.x);
    if (hue_diff > 0.5) {
        hue_diff = 1.0 - hue_diff;
    }
    
    let sat_diff = abs(color_hsv.y - target_hsv.y);
    let val_diff = abs(color_hsv.z - target_hsv.z);
    let color_dist = hue_diff * 4.0 + sat_diff * 0.5 + val_diff * 0.5;
    
    let chroma_alpha = smoothstep(config.tolerance - config.smoothness, config.tolerance + config.smoothness, color_dist);
    
    if (chroma_alpha < 1.0) {
        let max_rb = max(tex_color.r, tex_color.b);
        if (tex_color.g > max_rb) {
            tex_color.g = max_rb;
        }
    }
    
    // Masking logic using SDF
    // The mask is a circle or rounded rect in the local coordinate space (-1 to 1)
    // Let's create an elegant Avatar Circle mask
    let mask_dist = length(in.local_pos) - 0.95; 
    let mask_alpha = 1.0 - smoothstep(0.0, 0.05, mask_dist);
    
    // Final alpha is chroma * mask * opacity
    let final_alpha = chroma_alpha * mask_alpha * config.opacity * tex_color.a;
    
    return vec4<f32>(tex_color.rgb, final_alpha);
}
