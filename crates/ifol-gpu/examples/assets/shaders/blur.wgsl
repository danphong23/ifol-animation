struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0), vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0), vec2<f32>( 1.0, -1.0), vec2<f32>( 1.0,  1.0)
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[vi], 0.0, 1.0);
    out.uv = pos[vi] * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(t_diffuse));
    let texel = 1.0 / dim;
    
    var color = vec4<f32>(0.0);
    color += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(-2.0) * texel) * 0.0545;
    color += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(-1.0) * texel) * 0.2442;
    color += textureSample(t_diffuse, s_diffuse, in.uv) * 0.4026;
    color += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(1.0) * texel) * 0.2442;
    color += textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(2.0) * texel) * 0.0545;
    
    return color;
}
