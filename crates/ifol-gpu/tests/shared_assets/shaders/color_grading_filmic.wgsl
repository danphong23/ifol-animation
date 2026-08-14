struct ColorGradingUniform {
    params: vec4<f32>,          // exposure, contrast, saturation, temperature
    shadow_tint_vig: vec4<f32>, // shadow_r, shadow_g, shadow_b, vignette_strength
    highlight_tint: vec4<f32>,  // highlight_r, highlight_g, highlight_b, 0.0
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_input: texture_2d<f32>;
@group(0) @binding(1) var s_input: sampler;
@group(1) @binding(0) var<uniform> grade: ColorGradingUniform;

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

fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let raw = textureSample(t_input, s_input, in.uv);
    var color = raw.rgb;

    let exposure = grade.params.x;
    let contrast = grade.params.y;
    let saturation = grade.params.z;
    let temperature = grade.params.w;
    let shadow_tint = grade.shadow_tint_vig.xyz;
    let vignette_strength = grade.shadow_tint_vig.w;
    let highlight_tint = grade.highlight_tint.xyz;

    // 1. Exposure
    color = color * pow(2.0, exposure);

    // 2. Temperature (Warm vs Cool)
    let warm_tint = vec3<f32>(1.12, 1.02, 0.90);
    let cool_tint = vec3<f32>(0.90, 1.02, 1.15);
    let temp_tint = mix(cool_tint, warm_tint, temperature * 0.5 + 0.5);
    color = color * temp_tint;

    // 3. Contrast
    color = (color - vec3<f32>(0.5)) * contrast + vec3<f32>(0.5);

    // 4. Saturation
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = mix(vec3<f32>(luma), color, saturation);

    // 5. Split Toning
    let shadow_weight = clamp(1.0 - luma * 2.0, 0.0, 1.0);
    let highlight_weight = clamp((luma - 0.5) * 2.0, 0.0, 1.0);
    color = color + shadow_tint * shadow_weight * 0.25;
    color = color + highlight_tint * highlight_weight * 0.25;

    // 6. ACES Filmic Tone Mapping
    color = aces_filmic(color);

    // 7. Vignette
    let coord = in.uv * (1.0 - in.uv.yx);
    let vig = coord.x * coord.y * 15.0;
    let vignette = clamp(pow(vig, vignette_strength), 0.0, 1.0);
    color = color * vignette;

    return vec4<f32>(color, raw.a);
}
