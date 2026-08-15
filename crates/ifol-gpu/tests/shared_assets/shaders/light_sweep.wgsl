struct LightSweepUniform {
    progress: f32,
    angle: f32, // in radians
    width: f32,
    intensity: f32,
    color: vec3<f32>,
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: LightSweepUniform;

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
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    // Only apply sweep where alpha > 0 to preserve transparency
    if (tex_color.a < 0.01) {
        return tex_color;
    }
    
    // Center UVs for rotation
    let uv_centered = in.uv - vec2<f32>(0.5, 0.5);
    
    // Rotate coordinates
    let s = sin(u_params.angle);
    let c = cos(u_params.angle);
    let rot_x = uv_centered.x * c - uv_centered.y * s;
    
    // Calculate distance from the sweeping line
    // Sweep moves from -1.0 to 1.0
    let current_pos = (u_params.progress * 2.0) - 1.0;
    
    let dist = abs(rot_x - current_pos);
    
    // Smoothstep for soft edges
    let sweep_factor = 1.0 - smoothstep(0.0, u_params.width, dist);
    
    let sweep_color = u_params.color * sweep_factor * u_params.intensity;
    
    // Additive blending on top of original color
    let final_rgb = tex_color.rgb + sweep_color * tex_color.a;
    
    return vec4<f32>(final_rgb, tex_color.a);
}
