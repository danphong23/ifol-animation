struct ParticleSimUniform {
    time: f32,
    wind_speed: f32,
    gravity: f32,
    particle_count: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
};

@group(0) @binding(0) var t_snow: texture_2d<f32>;
@group(0) @binding(1) var s_snow: sampler;
@group(1) @binding(0) var<uniform> sim: ParticleSimUniform;

// Pseudo-random hash
fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453123);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32
) -> VertexOutput {
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

    let seed = f32(ii) * 17.317;
    let spawn_x = hash(seed) * 2.4 - 1.2;
    let spawn_y = hash(seed + 1.0) * 2.4 - 1.2;
    let speed = 0.25 + hash(seed + 2.0) * 0.45;
    let depth = 0.2 + hash(seed + 3.0) * 0.8; // 0.2 (distant/small) to 1.0 (foreground/large)

    // Physics Simulation: Gravity fall + sinusoidal wind turbulence
    let fall_distance = (sim.time * sim.gravity * speed);
    var pos_y = spawn_y - fall_distance;
    // Wrap around screen y [-1.2 .. 1.2]
    pos_y = ((pos_y + 1.2) % 2.4) - 1.2;

    let wind_drift = sin(sim.time * 1.5 + seed) * 0.12 * sim.wind_speed;
    var pos_x = spawn_x + wind_drift;
    // Wrap around screen x
    pos_x = ((pos_x + 1.2) % 2.4) - 1.2;

    // Rotation spin
    let rot_angle = sim.time * (hash(seed + 4.0) * 4.0 - 2.0);
    let c = cos(rot_angle);
    let s = sin(rot_angle);
    let rotated_quad = vec2<f32>(
        quad_pos[vi].x * c - quad_pos[vi].y * s,
        quad_pos[vi].x * s + quad_pos[vi].y * c
    );

    // Scale with 800/600 aspect ratio preservation
    let base_scale = (0.025 + 0.045 * depth);
    let scale = vec2<f32>(base_scale * (600.0 / 800.0), base_scale);

    var out: VertexOutput;
    out.clip_position = vec4<f32>(vec2<f32>(pos_x, pos_y) + rotated_quad * scale, 0.2 + 0.5 * (1.0 - depth), 1.0);
    out.uv = norm_uv[vi];
    out.alpha = 0.4 + 0.6 * depth;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_snow, s_snow, in.uv);

    // Green Chroma Key
    let key_color = vec3<f32>(0.0, 1.0, 0.0);
    let dist = distance(color.rgb, key_color);

    if (dist < 0.40) {
        discard;
    }

    let alpha = smoothstep(0.40, 0.52, dist) * in.alpha;

    // Pure white glowing snowflake with slight icy blue hue
    let snow_color = vec3<f32>(0.92, 0.96, 1.15);
    return vec4<f32>(snow_color, alpha);
}
