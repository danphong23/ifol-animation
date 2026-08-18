struct AspectFillUniform {
    target_aspect: f32, // Canvas width / height (e.g. 800/600 = 1.333 or 9/16 = 0.5625)
    source_aspect: f32, // Source image aspect
    blur_strength: f32, // Blur amount for background
    shadow_opacity: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: AspectFillUniform;

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
    // 1. Calculate Background Fill UV (Crop & Scale to fill whole screen)
    // Scale factor: fill means scale = max(target_aspect / source_aspect, 1.0)
    var bg_uv = in.uv;
    if (u_params.target_aspect > u_params.source_aspect) {
        let scale = u_params.source_aspect / u_params.target_aspect;
        bg_uv.y = (bg_uv.y - 0.5) * scale + 0.5;
    } else {
        let scale = u_params.target_aspect / u_params.source_aspect;
        bg_uv.x = (bg_uv.x - 0.5) * scale + 0.5;
    }

    // Apply Gaussian-like blur to background fill
    var bg_color = vec4<f32>(0.0);
    var total_weight = 0.0;
    let b_radius = u_params.blur_strength;
    for (var x = -3; x <= 3; x++) {
        for (var y = -3; y <= 3; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * b_radius * 0.005;
            let w = exp(-f32(x*x + y*y) / 8.0);
            bg_color += textureSampleLevel(t_diffuse, s_diffuse, clamp(bg_uv + offset, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0) * w;
            total_weight += w;
        }
    }
    bg_color = (bg_color / total_weight) * 0.65; // Dim background slightly

    // 2. Calculate Foreground Fit UV (Fit inside canvas maintaining exact aspect)
    // We scale down the foreground slightly (e.g. 0.85 of canvas) for border room
    let fit_scale = 0.85;
    var fg_uv = (in.uv - vec2<f32>(0.5)) / fit_scale;
    
    if (u_params.target_aspect > u_params.source_aspect) {
        let scale = u_params.target_aspect / u_params.source_aspect;
        fg_uv.x = fg_uv.x * scale;
    } else {
        let scale = u_params.source_aspect / u_params.target_aspect;
        fg_uv.y = fg_uv.y * scale;
    }
    fg_uv = fg_uv + vec2<f32>(0.5);

    // 3. Drop shadow for foreground card
    let shadow_offset = vec2<f32>(0.015, 0.02);
    let shadow_uv = fg_uv - shadow_offset;
    var shadow_alpha = 0.0;
    if (shadow_uv.x >= 0.0 && shadow_uv.x <= 1.0 && shadow_uv.y >= 0.0 && shadow_uv.y <= 1.0) {
        let s_dist_x = min(shadow_uv.x, 1.0 - shadow_uv.x);
        let s_dist_y = min(shadow_uv.y, 1.0 - shadow_uv.y);
        let s_edge = min(s_dist_x, s_dist_y);
        shadow_alpha = smoothstep(0.0, 0.04, s_edge) * u_params.shadow_opacity;
    }

    // Blend shadow on background
    var final_color = mix(bg_color.rgb, vec3<f32>(0.0), shadow_alpha);

    // 4. Foreground content
    if (fg_uv.x >= 0.0 && fg_uv.x <= 1.0 && fg_uv.y >= 0.0 && fg_uv.y <= 1.0) {
        let fg_color = textureSampleLevel(t_diffuse, s_diffuse, fg_uv, 0.0);
        // Thin white border around foreground
        let dist_x = min(fg_uv.x, 1.0 - fg_uv.x);
        let dist_y = min(fg_uv.y, 1.0 - fg_uv.y);
        let edge_dist = min(dist_x, dist_y);
        let border = 1.0 - smoothstep(0.003, 0.006, edge_dist);
        
        let fg_with_border = mix(fg_color.rgb, vec3<f32>(1.0), border * 0.8);
        final_color = mix(final_color, fg_with_border, fg_color.a);
    }

    return vec4<f32>(final_color, 1.0);
}
