struct RimUniform {
    transform: mat4x4<f32>,
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    rim_color: vec3<f32>,
    rim_thickness: f32,
    shadow_offset: vec2<f32>,
    shadow_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) is_shadow: f32, // 1.0 for shadow pass, 0.0 for main pass
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(1) @binding(0) var<uniform> config: RimUniform;

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32 // 0 = Shadow, 1 = Main
) -> VertexOutput {
    var out: VertexOutput;
    
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
    
    var p = pos[vi];
    
    if (ii == 0u) {
        // Shadow pass
        p.x += config.shadow_offset.x;
        p.y += config.shadow_offset.y;
        out.is_shadow = 1.0;
    } else {
        // Main pass
        out.is_shadow = 0.0;
    }
    
    out.clip_position = config.transform * vec4<f32>(p, 0.0, 1.0);
    out.uv = mix(config.uv_min, config.uv_max, uv[vi]);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex_color = textureSampleLevel(t_diffuse, s_diffuse, in.uv, 0.0);
    
    // Chroma key despill for green (0, 1, 0)
    let max_rb = max(tex_color.r, tex_color.b);
    var alpha = tex_color.a;
    if (tex_color.g > max_rb * 1.1) {
        alpha = 0.0;
    }
    
    if (alpha < 0.1) {
        discard;
    }
    
    if (in.is_shadow > 0.5) {
        // Shadow pass: Just render the shadow color with the texture's alpha
        return vec4<f32>(config.shadow_color.rgb, config.shadow_color.a * alpha);
    }
    
    // Main pass: calculate rim light
    // Simple edge detection by sampling neighbours
    let tex_size = vec2<f32>(textureDimensions(t_diffuse, 0));
    let texel_size = 1.0 / tex_size;
    let offset = config.rim_thickness * texel_size;
    
    // Sample 4 directions to see if we are near the edge
    var edge_factor = 0.0;
    
    // Check left
    let c_left = textureSampleLevel(t_diffuse, s_diffuse, in.uv + vec2<f32>(-offset.x, 0.0), 0.0);
    if (c_left.g > max(c_left.r, c_left.b) * 1.1 || c_left.a < 0.1) { edge_factor += 0.25; }
    
    // Check right
    let c_right = textureSampleLevel(t_diffuse, s_diffuse, in.uv + vec2<f32>(offset.x, 0.0), 0.0);
    if (c_right.g > max(c_right.r, c_right.b) * 1.1 || c_right.a < 0.1) { edge_factor += 0.25; }
    
    // Check top
    let c_top = textureSampleLevel(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, -offset.y), 0.0);
    if (c_top.g > max(c_top.r, c_top.b) * 1.1 || c_top.a < 0.1) { edge_factor += 0.25; }
    
    // Check bottom
    let c_bottom = textureSampleLevel(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, offset.y), 0.0);
    if (c_bottom.g > max(c_bottom.r, c_bottom.b) * 1.1 || c_bottom.a < 0.1) { edge_factor += 0.25; }
    
    // Add rim light color on the edges
    let final_rgb = mix(tex_color.rgb, config.rim_color, edge_factor * 0.8);
    
    return vec4<f32>(final_rgb, alpha);
}
