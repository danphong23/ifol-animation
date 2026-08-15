struct Node {
    pos: vec2<f32>,
    prev_pos: vec2<f32>,
};

@group(0) @binding(0) var<storage, read> nodes: array<Node>;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    let node = nodes[in_instance_index];
    let pos = node.pos; // Screen coords (0 to 800, 0 to 600)
    
    // Convert to clip space
    let clip_x = (pos.x / 800.0) * 2.0 - 1.0;
    let clip_y = 1.0 - (pos.y / 600.0) * 2.0;
    
    // Quad vertices around origin
    let size = 3.0; // Radius
    var quad_pos = vec2<f32>(0.0);
    var uv = vec2<f32>(0.0);
    switch in_vertex_index {
        case 0u: { quad_pos = vec2<f32>(-size, -size); uv = vec2<f32>(-1.0, -1.0); }
        case 1u: { quad_pos = vec2<f32>( size, -size); uv = vec2<f32>( 1.0, -1.0); }
        case 2u: { quad_pos = vec2<f32>(-size,  size); uv = vec2<f32>(-1.0,  1.0); }
        case 3u: { quad_pos = vec2<f32>( size,  size); uv = vec2<f32>( 1.0,  1.0); }
        case 4u: { quad_pos = vec2<f32>(-size,  size); uv = vec2<f32>(-1.0,  1.0); } // duplicate for triangle strip if needed, wait we use TriangleList usually, so 6 vertices
        case 5u: { quad_pos = vec2<f32>( size, -size); uv = vec2<f32>( 1.0, -1.0); }
        default: {}
    }
    
    // We'll actually just use triangle list: 0,1,2, 2,1,3
    // But since vertex_index is 0..5:
    let v_idx = in_vertex_index;
    if (v_idx == 0u) { quad_pos = vec2<f32>(-size, -size); uv = vec2<f32>(-1.0, -1.0); }
    if (v_idx == 1u) { quad_pos = vec2<f32>( size, -size); uv = vec2<f32>( 1.0, -1.0); }
    if (v_idx == 2u) { quad_pos = vec2<f32>(-size,  size); uv = vec2<f32>(-1.0,  1.0); }
    if (v_idx == 3u) { quad_pos = vec2<f32>(-size,  size); uv = vec2<f32>(-1.0,  1.0); }
    if (v_idx == 4u) { quad_pos = vec2<f32>( size, -size); uv = vec2<f32>( 1.0, -1.0); }
    if (v_idx == 5u) { quad_pos = vec2<f32>( size,  size); uv = vec2<f32>( 1.0,  1.0); }
    
    // Convert size to clip space too
    let clip_size_x = quad_pos.x / 800.0 * 2.0;
    let clip_size_y = quad_pos.y / 600.0 * 2.0;

    out.clip_pos = vec4<f32>(clip_x + clip_size_x, clip_y + clip_size_y, 0.0, 1.0);
    out.uv = uv;
    
    // Color gradient based on chain position
    let chain_idx = in_instance_index / 16u;
    let node_idx = f32(in_instance_index % 16u) / 15.0;
    
    // Mix from purple to cyan
    out.color = vec4<f32>(mix(vec3<f32>(0.8, 0.2, 1.0), vec3<f32>(0.2, 0.8, 1.0), node_idx), 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Circle SDF
    let dist = length(in.uv);
    if (dist > 1.0) {
        discard;
    }
    
    return in.color;
}
