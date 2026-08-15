// Render Mesh Wave Shader (TC102)
// Reads vertex attributes from Storage Buffer copied via DMA CopyCommand

struct Vertex {
    pos: vec4f,
    color: vec4f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec4f,
    @location(1) world_pos: vec3f,
};

@group(0) @binding(0) var<storage, read> vertices: array<Vertex>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    let vert_idx = indices[in_vertex_index];
    let vert = vertices[vert_idx];
    
    // Isometric 3D projection
    let x = vert.pos.x;
    let y = vert.pos.y;
    let z = vert.pos.z;
    
    let screen_x = (x - z) * 0.7071;
    let screen_y = (x + z) * 0.4082 + y * 0.8165;
    
    out.clip_position = vec4f(screen_x * 0.9, screen_y * 0.9, 0.0, 1.0);
    out.color = vert.color;
    out.world_pos = vert.pos.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let sheen = pow(in.color.r, 2.0) * 0.3;
    let final_rgb = in.color.rgb + vec3f(sheen);
    return vec4f(final_rgb, 1.0);
}
