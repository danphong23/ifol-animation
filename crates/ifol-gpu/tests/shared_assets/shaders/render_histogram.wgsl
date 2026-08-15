@group(0) @binding(0) var background_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<storage, read> global_hist: array<u32, 256>;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((in_vertex_index << 1u) & 2u);
    let y = f32(in_vertex_index & 2u);
    out.clip_pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let bg_color = textureSampleLevel(background_tex, samp, in.uv, 0.0);
    
    // Histogram Overlay at bottom right: x in [0.5..0.95], y in [0.6..0.9]
    let hist_left = 0.5;
    let hist_right = 0.95;
    let hist_top = 0.6;
    let hist_bottom = 0.9;
    
    if (in.uv.x >= hist_left && in.uv.x <= hist_right && in.uv.y >= hist_top && in.uv.y <= hist_bottom) {
        // Find max histogram value (approximate max count for scaling)
        // Hardcoding a scale for typical 800x600 image (max bin around 10000)
        let max_count = 10000.0; 
        
        let norm_x = (in.uv.x - hist_left) / (hist_right - hist_left); // 0..1
        let bin_idx = u32(norm_x * 255.0);
        let count = f32(global_hist[bin_idx]);
        
        let hist_h = count / max_count;
        let bar_top_y = hist_bottom - hist_h * (hist_bottom - hist_top);
        
        // Draw background dark rect
        var color = bg_color.rgb * 0.3;
        
        // Draw histogram bar
        if (in.uv.y >= bar_top_y) {
            color = vec3<f32>(0.2, 0.8, 1.0); // Cyan bar
            
            // Highlight the bin based on luminance gradient
            color = mix(vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0), norm_x);
        }
        
        return vec4<f32>(color, 1.0);
    }

    return bg_color;
}
