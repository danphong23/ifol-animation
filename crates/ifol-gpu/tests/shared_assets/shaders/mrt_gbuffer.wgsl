struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

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

struct GBufferOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) emissive: vec4<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> GBufferOutput {
    let color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    var out: GBufferOutput;
    // Attachment 0: Albedo Base
    out.albedo = vec4<f32>(color.rgb, 1.0);
    
    // Attachment 1: Emissive extraction (Magic golden / neon / fire highlights)
    let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let is_warm_glow = color.r > 0.4 && color.g > 0.3 && color.b < 0.3; // Golden / Orange glow
    let is_high_bright = luminance > 0.65;
    
    if (is_warm_glow || is_high_bright) {
        out.emissive = vec4<f32>(color.rgb * 1.5, 1.0);
    } else {
        out.emissive = vec4<f32>(0.02, 0.02, 0.05, 1.0);
    }

    return out;
}
