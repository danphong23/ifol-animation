struct PixelationUniform {
    block_size: f32, // Size of pixels in screen coords
    screen_width: f32,
    screen_height: f32,
    _pad0: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: PixelationUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[vi], 0.0, 1.0);
    out.uv = uv[vi];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (u_params.block_size <= 1.0) {
        return textureSample(t_diffuse, s_diffuse, in.uv);
    }
    
    // Convert UV to pixel coordinates
    let screen_pos = vec2<f32>(
        in.uv.x * u_params.screen_width,
        in.uv.y * u_params.screen_height
    );
    
    // Snap to grid
    let snapped_pos = floor(screen_pos / u_params.block_size) * u_params.block_size;
    
    // Add half block to sample from center of block
    let center_pos = snapped_pos + vec2<f32>(u_params.block_size * 0.5, u_params.block_size * 0.5);
    
    // Convert back to UV
    let pixelated_uv = vec2<f32>(
        center_pos.x / u_params.screen_width,
        center_pos.y / u_params.screen_height
    );
    
    return textureSample(t_diffuse, s_diffuse, pixelated_uv);
}
