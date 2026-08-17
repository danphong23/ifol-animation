struct SkyUniform {
    top_color: vec3<f32>,
    noise_strength: f32,
    bottom_color: vec3<f32>,
    time: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_noise: texture_2d<f32>;
@group(0) @binding(1) var s_noise: sampler;
@group(1) @binding(0) var<uniform> sky: SkyUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0), vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0), vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0)
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0)
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[vi], 0.95, 1.0);
    out.uv = uv[vi];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let grad = mix(sky.top_color, sky.bottom_color, smoothstep(0.0, 1.0, in.uv.y));
    let dimensions = vec2<f32>(textureDimensions(t_noise));
    let noise_uv = fract(in.uv * 2.5);
    let noise_coord = vec2<i32>(noise_uv * dimensions);
    let noise_val = textureLoad(t_noise, noise_coord, 0).r;
    let final_sky = grad + (noise_val - 0.5) * sky.noise_strength;
    return vec4<f32>(clamp(final_sky, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
