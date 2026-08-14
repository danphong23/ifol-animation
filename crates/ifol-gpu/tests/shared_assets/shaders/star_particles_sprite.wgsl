struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0) var t_props: texture_2d<f32>;
@group(0) @binding(1) var s_props: sampler;

fn hash(n: u32) -> f32 {
    var x = n * 1103515245u + 12345u;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return f32(x) / 4294967295.0;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VertexOutput {
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

    let seed = ii * 19u + 37u;
    let rand_x = hash(seed) * 1.94 - 0.97;
    let rand_y = hash(seed + 1u) * 1.6 - 0.65; // Spread gracefully across upper/mid sky

    var star_scale_y: f32;
    var brightness: f32;
    var uv_min = vec2<f32>(0.03, 0.03);
    var uv_max = vec2<f32>(0.12, 0.22);
    var tint = vec3<f32>(1.0, 1.0, 1.0);

    if (ii < 40u) {
        // Tier 0: Distant micro twinkling stars (tiny, soft)
        star_scale_y = 0.008 + hash(seed + 2u) * 0.008;
        brightness = 0.45 + hash(seed + 3u) * 0.4;
        uv_min = vec2<f32>(0.03, 0.28);
        uv_max = vec2<f32>(0.12, 0.45);
        tint = vec3<f32>(0.85, 0.95, 1.15);
    } else if (ii < 75u) {
        // Tier 1: Midground distinct sparkling stars
        star_scale_y = 0.016 + hash(seed + 2u) * 0.012;
        brightness = 0.75 + hash(seed + 3u) * 0.5;
        let star_type = (ii - 40u) % 3u;
        if (star_type == 0u) {
            uv_min = vec2<f32>(0.03, 0.03);
            uv_max = vec2<f32>(0.12, 0.22);
            tint = vec3<f32>(1.0, 1.0, 1.25);
        } else if (star_type == 1u) {
            uv_min = vec2<f32>(0.13, 0.03);
            uv_max = vec2<f32>(0.22, 0.22);
            tint = vec3<f32>(0.85, 1.15, 1.35);
        } else {
            uv_min = vec2<f32>(0.23, 0.03);
            uv_max = vec2<f32>(0.33, 0.22);
            tint = vec3<f32>(1.25, 1.15, 0.85);
        }
    } else {
        // Tier 2: Foreground bright glowing cross stars
        star_scale_y = 0.032 + hash(seed + 2u) * 0.022;
        brightness = 1.1 + hash(seed + 3u) * 0.6;
        uv_min = vec2<f32>(0.24, 0.30);
        uv_max = vec2<f32>(0.35, 0.48);
        tint = vec3<f32>(1.3, 1.4, 1.8);
    }

    let star_scale_x = star_scale_y * (600.0 / 800.0);

    var out: VertexOutput;
    let world_pos = vec2<f32>(rand_x, rand_y) + quad_pos[vi] * vec2<f32>(star_scale_x, star_scale_y);
    out.clip_position = vec4<f32>(world_pos, 0.5, 1.0);
    out.uv = mix(uv_min, uv_max, norm_uv[vi]);
    out.color = vec4<f32>(tint, brightness);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let raw_color = textureSample(t_props, s_props, in.uv);

    let key_color = vec3<f32>(0.0, 1.0, 0.0);
    let dist = distance(raw_color.rgb, key_color);

    if (dist < 0.45) {
        discard;
    }

    let alpha = smoothstep(0.45, 0.65, dist);
    let emissive = raw_color.rgb * in.color.rgb * in.color.a * alpha;
    return vec4<f32>(emissive, alpha);
}
