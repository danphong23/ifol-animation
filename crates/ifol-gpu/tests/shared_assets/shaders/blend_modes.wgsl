struct BlendUniform {
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_base: texture_2d<f32>;
@group(0) @binding(1) var s_base: sampler;
@group(0) @binding(2) var t_blend: texture_2d<f32>;
@group(0) @binding(3) var s_blend: sampler;

@group(1) @binding(0) var<uniform> u_params: BlendUniform;

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

// 8 Blend Mode Functions
fn blend_multiply(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return base * blend;
}

fn blend_screen(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(1.0) - (vec3<f32>(1.0) - base) * (vec3<f32>(1.0) - blend);
}

fn blend_overlay_ch(b: f32, l: f32) -> f32 {
    if (b < 0.5) {
        return 2.0 * b * l;
    } else {
        return 1.0 - 2.0 * (1.0 - b) * (1.0 - l);
    }
}
fn blend_overlay(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        blend_overlay_ch(base.r, blend.r),
        blend_overlay_ch(base.g, blend.g),
        blend_overlay_ch(base.b, blend.b)
    );
}

fn blend_hard_light(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return blend_overlay(blend, base);
}

fn blend_soft_light_ch(b: f32, l: f32) -> f32 {
    if (l < 0.5) {
        return 2.0 * b * l + b * b * (1.0 - 2.0 * l);
    } else {
        return sqrt(b) * (2.0 * l - 1.0) + 2.0 * b * (1.0 - l);
    }
}
fn blend_soft_light(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        blend_soft_light_ch(base.r, blend.r),
        blend_soft_light_ch(base.g, blend.g),
        blend_soft_light_ch(base.b, blend.b)
    );
}

fn blend_color_dodge(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return clamp(base / (vec3<f32>(1.0) - blend + vec3<f32>(0.001)), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn blend_difference(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    return abs(base - blend);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(t_base, s_base, in.uv);

    // Each cell is 200x300. Center Paladin in each cell with exact natural aspect ratio
    let cell_uv = fract(in.uv * vec2<f32>(4.0, 2.0));
    let scale_y = 0.85;
    let hero_aspect = 0.2835; // Paladin width/height ratio
    let cell_aspect = 200.0 / 300.0;
    let scale_x = scale_y * (hero_aspect / cell_aspect) * 1.5; // Perfectly natural proportions

    let norm_x = (cell_uv.x - 0.5) / scale_x + 0.5;
    let norm_y = (cell_uv.y - 0.5) / scale_y + 0.5;

    var blend_color = vec4<f32>(0.0);
    if (norm_x >= 0.0 && norm_x <= 1.0 && norm_y >= 0.0 && norm_y <= 1.0) {
        let hero_uv = mix(vec2<f32>(0.005, 0.01), vec2<f32>(0.28, 0.98), vec2<f32>(norm_x, norm_y));
        let raw = textureSample(t_blend, s_blend, hero_uv);
        let dist = distance(raw.rgb, vec3<f32>(0.0, 1.0, 0.0));
        let a = smoothstep(0.45, 0.55, dist);
        blend_color = vec4<f32>(raw.rgb, a);
    }

    // Determine 4x2 Tile Index
    let col = i32(in.uv.x * 4.0);
    let row = i32(in.uv.y * 2.0);
    let tile_idx = row * 4 + col; // 0 to 7

    var blended_rgb = blend_color.rgb;

    if (tile_idx == 0) {
        // 0: Normal / Alpha Over
        blended_rgb = blend_color.rgb;
    } else if (tile_idx == 1) {
        // 1: Multiply
        blended_rgb = blend_multiply(base_color.rgb, blend_color.rgb);
    } else if (tile_idx == 2) {
        // 2: Screen
        blended_rgb = blend_screen(base_color.rgb, blend_color.rgb);
    } else if (tile_idx == 3) {
        // 3: Overlay
        blended_rgb = blend_overlay(base_color.rgb, blend_color.rgb);
    } else if (tile_idx == 4) {
        // 4: Hard Light
        blended_rgb = blend_hard_light(base_color.rgb, blend_color.rgb);
    } else if (tile_idx == 5) {
        // 5: Soft Light
        blended_rgb = blend_soft_light(base_color.rgb, blend_color.rgb);
    } else if (tile_idx == 6) {
        // 6: Color Dodge
        blended_rgb = blend_color_dodge(base_color.rgb, blend_color.rgb);
    } else {
        // 7: Difference
        blended_rgb = blend_difference(base_color.rgb, blend_color.rgb);
    }

    // Blend result over base according to blend layer alpha
    var final_rgb = mix(base_color.rgb, blended_rgb, blend_color.a * u_params.opacity);

    // Draw Grid Borders
    let grid_x = abs(fract(in.uv.x * 4.0) - 0.5);
    let grid_y = abs(fract(in.uv.y * 2.0) - 0.5);
    if (grid_x > 0.492 || grid_y > 0.485) {
        final_rgb = vec3<f32>(1.0, 1.0, 1.0); // White grid line
    }

    return vec4<f32>(final_rgb, 1.0);
}
