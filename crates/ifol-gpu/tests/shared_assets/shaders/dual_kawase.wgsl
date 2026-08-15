struct KawaseUniform {
    offset: f32,
    intensity: f32,
    _pad0: f32,
    _pad1: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: KawaseUniform;

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

// 8-Tap Dual Kawase Filter (Ultra-fast 60FPS wide bloom)
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let half_pixel = vec2<f32>(0.00125, 0.00166) * u_params.offset; // 1/800, 1/600

    var sum = vec4<f32>(0.0);

    // Center sample
    sum += textureSample(t_diffuse, s_diffuse, in.uv) * 4.0;

    // 4 Diagonal samples
    sum += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(-half_pixel.x, -half_pixel.y)) * 2.0;
    sum += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>( half_pixel.x, -half_pixel.y)) * 2.0;
    sum += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(-half_pixel.x,  half_pixel.y)) * 2.0;
    sum += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>( half_pixel.x,  half_pixel.y)) * 2.0;

    // 4 Orthogonal samples
    sum += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(-half_pixel.x * 2.0, 0.0));
    sum += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>( half_pixel.x * 2.0, 0.0));
    sum += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, -half_pixel.y * 2.0));
    sum += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0,  half_pixel.y * 2.0));

    let blurred = (sum / 16.0) * u_params.intensity;
    return blurred;
}
