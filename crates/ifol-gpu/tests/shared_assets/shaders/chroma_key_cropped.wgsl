struct SpriteUniform {
    pos: vec2<f32>,
    scale: vec2<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    key_color: vec3<f32>,
    tolerance: f32,
    smoothness: f32,
    z_depth: f32,
    opacity: f32,
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_texture: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;
@group(1) @binding(0) var<uniform> sprite: SpriteUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var quad_pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    var norm_uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    var out: VertexOutput;
    let world_pos = sprite.pos + quad_pos[vi] * sprite.scale;
    out.clip_position = vec4<f32>(world_pos, sprite.z_depth, 1.0);
    out.uv = mix(sprite.uv_min, sprite.uv_max, norm_uv[vi]);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_texture, s_sampler, in.uv);
    
    // Dynamic Chroma Key
    let dist = distance(color.rgb, sprite.key_color);
    let edge_low = sprite.tolerance - sprite.smoothness * 0.5;
    let edge_high = sprite.tolerance + sprite.smoothness * 0.5;

    if (dist < edge_low) {
        discard;
    }

    let alpha = smoothstep(edge_low, edge_high, dist) * sprite.opacity;

    // Green Despill Filter
    var base_color = color.rgb;
    let max_rb = max(base_color.r, base_color.b);
    if (base_color.g > max_rb) {
        base_color.g = max_rb;
    }

    return vec4<f32>(base_color, alpha);
}
