struct InstancedUniform {
    aspect_ratio: f32, // screen width / height
    time: f32,
    _pad: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> config: InstancedUniform;

// Pseudo-random function
fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VertexOutput {
    // Quad vertices
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0)
    );
    
    // UVs for the shield/sword crop (using generic hero spritesheet crop)
    // For this test, we use uv_min = (0.7, 0.4), uv_max = (0.9, 0.7) - assuming a shield or sword is there.
    let uv_min = vec2<f32>(0.28, 0.25);
    let uv_max = vec2<f32>(0.48, 0.45);
    
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(uv_min.x, uv_min.y),
        vec2<f32>(uv_min.x, uv_max.y),
        vec2<f32>(uv_max.x, uv_min.y),
        vec2<f32>(uv_max.x, uv_min.y),
        vec2<f32>(uv_min.x, uv_max.y),
        vec2<f32>(uv_max.x, uv_max.y)
    );
    
    let p = pos[vi];
    
    // Generate pseudo-random instance properties
    let i_f = f32(ii);
    let rand_x = hash(i_f * 1.1) * 2.0 - 1.0;
    let rand_y = hash(i_f * 2.3) * 2.0 - 1.0;
    let rand_scale = hash(i_f * 3.7) * 0.1 + 0.05;
    let rand_rot = hash(i_f * 4.1) * 6.28;
    
    // Base scale
    let s_x = rand_scale * (1.0 / config.aspect_ratio);
    let s_y = rand_scale;
    
    // Rotation
    let c = cos(rand_rot);
    let s = sin(rand_rot);
    let rotated_p = vec2<f32>(
        p.x * c - p.y * s,
        p.x * s + p.y * c
    );
    
    // Final position
    let final_pos = vec2<f32>(
        rotated_p.x * s_x + rand_x,
        rotated_p.y * s_y + rand_y
    );
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(final_pos, 0.0, 1.0);
    out.uv = uvs[vi];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex_color = textureSampleLevel(t_diffuse, s_diffuse, in.uv, 0.0);
    
    // Chroma key despill for green (0, 1, 0)
    let max_rb = max(tex_color.r, tex_color.b);
    if (tex_color.g > max_rb * 1.1) {
        tex_color.g = max_rb;
        tex_color.a = 0.0; // Hard discard green
    }
    
    if (tex_color.a < 0.1) {
        discard;
    }
    
    return tex_color;
}
