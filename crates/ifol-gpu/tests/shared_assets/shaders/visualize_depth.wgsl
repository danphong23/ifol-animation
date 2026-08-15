// Depth Map Visualization Post-Process Shader (TC103)
// Samples Depth Texture extracted via DMA TextureToTextureAspect and converts to crisp false-color depth ramp

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = array<vec2f, 4>(
        vec2f(-1.0,  1.0),
        vec2f(-1.0, -1.0),
        vec2f( 1.0,  1.0),
        vec2f( 1.0, -1.0)
    );
    out.position = vec4f(pos[in_vertex_index], 0.0, 1.0);
    out.uv = pos[in_vertex_index] * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

@group(0) @binding(0) var depth_tex: texture_depth_2d;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let dims = textureDimensions(depth_tex);
    let coords = vec2i(in.uv * vec2f(dims));
    let raw_depth = textureLoad(depth_tex, coords, 0);
    
    if raw_depth >= 0.999 {
        // Void Background (Z = 1.0): Dark Slate
        return vec4f(0.10, 0.12, 0.18, 1.0);
    }
    
    // Crisp Multi-Tier Depth False-Color:
    // Tier 1: Near (Z <= 0.35) -> Bright Golden Amber
    // Tier 2: Mid  (0.35 < Z <= 0.70) -> Emerald Mint Green
    // Tier 3: Far  (0.70 < Z < 1.00) -> Deep Royal Cobalt Blue
    if raw_depth <= 0.35 {
        return vec4f(1.0, 0.82, 0.10, 1.0);
    } else if raw_depth <= 0.70 {
        return vec4f(0.18, 0.82, 0.44, 1.0);
    } else {
        return vec4f(0.05, 0.45, 1.0, 1.0);
    }
}
