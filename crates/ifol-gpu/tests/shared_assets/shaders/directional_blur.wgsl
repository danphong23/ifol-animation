struct DirBlurUniform {
    angle: f32, // in radians
    strength: f32, // blur amount
    samples: f32, // number of samples (cast to int in shader)
    _pad0: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: DirBlurUniform;

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

    let dir = vec2<f32>(cos(u_params.angle), sin(u_params.angle));
    let num_samples = i32(u_params.samples);
    let step_size = u_params.strength / f32(num_samples);
    
    var color = vec4<f32>(0.0);
    var total_weight = 0.0;
    
    // Simple box blur along the direction vector
    // We sample on both sides (-num_samples/2 to +num_samples/2)
    let half_samples = num_samples / 2;
    for (var i = -half_samples; i <= half_samples; i = i + 1) {
        let offset = dir * (f32(i) * step_size);
        let sample_uv = in.uv + offset;
        
        // Accumulate (simple box filter)
        color += textureSample(t_diffuse, s_diffuse, sample_uv);
        total_weight += 1.0;
    }
    
    return color / total_weight;
}
