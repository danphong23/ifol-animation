struct PostProcessUniform {
    bloom_intensity: f32,
    exposure: f32,
    contrast: f32,
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_scene: sampler;
@group(1) @binding(0) var<uniform> params: PostProcessUniform;

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
    let base = textureSample(t_scene, s_scene, in.uv);

    // Multi-radius soft gaussian-like blur for radiant celestial bloom
    let r1 = vec2<f32>(0.004, 0.005);
    let r2 = vec2<f32>(0.009, 0.012);

    let tap1 = textureSample(t_scene, s_scene, in.uv + vec2<f32>(r1.x, r1.y));
    let tap2 = textureSample(t_scene, s_scene, in.uv + vec2<f32>(-r1.x, r1.y));
    let tap3 = textureSample(t_scene, s_scene, in.uv + vec2<f32>(r1.x, -r1.y));
    let tap4 = textureSample(t_scene, s_scene, in.uv + vec2<f32>(-r1.x, -r1.y));

    let tap5 = textureSample(t_scene, s_scene, in.uv + vec2<f32>(r2.x, 0.0));
    let tap6 = textureSample(t_scene, s_scene, in.uv + vec2<f32>(-r2.x, 0.0));
    let tap7 = textureSample(t_scene, s_scene, in.uv + vec2<f32>(0.0, r2.y));
    let tap8 = textureSample(t_scene, s_scene, in.uv + vec2<f32>(0.0, -r2.y));

    let blur_narrow = (tap1 + tap2 + tap3 + tap4) * 0.25;
    let blur_wide = (tap5 + tap6 + tap7 + tap8) * 0.25;
    let combined_blur = blur_narrow * 0.6 + blur_wide * 0.4;

    // High threshold bloom extraction: only ultra-bright stars and radiant moon outer halo
    let lum = dot(combined_blur.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let bloom_mask = smoothstep(0.68, 1.10, lum);
    let bloom_radiance = combined_blur.rgb * bloom_mask * params.bloom_intensity;

    // Soft celestial moonlight tint for bloom glow
    let bloom_tint = vec3<f32>(0.85, 0.95, 1.25);
    var final_color = base.rgb * params.exposure + bloom_radiance * bloom_tint;

    // Cinematic tone curve
    final_color = pow(final_color, vec3<f32>(1.0 / params.contrast));

    return vec4<f32>(clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
