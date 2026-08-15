@group(0) @binding(0) var<storage, read> audio_samples: array<f32>; // 4096 samples
@group(0) @binding(1) var<storage, read> spectrum: array<f32, 64>; // 64 bins

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
    let uv = in.uv;
    
    // Background: Dark studio blueprint grid
    let bg_color = vec3<f32>(0.03, 0.04, 0.08);
    let grid_x = abs(fract(uv.x * 32.0 - 0.5) - 0.5) / fwidth(uv.x * 32.0);
    let grid_y = abs(fract(uv.y * 24.0 - 0.5) - 0.5) / fwidth(uv.y * 24.0);
    let grid = (1.0 - min(grid_x, 1.0)) * 0.15 + (1.0 - min(grid_y, 1.0)) * 0.15;
    var final_color = bg_color + vec3<f32>(0.0, 0.2, 0.4) * grid;

    // Divider between Oscilloscope (Top) and Equalizer Bars (Bottom) at y = 0.32
    if (abs(uv.y - 0.32) < 0.002) {
        return vec4<f32>(0.15, 0.3, 0.5, 1.0);
    }

    // -------------------------------------------------------------
    // TOP SECTION: Oscilloscope (Raw Audio Input Waveform)
    // -------------------------------------------------------------
    if (uv.y < 0.30) {
        // Map uv.x [0.05 .. 0.95] to audio sample indices [0 .. 1024]
        if (uv.x >= 0.04 && uv.x <= 0.96) {
            let sample_idx = u32(((uv.x - 0.04) / 0.92) * 1024.0);
            let sample_val = audio_samples[clamp(sample_idx, 0u, 4095u)];

            // Oscilloscope center y = 0.16, amplitude scale 0.10
            let wave_y = 0.16 - sample_val * 0.10;
            let dist_to_wave = abs(uv.y - wave_y);

            // Neon Cyan glowing oscilloscope line
            let line_intensity = exp(-dist_to_wave * 120.0);
            let glow_intensity = exp(-dist_to_wave * 25.0) * 0.4;

            let wave_color = vec3<f32>(0.0, 0.95, 1.0) * line_intensity + vec3<f32>(0.0, 0.5, 1.0) * glow_intensity;
            final_color = final_color + wave_color;
        }
        return vec4<f32>(final_color, 1.0);
    }

    // -------------------------------------------------------------
    // BOTTOM SECTION: 64 Equalizer Spectrum Bars (Audio FFT Energy)
    // -------------------------------------------------------------
    let eq_top = 0.36;
    let eq_bottom = 0.92;
    let eq_height = eq_bottom - eq_top;

    if (uv.y >= eq_top && uv.y <= eq_bottom + 0.02 && uv.x >= 0.04 && uv.x <= 0.96) {
        let norm_x = (uv.x - 0.04) / 0.92; // [0.0 .. 1.0] across 64 bars
        let bar_idx_f = norm_x * 64.0;
        let bar_idx = clamp(u32(bar_idx_f), 0u, 63u);
        let bar_fract_x = fract(bar_idx_f);

        // Bar width with gap (80% bar, 20% gap)
        if (bar_fract_x > 0.15 && bar_fract_x < 0.85) {
            let energy = spectrum[bar_idx]; // [0.0 .. 1.0]
            let bar_top_y = eq_bottom - energy * eq_height;

            if (uv.y >= bar_top_y && uv.y <= eq_bottom) {
                // Vertical gradient along the bar:
                // Base = Emerald Green, Middle = Electric Yellow, Top = Crimson Red
                let bar_norm_y = (eq_bottom - uv.y) / eq_height; // 0.0 at bottom, 1.0 at max
                
                var bar_color = vec3<f32>(0.0);
                if (bar_norm_y < 0.5) {
                    bar_color = mix(vec3<f32>(0.0, 0.9, 0.4), vec3<f32>(1.0, 0.85, 0.0), bar_norm_y / 0.5);
                } else {
                    bar_color = mix(vec3<f32>(1.0, 0.85, 0.0), vec3<f32>(1.0, 0.15, 0.2), (bar_norm_y - 0.5) / 0.5);
                }

                // Inner glow & highlight
                let edge_highlight = smoothstep(0.15, 0.3, bar_fract_x) * smoothstep(0.85, 0.7, bar_fract_x);
                final_color = bar_color * (0.8 + edge_highlight * 0.4);
            }

            // Peak hold cap marker (white glowing line above each bar)
            let peak_y = bar_top_y - 0.006;
            if (abs(uv.y - peak_y) < 0.003 && energy > 0.02) {
                final_color = vec3<f32>(1.0, 1.0, 1.0);
            }
        }
    }

    return vec4<f32>(final_color, 1.0);
}
