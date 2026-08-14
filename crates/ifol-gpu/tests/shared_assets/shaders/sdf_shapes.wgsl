struct SdfShapeUniform {
    shape_type: f32, // 0.0: Circle, 1.0: RoundedRect, 2.0: Ring, 3.0: Triangle
    size_x: f32,
    size_y: f32,
    corner_radius: f32,
    color: vec4<f32>,
    border_color: vec4<f32>,
    border_width: f32,
    glow_strength: f32,
    pos_x: f32,
    pos_y: f32,
    rotation: f32,
    scale: f32,
    aspect_ratio: f32,
    _pad: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_uv: vec2<f32>, // -1.0 to 1.0
};

@group(1) @binding(0) var<uniform> shape: SdfShapeUniform;

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

    let base_pos = pos[vi];
    
    // Rotate
    let c = cos(shape.rotation);
    let s = sin(shape.rotation);
    let rot_x = base_pos.x * c - base_pos.y * s;
    let rot_y = base_pos.x * s + base_pos.y * c;

    // Apply scaling & aspect ratio correction
    // The quad itself spans from -1.0 to 1.0 in local space.
    let screen_x = shape.pos_x + rot_x * shape.scale * (1.0 / shape.aspect_ratio);
    let screen_y = shape.pos_y + rot_y * shape.scale;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(screen_x, screen_y, 0.0, 1.0);
    out.local_uv = base_pos; // Keep local -1.0 to 1.0 space for SDF math
    return out;
}

// ---- SDF Functions ----

fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sd_rounded_rect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let d = abs(p) - b + vec2<f32>(r);
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0))) - r;
}

fn sd_equilateral_triangle(p: vec2<f32>, r: f32) -> f32 {
    let k = sqrt(3.0);
    var p2 = p;
    p2.x = abs(p2.x) - r;
    p2.y = p2.y + r / k;
    if (p2.x + k * p2.y > 0.0) {
        p2 = vec2<f32>(p2.x - k * p2.y, -k * p2.x - p2.y) / 2.0;
    }
    p2.x -= clamp(p2.x, -2.0 * r, 0.0);
    return -length(p2) * sign(p2.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.local_uv;
    var d: f32 = 0.0;

    let type_id = i32(shape.shape_type + 0.5);

    if (type_id == 0) {
        // Circle
        d = sd_circle(p, shape.size_x);
    } else if (type_id == 1) {
        // Rounded Rect
        d = sd_rounded_rect(p, vec2<f32>(shape.size_x, shape.size_y), shape.corner_radius);
    } else if (type_id == 2) {
        // Ring (Circle outline with thickness)
        d = abs(sd_circle(p, shape.size_x)) - shape.border_width;
    } else if (type_id == 3) {
        // Triangle
        d = sd_equilateral_triangle(p, shape.size_x);
    }

    // Anti-Aliasing (Smoothstep)
    // Pixel derivatives to find edge width
    let aa = fwidth(d) * 0.7071; // roughly 1 pixel thickness

    // Inner Color (Fill)
    let fill_alpha = 1.0 - smoothstep(-aa, aa, d);
    var final_color = shape.color * fill_alpha;

    // Border Color (Stroke)
    if (shape.border_width > 0.0 && type_id != 2) {
        let border_dist = abs(d) - shape.border_width * 0.5;
        let border_alpha = 1.0 - smoothstep(-aa, aa, border_dist);
        // Blend stroke over fill
        final_color = mix(final_color, shape.border_color, border_alpha * shape.border_color.a);
    }

    // Outer Glow
    if (shape.glow_strength > 0.0) {
        let glow = exp(-max(0.0, d) * shape.glow_strength);
        final_color = final_color + shape.border_color * glow * 0.6;
    }

    if (final_color.a < 0.001) {
        discard;
    }

    return final_color;
}
