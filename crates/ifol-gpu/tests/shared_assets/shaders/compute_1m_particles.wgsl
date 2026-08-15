struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= 1000000u) {
        return;
    }

    var p = particles[idx];
    
    // Euler Physics Integration
    let center = vec2<f32>(0.0, 0.0);
    let dir = p.pos - center;
    let dist = max(length(dir), 0.01);
    
    // Swirl force + Gravity pull
    let perp = vec2<f32>(-dir.y, dir.x);
    let force = (perp * 0.5 - dir * 0.2) / (dist * dist + 0.1);
    
    p.vel += force * 0.001;
    p.pos += p.vel * 0.016;

    // Bounds Bounce
    if (abs(p.pos.x) > 0.95) {
        p.vel.x = -p.vel.x * 0.8;
    }
    if (abs(p.pos.y) > 0.95) {
        p.vel.y = -p.vel.y * 0.8;
    }

    particles[idx] = p;
}
