struct ExposureUniform {
    zebra_threshold: f32, // e.g. 0.85
    zebra_speed: f32,
    time: f32,
    mode: f32, // 0 = Split Screen (Left: Zebra, Right: False Color), 1 = Full Zebra, 2 = Full False Color
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: ExposureUniform;

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

// Map IRE Luminance (0.0 to 1.0) to ARRI False Color Palette
fn ire_to_false_color(ire: f32) -> vec3<f32> {
    if (ire > 0.95) {
        return vec3<f32>(1.0, 0.0, 0.0); // Red: Overexposed / Clipping
    } else if (ire > 0.85) {
        return vec3<f32>(1.0, 0.9, 0.0); // Yellow: Near Clipping
    } else if (ire > 0.65) {
        return vec3<f32>(1.0, 0.4, 0.7); // Pink: Optimal Skin Tone (Caucasian / Asian)
    } else if (ire > 0.45) {
        return vec3<f32>(0.0, 1.0, 0.2); // Green: 18% Neutral Gray / Midtones
    } else if (ire > 0.25) {
        return vec3<f32>(0.2, 0.7, 1.0); // Light Blue: Shadow Transition
    } else if (ire > 0.10) {
        return vec3<f32>(0.0, 0.1, 0.8); // Deep Blue: Dark Shadows
    } else {
        return vec3<f32>(0.5, 0.0, 0.8); // Purple: Crushed Blacks (< 10 IRE)
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.uv);
    let ire = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));

    // 1. Calculate Zebra Pattern (45-degree moving stripes)
    let screen_pos = in.uv * vec2<f32>(800.0, 600.0);
    let diag = (screen_pos.x + screen_pos.y) + u_params.time * u_params.zebra_speed * 10.0;
    let stripe = step(8.0, diag % 16.0); // 8px black, 8px white

    var zebra_pixel = color.rgb;
    if (ire > u_params.zebra_threshold) {
        // Overlay zebra stripes on clipping highlights
        zebra_pixel = mix(vec3<f32>(0.1), vec3<f32>(1.0), stripe);
    }

    // 2. Calculate False Color Pixel
    let false_color_pixel = ire_to_false_color(ire);

    // 3. Render Mode (Default: Split Screen Left = Zebra, Right = False Color)
    var final_rgb = zebra_pixel;
    if (in.uv.x > 0.5) {
        final_rgb = false_color_pixel;
    }

    // White divider line at center
    let split_dist = abs(in.uv.x - 0.5);
    if (split_dist < 0.002) {
        final_rgb = vec3<f32>(1.0, 1.0, 1.0);
    }

    return vec4<f32>(final_rgb, color.a);
}
