struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct RippleUniform {
    center: vec2<f32>,
    time: f32,
    amplitude: f32,
    frequency: f32,
    speed: f32,
    _pad: vec2<f32>,
};

@group(1) @binding(0) var<uniform> u_params: RippleUniform;

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
    
    // Convert to aspect-corrected coords for distance if aspect is known, 
    // but assuming 1:1 or basic distortion for now
    let dist = distance(uv, u_params.center);
    
    let wave = sin(dist * u_params.frequency - u_params.time * u_params.speed);
    
    // Direction vector
    var dir = normalize(uv - u_params.center);
    if (dist == 0.0) {
        dir = vec2<f32>(0.0, 0.0);
    }
    
    // Dampen amplitude over distance
    let damp = max(0.0, 1.0 - dist * 2.0); 
    
    let offset = dir * wave * u_params.amplitude * damp;
    
    let final_uv = uv + offset;
    return textureSample(t_diffuse, s_diffuse, final_uv);
}
