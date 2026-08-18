struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    color: vec4<f32>,
}

struct GridCell {
    count: atomic<u32>,
    particles: array<u32, 32>,
}

@group(0) @binding(0) var<storage, read> source_particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> destination_particles: array<Particle>;
@group(0) @binding(2) var<storage, read_write> grid: array<GridCell>;

struct Params {
    num_particles: u32,
    grid_size: u32,     // e.g. 32
    cell_size: f32,     // e.g. 20.0
    radius: f32,        // e.g. 5.0
    dt: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
@group(0) @binding(3) var<uniform> params: Params;

fn quantize_state(value: f32) -> f32 {
    return round(value * 8.0) / 8.0;
}

// Pass 1: Reset grid counters
@compute @workgroup_size(64)
fn cs_reset_grid(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell_idx = global_id.x;
    if (cell_idx >= params.grid_size * params.grid_size) {
        return;
    }
    atomicStore(&grid[cell_idx].count, 0u);
}

// Pass 2: Hash particles into grid
@compute @workgroup_size(64)
fn cs_hash_particles(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let p_idx = global_id.x;
    if (p_idx >= params.num_particles) {
        return;
    }
    
    let p = source_particles[p_idx];

    // Compute a deterministic slot by counting lower particle indices in the
    // same cell. This avoids atomic insertion order changing the simulation.
    let cx = u32(clamp(p.pos.x / params.cell_size, 0.0, f32(params.grid_size - 1u)));
    let cy = u32(clamp(p.pos.y / params.cell_size, 0.0, f32(params.grid_size - 1u)));
    let cell_idx = cy * params.grid_size + cx;

    var slot = 0u;
    for (var previous = 0u; previous < p_idx; previous++) {
        let other = source_particles[previous];
        let other_cx = u32(clamp(other.pos.x / params.cell_size, 0.0, f32(params.grid_size - 1u)));
        let other_cy = u32(clamp(other.pos.y / params.cell_size, 0.0, f32(params.grid_size - 1u)));
        if (other_cy * params.grid_size + other_cx == cell_idx) {
            slot += 1u;
        }
    }
    atomicMax(&grid[cell_idx].count, slot + 1u);
    if (slot < 32u) {
        grid[cell_idx].particles[slot] = p_idx;
    }
}

// Pass 3: Simulate physics with neighbor checking
@compute @workgroup_size(64)
fn cs_simulate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let p_idx = global_id.x;
    if (p_idx >= params.num_particles) {
        return;
    }
    
    var p = source_particles[p_idx];
    
    let cx = i32(p.pos.x / params.cell_size);
    let cy = i32(p.pos.y / params.cell_size);
    
    var force = vec2<f32>(0.0, 0.0);
    
    // Check 3x3 neighborhood
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let nx = cx + dx;
            let ny = cy + dy;
            
            if (nx >= 0 && nx < i32(params.grid_size) && ny >= 0 && ny < i32(params.grid_size)) {
                let cell_idx = u32(ny) * params.grid_size + u32(nx);
                let count = min(atomicLoad(&grid[cell_idx].count), 32u);
                for (var i = 0u; i < count; i++) {
                    let other_idx = grid[cell_idx].particles[i];
                    if (other_idx != p_idx) {
                        let other = source_particles[other_idx];
                        let diff = p.pos - other.pos;
                        let dist2 = dot(diff, diff);
                        let min_dist = params.radius * 2.0;
                        if (dist2 > 0.0 && dist2 < min_dist * min_dist) {
                            let dist = sqrt(dist2);
                            let overlap = min_dist - dist;
                            force += (diff / dist) * overlap * 100.0;
                        }
                    }
                }
            }
        }
    }
    
    // Gravity to center
    let center = vec2<f32>(f32(params.grid_size) * params.cell_size * 0.5);
    let diff_center = center - p.pos;
    force += diff_center * 1.5; // pull to center
    
    // Integrate
    p.vel = (p.vel + force * params.dt) * 0.98; // apply damping
    p.pos += p.vel * params.dt;
    
    // Boundary check
    let max_pos = f32(params.grid_size) * params.cell_size;
    if (p.pos.x < params.radius) { p.pos.x = params.radius; p.vel.x *= -0.5; }
    if (p.pos.y < params.radius) { p.pos.y = params.radius; p.vel.y *= -0.5; }
    if (p.pos.x > max_pos - params.radius) { p.pos.x = max_pos - params.radius; p.vel.x *= -0.5; }
    if (p.pos.y > max_pos - params.radius) { p.pos.y = max_pos - params.radius; p.vel.y *= -0.5; }

    // Keep the simulation state on a canonical fixed grid. This prevents
    // backend-specific float rounding from amplifying over ten feedback steps.
    p.vel.x = quantize_state(p.vel.x);
    p.vel.y = quantize_state(p.vel.y);
    p.pos.x = quantize_state(p.pos.x);
    p.pos.y = quantize_state(p.pos.y);
    
    // Color based on velocity
    let speed = quantize_state(length(p.vel));
    p.color = vec4<f32>(0.2 + speed * 0.005, 0.5, 1.0 - speed * 0.005, 1.0);
    
    destination_particles[p_idx] = p;
}
