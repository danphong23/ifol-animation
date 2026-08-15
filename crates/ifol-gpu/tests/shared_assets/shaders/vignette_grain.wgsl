struct VignetteUniform {
    vignette_radius: f32, // radius of clear circle
    vignette_softness: f32, // blur at edge
    grain_strength: f32, // how much noise
    time: f32, // for animating noise
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: VignetteUniform;

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

// Pseudo-random function
fn rand(uv: vec2<f32>) -> f32 {
    return fract(sin(dot(uv.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    // Vignette
    let dist = distance(in.uv, vec2<f32>(0.5, 0.5));
    let vignette = smoothstep(u_params.vignette_radius, u_params.vignette_radius - u_params.vignette_softness, dist);
    color = vec4<f32>(color.rgb * vignette, color.a);
    
    // Film grain
    let noise = (rand(in.uv + u_params.time) - 0.5) * 2.0; // -1 to 1
    let grain = noise * u_params.grain_strength;
    
    // Add grain (only to opaque areas if we want, or everywhere)
    if (color.a > 0.01) {
        color = vec4<f32>(color.rgb + grain, color.a);
    }
    
    return color;
}
