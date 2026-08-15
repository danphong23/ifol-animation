struct Node {
    pos: vec2<f32>,
    prev_pos: vec2<f32>,
};

@group(0) @binding(0) var<storage, read_write> nodes: array<Node>;

const GRAVITY: vec2<f32> = vec2<f32>(0.0, 980.0);
const DT: f32 = 0.016;
const REST_LENGTH: f32 = 20.0;

// Pass 1: Integration
@compute @workgroup_size(64, 1, 1)
fn integrate_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= 4096u) { return; }

    var node = nodes[idx];
    let vel = node.pos - node.prev_pos;
    
    // Verlet integration
    let next_pos = node.pos + vel * 0.99 + GRAVITY * (DT * DT);
    
    node.prev_pos = node.pos;
    node.pos = next_pos;
    
    nodes[idx] = node;
}

struct Uniforms {
    time: f32,
};
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

// Pass 2: Constraints (1 thread per chain, 256 chains total)
@compute @workgroup_size(64, 1, 1)
fn constrain_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let chain_idx = global_id.x;
    if (chain_idx >= 256u) { return; }

    let base_idx = chain_idx * 16u;
    
    // 5 iterations for stability
    for (var iter = 0; iter < 5; iter++) {
        // Enforce anchor with sine wave oscillation
        let anchor_x = f32(chain_idx % 16u) * 50.0 + 25.0 + sin(uniforms.time * 5.0 + f32(chain_idx)) * 20.0;
        let anchor_y = f32(chain_idx / 16u) * 10.0 + 50.0; // Chains spaced out in a grid
        nodes[base_idx].pos = vec2<f32>(anchor_x, anchor_y);

        for (var i = 1u; i < 16u; i++) {
            let idx = base_idx + i;
            let prev_idx = base_idx + i - 1u;
            
            var p1 = nodes[prev_idx].pos;
            var p2 = nodes[idx].pos;
            
            let delta = p2 - p1;
            let dist = length(delta);
            
            if (dist > 0.001) {
                let diff = (dist - REST_LENGTH) / dist;
                let offset = delta * 0.5 * diff;
                
                // p1 is moved half, p2 is moved half. 
                // But p1 is only moved if it's not the anchor (i > 1).
                // Actually, sequential solving:
                if (i > 1u) {
                    nodes[prev_idx].pos = p1 + offset;
                }
                nodes[idx].pos = p2 - offset;
            }
        }
    }
}
