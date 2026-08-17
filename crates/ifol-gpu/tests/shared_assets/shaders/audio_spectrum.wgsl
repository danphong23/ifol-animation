struct AudioUniform {
    freqs: array<vec4<f32>, 4>, // 16 frequency bands packed
    base_color: vec4<f32>,
    time: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_noise: texture_2d<f32>;
@group(0) @binding(1) var s_noise: sampler;
@group(1) @binding(0) var<uniform> audio: AudioUniform;

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

fn get_freq(index: i32) -> f32 {
    let vec_idx = index / 4;
    let comp_idx = index % 4;
    let v = audio.freqs[vec_idx];
    if (comp_idx == 0) { return v.x; }
    if (comp_idx == 1) { return v.y; }
    if (comp_idx == 2) { return v.z; }
    return v.w;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    
    // Grid Background
    let grid_x = smoothstep(0.95, 1.0, fract(uv.x * 20.0));
    let grid_y = smoothstep(0.95, 1.0, fract(uv.y * 15.0));
    var final_color = vec3<f32>(grid_x + grid_y) * 0.1 * audio.base_color.rgb;

    // Spectrum Bars
    let num_bars: f32 = 16.0;
    let bar_idx_f = uv.x * num_bars;
    let bar_idx = i32(floor(bar_idx_f));
    let bar_uv_x = fract(bar_idx_f); // 0.0 to 1.0 inside a single bar

    if (bar_idx >= 0 && bar_idx < 16) {
        let freq_val = get_freq(bar_idx);
        
        // Bar thickness (gap between bars)
        let bar_width = smoothstep(0.0, 0.1, bar_uv_x) * smoothstep(1.0, 0.9, bar_uv_x);
        
        // Value intensity (y goes from 0 at top to 1 at bottom, we want origin at bottom)
        let normalized_y = 1.0 - uv.y;
        
        // Intensity curve (Neon Glow)
        let dist = max(0.0, freq_val - normalized_y);
        let glow = exp(-dist * 10.0) * freq_val; // Glow bleeding
        
        let bar_fill = step(normalized_y, freq_val);
        
        // Color mapping: Blue at bottom, Purple/Pink at top
        let bar_color = mix(audio.base_color.rgb, vec3<f32>(1.0, 0.2, 0.8), normalized_y);
        
        // Final combination: Solid fill + Soft glow
        let bright = (bar_fill + glow * 2.0) * bar_width;
        final_color = final_color + bar_color * bright;
        
        // Add peak line on top
        let peak = smoothstep(0.01, 0.0, abs(normalized_y - freq_val)) * bar_width;
        final_color = final_color + vec3<f32>(1.0, 1.0, 1.0) * peak;
    }

    // Add noise for a retro screen feel
    let noise = textureSampleLevel(t_noise, s_noise, uv + vec2<f32>(0.0, audio.time * 0.1), 0.0).r;
    final_color = final_color + (noise - 0.5) * 0.05;

    return vec4<f32>(final_color, 1.0);
}
