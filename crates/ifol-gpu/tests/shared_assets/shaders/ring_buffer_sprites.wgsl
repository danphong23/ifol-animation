// Uniform Ring Buffer Sprite Shader (TC98)
// Renders hundreds of dynamic orbiting particles using Dynamic Offsets from UniformRingBuffer

struct SpriteData {
    position: vec2f,
    scale: vec2f,
    color: vec4f,
    rotation: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> sprite: SpriteData;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Quad vertices (-0.5 to 0.5)
    var local_pos = array<vec2f, 4>(
        vec2f(-0.5,  0.5),
        vec2f(-0.5, -0.5),
        vec2f( 0.5,  0.5),
        vec2f( 0.5, -0.5)
    );
    
    let p = local_pos[in_vertex_index];
    
    // Rotate
    let cos_r = cos(sprite.rotation);
    let sin_r = sin(sprite.rotation);
    let rot_p = vec2f(
        p.x * cos_r - p.y * sin_r,
        p.x * sin_r + p.y * cos_r
    );
    
    // Scale and translate
    let world_pos = rot_p * sprite.scale + sprite.position;
    
    out.position = vec4f(world_pos, 0.0, 1.0);
    out.uv = p + vec2f(0.5);
    out.color = sprite.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // Soft circle glow
    let dist = length(in.uv - vec2f(0.5));
    let alpha = smoothstep(0.5, 0.1, dist);
    return vec4f(in.color.rgb, in.color.a * alpha);
}
