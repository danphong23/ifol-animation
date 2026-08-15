struct IndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
};

struct Particle {
    pos: vec2<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<storage, read_write> indirect_args: IndirectArgs;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_particles = 1000u;
    
    if (idx >= total_particles) {
        return;
    }

    // Generate procedural spiral particle positions
    let f = f32(idx);
    let angle = f * 0.02;
    let radius = (f / f32(total_particles)) * 0.8;
    
    let x = cos(angle) * radius;
    let y = sin(angle) * radius;

    particles[idx].pos = vec2<f32>(x, y);
    particles[idx].color = vec4<f32>(
        sin(angle) * 0.5 + 0.5,
        cos(angle) * 0.5 + 0.5,
        1.0 - (radius / 0.8),
        1.0
    );

    // Only thread 0 writes the DrawIndirect arguments struct into GPU buffer!
    if (idx == 0u) {
        indirect_args.vertex_count = 6u; // Quad vertex count
        indirect_args.instance_count = total_particles; // 1,000 instances computed on GPU
        indirect_args.first_vertex = 0u;
        indirect_args.first_instance = 0u;
    }
}
