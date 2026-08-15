struct Particle {
    pos: vec2<f32>,
    radius: f32,
    _pad: f32,
};

struct IndirectArgs {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_vertex: u32,
    first_instance: u32,
};

@group(0) @binding(0) var<storage, read> in_particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> out_particles: array<Particle>;
@group(0) @binding(2) var<storage, read_write> indirect: IndirectArgs;

struct Uniforms {
    cull_center: vec2<f32>,
    cull_radius: f32,
    _pad: f32,
};
@group(0) @binding(3) var<uniform> uniforms: Uniforms;

@compute @workgroup_size(64, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&in_particles)) {
        return;
    }

    let p = in_particles[idx];
    
    // Cull logic: keep if particle is INSIDE the cull circle (corrected for 800x600 aspect ratio)
    let aspect = 800.0 / 600.0;
    let diff = p.pos - uniforms.cull_center;
    let dist = length(vec2<f32>(diff.x * aspect, diff.y));
    if (dist <= uniforms.cull_radius) {
        // Atomic append directly into the indirect draw arguments
        let write_idx = atomicAdd(&indirect.instance_count, 1u);
        out_particles[write_idx] = p;
    }
}
