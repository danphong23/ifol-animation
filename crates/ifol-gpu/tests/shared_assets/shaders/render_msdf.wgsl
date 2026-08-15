struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var msdf_texture: texture_2d<f32>;
@group(0) @binding(1) var msdf_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = vec2<f32>(0.0);
    var uv = vec2<f32>(0.0);
    if (vertex_index == 0u) { pos = vec2<f32>(-1.0, -1.0); uv = vec2<f32>(0.0, 1.0); }
    else if (vertex_index == 1u) { pos = vec2<f32>( 3.0, -1.0); uv = vec2<f32>(2.0, 1.0); }
    else if (vertex_index == 2u) { pos = vec2<f32>(-1.0,  3.0); uv = vec2<f32>(0.0, -1.0); }
    
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn median(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Zoom in to see the anti-aliased edge
    let uv = (in.uv - 0.5) * 0.8 + 0.5;
    
    let sample = textureSample(msdf_texture, msdf_sampler, uv);
    
    // MSDF logic: median of RGB channels minus 0.5 threshold
    let sig_dist = median(sample.r, sample.g, sample.b) - 0.5;
    
    // Pixel-perfect anti-aliasing using fwidth
    let w = fwidth(sig_dist);
    let opacity = smoothstep(-w, w, sig_dist);
    
    // Draw an outline using distance field
    let outline_width = 0.05;
    let outline_w = fwidth(sig_dist - outline_width);
    let outline_opacity = smoothstep(-outline_w, outline_w, sig_dist + outline_width) - opacity;
    
    let bg_color = vec3<f32>(0.1, 0.1, 0.12);
    let text_color = vec3<f32>(1.0, 0.9, 0.2);
    let outline_color = vec3<f32>(0.9, 0.1, 0.3);
    
    var final_color = bg_color;
    final_color = mix(final_color, outline_color, outline_opacity);
    final_color = mix(final_color, text_color, opacity);
    
    return vec4<f32>(final_color, 1.0);
}
