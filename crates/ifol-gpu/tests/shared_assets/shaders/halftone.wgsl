struct HalftoneUniform {
    dot_size: f32, // scale of the halftone grid
    angle: f32, // rotation of the grid
    smoothness: f32, // AA for dots
    _pad0: f32,
    screen_width: f32,
    screen_height: f32,
    _pad1: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: HalftoneUniform;


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
    let uv = in.uv;
    let color = textureSampleLevel(t_diffuse, s_diffuse, uv, 0.0);
    if (color.a < 0.01) {
        return color; // Preserve transparent background
    }

    // Convert to grayscale (luminance)
    let lum = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    
    // Invert lum so dark is large dot, bright is small dot
    let radius = 1.0 - lum;
    
    // Map UV to pixel coordinates, then apply rotation
    let aspect = u_params.screen_width / u_params.screen_height;
    let s = sin(u_params.angle);
    let c = cos(u_params.angle);
    
    // Aspect corrected UV centered
    let uv_aspect = vec2<f32>((in.uv.x - 0.5) * aspect, in.uv.y - 0.5);
    
    // Rotate grid
    let rotated_uv = vec2<f32>(
        uv_aspect.x * c - uv_aspect.y * s,
        uv_aspect.x * s + uv_aspect.y * c
    );
    
    // Scale to get grid cells
    let scaled_uv = rotated_uv * (u_params.screen_height / u_params.dot_size);
    
    // Get local coordinate within the cell (-0.5 to 0.5)
    let local_uv = fract(scaled_uv) - vec2<f32>(0.5);
    
    // Distance from center of cell
    let dist = length(local_uv);
    
    // Calculate dot threshold (radius scaled to fit inside cell, max 0.5 * sqrt(2))
    // We multiply by 0.707 to make sure max black covers the cell
    let max_r = 0.707; 
    let threshold = radius * max_r;
    
    // Smoothstep for anti-aliased dots
    let dot_val = 1.0 - smoothstep(threshold - u_params.smoothness, threshold + u_params.smoothness, dist);
    
    // Final color: we can tint it or just output black dots
    // Let's do black dots on white paper, but multiply by original alpha
    let paper_color = vec3<f32>(1.0);
    let ink_color = vec3<f32>(0.1);
    
    let final_color = mix(paper_color, ink_color, dot_val);
    
    return vec4<f32>(final_color, color.a);
}
