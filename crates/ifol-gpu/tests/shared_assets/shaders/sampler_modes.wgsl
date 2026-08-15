struct SpriteUniform {
    pos: vec2<f32>,
    scale: vec2<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    key_color: vec3<f32>,
    tolerance: f32,
    smoothness: f32,
    z_depth: f32,
    opacity: f32,
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> uniform_data: SpriteUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var quad_pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var norm_uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    var out: VertexOutput;
    let world_pos = uniform_data.pos + quad_pos[vi] * uniform_data.scale;
    out.clip_position = vec4<f32>(world_pos, uniform_data.z_depth, 1.0);
    out.uv = mix(uniform_data.uv_min, uniform_data.uv_max, norm_uv[vi]);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}
