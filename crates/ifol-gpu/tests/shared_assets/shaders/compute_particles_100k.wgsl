struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    color: vec4<f32>,
    life: f32,
    max_life: f32,
    size: f32,
    pad: f32,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;

struct SimParams {
    delta_time: f32,
    attractor_count: u32,
    time: f32,
    damping: f32,
};

@group(0) @binding(1) var<uniform> params: SimParams;

@compute @workgroup_size(64, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let total_count = arrayLength(&particles);
    if (index >= total_count) {
        return;
    }

    var p = particles[index];

    // Gravitational Vortex Field
    let center = vec2<f32>(0.0, 0.0);
    let to_center = center - p.pos;
    let dist = max(length(to_center), 0.02);

    let dir = to_center / dist;
    let tangent = vec2<f32>(-dir.y, dir.x);

    // Gravity pull + Strong Tangential Spiral acceleration
    let gravity = dir * (0.8 / (dist * dist + 0.1));
    let vortex = tangent * (1.2 / (dist + 0.2));

    // Multi-frequency noise / galactic turbulence
    let angle_curr = atan2(p.pos.y, p.pos.x);
    let arm_modulation = sin(angle_curr * 3.0 - dist * 8.0 + params.time) * 0.4;
    let curl_noise = vec2<f32>(
        sin(p.pos.y * 6.0 + params.time),
        cos(p.pos.x * 6.0 + params.time)
    ) * (0.15 + arm_modulation * 0.2);

    let accel = gravity + vortex + curl_noise;

    p.vel = (p.vel + accel * params.delta_time) * params.damping;
    p.pos = p.pos + p.vel * params.delta_time;

    // Rich color based on speed and radius
    let speed = length(p.vel);
    let norm_r = clamp(dist / 0.85, 0.0, 1.0);

    // Color gradient: Core Gold -> Hot Magenta -> Neon Cyan -> Deep Sapphire
    var col = vec3<f32>(0.0);
    if (norm_r < 0.2) {
        col = mix(vec3<f32>(1.0, 0.9, 0.4), vec3<f32>(1.0, 0.3, 0.6), norm_r / 0.2);
    } else if (norm_r < 0.6) {
        col = mix(vec3<f32>(1.0, 0.3, 0.6), vec3<f32>(0.0, 0.8, 1.0), (norm_r - 0.2) / 0.4);
    } else {
        col = mix(vec3<f32>(0.0, 0.8, 1.0), vec3<f32>(0.1, 0.2, 0.8), (norm_r - 0.6) / 0.4);
    }

    col = col * (0.6 + speed * 0.4);
    p.color = vec4<f32>(col, 1.0);

    // Re-spawn particles that get too close to singularity or escape bounds
    if (dist < 0.04 || dist > 1.25) {
        let seed = f32(index) * 0.0137;
        let spawn_angle = seed * 6.2831853;
        let spawn_r = 0.25 + fract(sin(seed * 78.233) * 43758.5453) * 0.6;
        p.pos = vec2<f32>(cos(spawn_angle), sin(spawn_angle)) * spawn_r;
        let sp_tangent = vec2<f32>(-sin(spawn_angle), cos(spawn_angle));
        p.vel = sp_tangent * (0.5 + spawn_r * 0.5);
    }

    particles[index] = p;
}
