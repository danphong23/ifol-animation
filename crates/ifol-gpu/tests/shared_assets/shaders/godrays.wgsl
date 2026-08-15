struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct GodRaysUniform {
    light_pos: vec2<f32>,
    exposure: f32,
    decay: f32,
    density: f32,
    weight: f32,
};

@group(1) @binding(0) var<uniform> u_params: GodRaysUniform;

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
    var uv = in.uv;
    var delta_uv = uv - u_params.light_pos;
    delta_uv = delta_uv * (1.0 / f32(100)) * u_params.density;
    
    var color = textureSample(t_diffuse, s_diffuse, uv);
    var illumination_decay = 1.0;
    
    for (var i = 0; i < 100; i++) {
        uv -= delta_uv;
        var sample = textureSample(t_diffuse, s_diffuse, uv);
        sample *= illumination_decay * u_params.weight;
        color += sample;
        illumination_decay *= u_params.decay;
    }
    
    return color * u_params.exposure;
}
