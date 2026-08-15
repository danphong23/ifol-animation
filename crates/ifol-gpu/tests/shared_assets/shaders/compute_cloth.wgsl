struct Particle {
    pos: vec4<f32>,
    old_pos: vec4<f32>,
}

struct Params {
    time: f32,
    delta_time: f32,
    grid_size: u32,
    pad: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: Params;

var<workgroup> shared_pos: array<vec4<f32>, 256>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(local_invocation_id) local_id: vec3<u32>) {
    let size = params.grid_size; // 16
    let index = local_id.y * size + local_id.x;
    
    var p = particles[index];
    
    // 1. Verlet Integration
    if (local_id.y == 0u) {
        // Top row is pinned, do not move. Move slightly with wind.
        p.pos.z = sin(params.time * 2.0 + f32(local_id.x) * 0.5) * 0.2;
    } else {
        let velocity = p.pos - p.old_pos;
        p.old_pos = p.pos;
        
        let gravity = vec4<f32>(0.0, -0.02, 0.0, 0.0) * params.delta_time * params.delta_time;
        // add wind
        let wind = vec4<f32>(0.0, 0.0, -0.01 * sin(params.time + p.pos.y), 0.0) * params.delta_time * params.delta_time;
        
        p.pos = p.pos + velocity * 0.99 + gravity + wind;
    }
    
    shared_pos[index] = p.pos;
    workgroupBarrier();
    
    // 2. Relaxation (8 iterations)
    let rest_dist = 1.8 / f32(size - 1u);
    
    for (var i = 0u; i < 8u; i++) {
        var my_pos = shared_pos[index];
        
        if (local_id.y > 0u) { // not pinned
            var force = vec4<f32>(0.0);
            var num_constraints = 0.0;
            
            // left
            if (local_id.x > 0u) {
                let n_pos = shared_pos[index - 1u];
                let diff = my_pos - n_pos;
                force -= normalize(diff) * (length(diff) - rest_dist) * 0.5;
                num_constraints += 1.0;
            }
            // right
            if (local_id.x < size - 1u) {
                let n_pos = shared_pos[index + 1u];
                let diff = my_pos - n_pos;
                force -= normalize(diff) * (length(diff) - rest_dist) * 0.5;
                num_constraints += 1.0;
            }
            // up
            if (local_id.y > 0u) {
                let n_pos = shared_pos[index - size];
                let diff = my_pos - n_pos;
                force -= normalize(diff) * (length(diff) - rest_dist) * 0.5;
                num_constraints += 1.0;
            }
            // down
            if (local_id.y < size - 1u) {
                let n_pos = shared_pos[index + size];
                let diff = my_pos - n_pos;
                force -= normalize(diff) * (length(diff) - rest_dist) * 0.5;
                num_constraints += 1.0;
            }
            
            if (num_constraints > 0.0) {
                my_pos += force / num_constraints;
            }
        }
        
        workgroupBarrier();
        shared_pos[index] = my_pos;
        workgroupBarrier();
    }
    
    p.pos = shared_pos[index];
    particles[index] = p;
}
