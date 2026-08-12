struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Procedural random generator
fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co.xy ,vec2<f32>(12.9898,78.233))) * 43758.5453);
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VertexOutput {
    // Generate base quad
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0), vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0), vec2<f32>( 1.0, -1.0), vec2<f32>( 1.0,  1.0)
    );
    
    // Pseudo-random position based on instance_index
    let seed = vec2<f32>(f32(ii), f32(ii) * 1.5);
    let rand_x = rand(seed) * 2.0 - 1.0;
    let rand_y = rand(seed * 2.0) * 2.0 - 1.0;
    
    let rand_scale = 0.002 + rand(seed * 3.0) * 0.01; // smaller
    let rand_alpha = 0.1 + rand(seed * 4.0) * 0.9; // varying opacity
    
    let x_offset = pos[vi].x * rand_scale;
    let y_offset = pos[vi].y * rand_scale;
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(rand_x + x_offset, rand_y + y_offset, 0.5, 1.0);
    
    // Colorful particles for visibility
    let r = 0.5 + rand(seed * 5.0) * 0.5;
    let g = 0.5 + rand(seed * 6.0) * 0.5;
    let b = 0.5 + rand(seed * 7.0) * 0.5;
    
    out.color = vec4<f32>(r, g, b, rand_alpha);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
