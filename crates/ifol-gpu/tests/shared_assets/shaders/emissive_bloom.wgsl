struct BloomUniform {
    threshold: f32,
    intensity: f32,
    blur_radius: f32,
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: BloomUniform;

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

// Wide Full-screen Gaussian Glow Extractor
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var glow = vec4<f32>(0.0);
    var total_weight = 0.0;
    let radius = u_params.blur_radius * 0.003;

    // Sample a 9x9 Gaussian disk to bleed far beyond sprite boundary
    for (var x = -4; x <= 4; x++) {
        for (var y = -4; y <= 4; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * radius;
            let sample_color = textureSample(t_diffuse, s_diffuse, in.uv + offset);
            
            // Brightness calculation
            let lum = dot(sample_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
            let weight = exp(-f32(x*x + y*y) / 12.0);
            
            // Only add bright parts or glowing aura
            if (lum > u_params.threshold || sample_color.a > 0.1) {
                // Boost blue/cyan mystical magic glow
                let boosted = sample_color.rgb * 1.5 + vec3<f32>(0.2, 0.6, 1.0) * sample_color.a;
                glow += vec4<f32>(boosted * weight, sample_color.a * weight);
            }
            total_weight += weight;
        }
    }

    let final_glow = (glow / total_weight) * u_params.intensity;
    return final_glow;
}
