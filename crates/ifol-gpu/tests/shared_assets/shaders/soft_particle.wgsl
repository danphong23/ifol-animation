struct SoftParticleUniform {
    pos: vec2<f32>,
    scale: vec2<f32>,
    particle_depth: f32,
    softness: f32,
    core_intensity: f32,
    _pad: f32,
    particle_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: SoftParticleUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var quad = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    let p = quad[vi] * u_params.scale + u_params.pos;
    var out: VertexOutput;
    out.clip_position = vec4<f32>(p, u_params.particle_depth, 1.0);
    out.uv = uvs[vi];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let centered = in.uv * 2.0 - vec2<f32>(1.0);
    let dist = length(centered);
    
    if (dist > 1.0) {
        return vec4<f32>(0.0);
    }

    // Volumetric sphere thickness profile
    let z = sqrt(1.0 - dist * dist);
    
    // Smooth radial energy aura
    let radial_fade = smoothstep(1.0, 0.0, dist);
    let core = pow(radial_fade, 2.5) * u_params.core_intensity;

    let final_rgb = u_params.particle_color.rgb * (1.0 + core * 2.0);
    let final_alpha = (radial_fade * 0.7 + core * 0.3) * u_params.particle_color.a;

    return vec4<f32>(final_rgb * final_alpha, final_alpha);
}
