struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct ParticleUniforms {
    time: f32,
    screen_ratio: f32,
}
@group(0) @binding(0) var<uniform> uniforms: ParticleUniforms;

// Basic procedural hash function to generate random values from 0.0 to 1.0
fn hash11(p: f32) -> f32 {
    var p2 = fract(p * 0.1031);
    p2 *= p2 + 33.33;
    p2 *= p2 + p2;
    return fract(p2);
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32
) -> VertexOutput {
    // Generate quad vertices procedurally
    var pos = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0)
    );
    let vertex_pos = pos[in_vertex_index];

    // Compute random start position and speed for this particle (instance)
    let id_f = f32(in_instance_index);
    let start_x = hash11(id_f) * 2.0 - 1.0; // -1 to 1
    let speed = hash11(id_f + 123.4) * 0.5 + 0.1;
    let offset_time = hash11(id_f + 456.7) * 100.0;
    let sway = sin(uniforms.time * 2.0 + offset_time) * 0.05;

    // Calculate falling position
    // Modulo arithmetic to loop it from top to bottom
    let current_y = 1.0 - ((uniforms.time * speed + offset_time) % 2.0); 

    // Size of the particle
    let size = (hash11(id_f + 789.0) * 0.02 + 0.01);
    
    // Aspect ratio correction (assume width > height usually)
    var final_pos = vertex_pos * size;
    final_pos.y *= uniforms.screen_ratio;

    // Apply translation
    final_pos += vec2<f32>(start_x + sway, current_y);

    var out: VertexOutput;
    out.clip_position = vec4<f32>(final_pos, 0.5, 1.0);
    // standard quad UVs
    out.uv = pos[in_vertex_index] * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Make it a soft circle
    let dist = distance(in.uv, vec2<f32>(0.5, 0.5));
    if (dist > 0.5) {
        discard;
    }
    // Soft edge
    let alpha = 1.0 - smoothstep(0.3, 0.5, dist);
    return vec4<f32>(1.0, 1.0, 1.0, alpha * 0.8);
}
