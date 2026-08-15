struct AtlasUniform {
    pos: vec2<f32>,
    scale: vec2<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    texture_size: vec2<f32>,
    enable_clamp: f32,
    tolerance: f32,
    smoothness: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    key_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: AtlasUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var quad = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    let p = quad[vi] * u_params.scale + u_params.pos;
    var out: VertexOutput;
    out.clip_position = vec4<f32>(p, 0.5, 1.0);
    out.uv = uvs[vi];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let raw_atlas_uv = mix(u_params.uv_min, u_params.uv_max, in.uv);

    var sample_uv = raw_atlas_uv;
    if (u_params.enable_clamp > 0.5) {
        // Sub-pixel Half-Texel Inset Clamping to avoid sampling adjacent atlas sprites during bilinear interpolation
        let half_texel = vec2<f32>(0.5 / u_params.texture_size.x, 0.5 / u_params.texture_size.y);
        sample_uv = clamp(raw_atlas_uv, u_params.uv_min + half_texel, u_params.uv_max - half_texel);
    }

    let tex_color = textureSample(t_diffuse, s_diffuse, sample_uv);

    // Chroma key despill
    let diff = distance(tex_color.rgb, u_params.key_color.rgb);
    let alpha = smoothstep(u_params.tolerance, u_params.tolerance + u_params.smoothness, diff);
    
    return vec4<f32>(tex_color.rgb, alpha);
}
