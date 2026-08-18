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

@group(0) @binding(0) var<storage, read> source_particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> destination_particles: array<Particle>;
@group(0) @binding(2) var<uniform> params: SortParams;

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let j = params.j;
    let k = params.k;
    
    let n = arrayLength(&source_particles);
    if i >= n { return; }

    let ixj = i ^ j;
    if ixj >= n { return; }

    let pi = source_particles[i];
    let pixj = source_particles[ixj];
    var lower = pi;
    var higher = pixj;
    if (pi.depth > pixj.depth) {
        lower = pixj;
        higher = pi;
    }

    let ascending = (i & k) == 0u;
    let lower_index = i < ixj;
    if (ascending == lower_index) {
        destination_particles[i] = lower;
    } else {
        destination_particles[i] = higher;
    }
}
