struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(0) @binding(2) var t_noise: texture_2d<f32>;
@group(0) @binding(3) var s_noise: sampler;

struct DissolveUniform {
    threshold: f32,
    edge_width: f32,
    edge_color: vec3<f32>,
};

@group(1) @binding(0) var<uniform> u_params: DissolveUniform;

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
    var color = textureSample(t_diffuse, s_diffuse, in.uv);
    let noise = textureSample(t_noise, s_noise, in.uv).r;
    
    if (noise < u_params.threshold) {
        discard;
    }
    
    if (noise < u_params.threshold + u_params.edge_width) {
        // Mix between edge color and original color
        let t = (noise - u_params.threshold) / u_params.edge_width; // 0 to 1
        // Create an intense glow effect near the edge
        let glow = u_params.edge_color * 2.0; 
        color = vec4<f32>(mix(glow, color.rgb, t), color.a);
    }
    
    return color;
}
