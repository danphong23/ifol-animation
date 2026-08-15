struct CAUniform {
    center: vec2<f32>,
    amount: f32, // strength of separation
    _pad0: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: CAUniform;

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
    if (u_params.amount <= 0.0) {
        return textureSample(t_diffuse, s_diffuse, in.uv);
    }
    
    // Vector from center to current pixel
    let dir = in.uv - u_params.center;
    
    // Distance from center determines how strong the split is
    let dist = length(dir);
    let shift = dir * u_params.amount * dist;
    
    // Sample RGB independently at slightly different UVs
    let colorR = textureSample(t_diffuse, s_diffuse, in.uv - shift);
    let colorG = textureSample(t_diffuse, s_diffuse, in.uv);
    let colorB = textureSample(t_diffuse, s_diffuse, in.uv + shift);
    
    // Alpha should probably just be the alpha from G (center)
    // or max of all three to avoid weird cutoffs
    let alpha = max(colorR.a, max(colorG.a, colorB.a));
    
    return vec4<f32>(colorR.r, colorG.g, colorB.b, alpha);
}
