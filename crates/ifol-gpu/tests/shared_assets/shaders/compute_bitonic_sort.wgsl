struct Particle {
    pos: vec2<f32>,
    depth: f32,
    _pad: f32,
    color: vec4<f32>,
}

struct SortParams {
    j: u32,
    k: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SortParams;

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let j = params.j;
    let k = params.k;
    
    let n = arrayLength(&particles);
    if i >= n { return; }

    let ixj = i ^ j;
    
    if ixj > i {
        let pi = particles[i];
        let pixj = particles[ixj];
        
        let dir = (i & k) == 0u;
        
        // Depth from large to small (1.0 is far, 0.0 is near).
        let p_i_depth = pi.depth;
        let p_ixj_depth = pixj.depth;
        
        var swap = false;
        if dir {
            swap = p_i_depth > p_ixj_depth;
        } else {
            swap = p_i_depth < p_ixj_depth;
        }
        
        if swap {
            particles[i] = pixj;
            particles[ixj] = pi;
        }
    }
}
