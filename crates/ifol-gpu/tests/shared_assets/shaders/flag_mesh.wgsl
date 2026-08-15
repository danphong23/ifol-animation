struct FlagUniform {
    time: f32,
    wave_freq: f32,
    wave_amp: f32,
    _pad: f32,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: FlagUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    // Flag pinned on the left (x = -0.7), waves increase towards the right
    let pin_factor = (model.position.x + 0.7) * 0.7;
    let wave1 = sin(model.position.x * u_params.wave_freq + u_params.time * 4.0);
    let wave2 = cos(model.position.y * u_params.wave_freq * 0.7 + u_params.time * 3.0);
    let displacement_z = (wave1 + wave2 * 0.5) * u_params.wave_amp * pin_factor;

    var displaced_pos = model.position;
    displaced_pos.z += displacement_z;
    displaced_pos.y += wave1 * 0.03 * pin_factor;

    // Approximate normal from wave derivative
    let dz_dx = cos(model.position.x * u_params.wave_freq + u_params.time * 4.0) * u_params.wave_freq * u_params.wave_amp * pin_factor;
    let normal = normalize(vec3<f32>(-dz_dx, 0.0, 1.0));

    // Map Z safely to [0.2, 0.8] so GPU near plane (Z < 0) NEVER clips the flag mesh!
    let safe_z = 0.5 + displacement_z * 0.5;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(displaced_pos.x * 1.2, displaced_pos.y * 1.4, safe_z, 1.0);
    out.uv = model.uv;
    out.normal = normal;
    out.world_pos = displaced_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);

    // Directional 3D Phong Lighting on Cloth
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 1.0));
    let view_dir = vec3<f32>(0.0, 0.0, 1.0);
    let half_dir = normalize(light_dir + view_dir);

    let diff = max(dot(in.normal, light_dir), 0.0);
    let spec = pow(max(dot(in.normal, half_dir), 0.0), 16.0);

    let ambient = 0.40;
    let lighting = ambient + diff * 0.60;
    let final_rgb = tex_color.rgb * lighting + vec3<f32>(spec * 0.35);

    return vec4<f32>(final_rgb, 1.0);
}
