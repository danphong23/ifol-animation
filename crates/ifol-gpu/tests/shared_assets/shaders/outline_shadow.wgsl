struct OutlineUniform {
    outline_color: vec4<f32>,
    shadow_color: vec4<f32>,
    shadow_offset: vec2<f32>,
    texel_size: vec2<f32>,
    outline_thickness: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_input: texture_2d<f32>;
@group(0) @binding(1) var s_input: sampler;
@group(1) @binding(0) var<uniform> fx: OutlineUniform;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let center = textureSample(t_input, s_input, in.uv);
    var result_color = center;

    // 1. Drop Shadow
    // Sample texture at offset
    let shadow_uv = in.uv - fx.shadow_offset;
    var shadow_sample = vec4<f32>(0.0);
    if (shadow_uv.x >= 0.0 && shadow_uv.x <= 1.0 && shadow_uv.y >= 0.0 && shadow_uv.y <= 1.0) {
        shadow_sample = textureSample(t_input, s_input, shadow_uv);
    }
    
    // Composite shadow BEHIND the object
    if (center.a < 0.1 && shadow_sample.a > 0.1) {
        result_color = vec4<f32>(fx.shadow_color.rgb, fx.shadow_color.a * shadow_sample.a);
    } else if (center.a > 0.1) {
        result_color = center; // Object covers shadow
    }

    // 2. Outline (Stroke)
    // If the pixel itself is relatively transparent, but a neighbor is opaque, it gets the outline!
    if (center.a < 0.95 && fx.outline_thickness > 0.0) {
        var max_alpha: f32 = 0.0;
        let t = fx.outline_thickness;
        let ts = fx.texel_size;
        
        // 8-way sampling
        max_alpha = max(max_alpha, textureSample(t_input, s_input, in.uv + vec2<f32>( t,  0.0) * ts).a);
        max_alpha = max(max_alpha, textureSample(t_input, s_input, in.uv + vec2<f32>(-t,  0.0) * ts).a);
        max_alpha = max(max_alpha, textureSample(t_input, s_input, in.uv + vec2<f32>( 0.0,  t) * ts).a);
        max_alpha = max(max_alpha, textureSample(t_input, s_input, in.uv + vec2<f32>( 0.0, -t) * ts).a);
        
        max_alpha = max(max_alpha, textureSample(t_input, s_input, in.uv + vec2<f32>( t,  t) * ts).a);
        max_alpha = max(max_alpha, textureSample(t_input, s_input, in.uv + vec2<f32>(-t, -t) * ts).a);
        max_alpha = max(max_alpha, textureSample(t_input, s_input, in.uv + vec2<f32>(-t,  t) * ts).a);
        max_alpha = max(max_alpha, textureSample(t_input, s_input, in.uv + vec2<f32>( t, -t) * ts).a);

        // Smooth outline blending
        if (max_alpha > 0.1) {
            let outline_alpha = smoothstep(0.1, 0.9, max_alpha) * fx.outline_color.a;
            // Blend outline OVER shadow but UNDER center
            result_color = mix(result_color, fx.outline_color, outline_alpha * (1.0 - center.a));
        }
    }

    return result_color;
}
