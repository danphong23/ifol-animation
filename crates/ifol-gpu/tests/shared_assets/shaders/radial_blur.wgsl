struct RadialBlurUniform {
    center: vec2<f32>,
    strength: f32, // blur amount
    samples: f32, // number of samples (cast to int in shader)
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: RadialBlurUniform;

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
    if (u_params.strength <= 0.0) {
        return textureSample(t_diffuse, s_diffuse, in.uv);
    }

    let dir = u_params.center - in.uv;
    let num_samples = i32(u_params.samples);
    
    var color = vec4<f32>(0.0);
    var total_weight = 0.0;
    
    for (var i = 0; i < num_samples; i = i + 1) {
        let percent = f32(i) / f32(num_samples);
        let weight = 1.0 - percent; // further samples have less weight
        let sample_uv = in.uv + dir * (percent * u_params.strength);
        
        color += textureSample(t_diffuse, s_diffuse, sample_uv) * weight;
        total_weight += weight;
    }
    
    return color / total_weight;
}
