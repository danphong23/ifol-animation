struct PageCurlUniform {
    progress: f32,      // 0.0 to 1.0
    radius: f32,        // cylinder radius
    _pad0: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_scene_a: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;
@group(0) @binding(2) var t_scene_b: texture_2d<f32>;
@group(0) @binding(3) var s_sampler2: sampler;

@group(1) @binding(0) var<uniform> u_params: PageCurlUniform;

fn stabilize_color(color: vec4<f32>) -> vec4<f32> {
    let rgb = floor(clamp(color.rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * 256.0 + 0.5) / 256.0;
    return vec4<f32>(rgb, color.a);
}

fn load_scene_a(uv: vec2<f32>) -> vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(t_scene_a));
    let coordinate = vec2<i32>(clamp(uv * dimensions, vec2<f32>(0.0), dimensions - 1.0));
    return textureLoad(t_scene_a, coordinate, 0);
}

fn load_scene_b(uv: vec2<f32>) -> vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(t_scene_b));
    let coordinate = vec2<i32>(clamp(uv * dimensions, vec2<f32>(0.0), dimensions - 1.0));
    return textureLoad(t_scene_b, coordinate, 0);
}

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
fn fs_main(
    @builtin(position) screen_position: vec4<f32>,
    @location(0) _input_uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    let MIN_AMOUNT = -0.16;
    let MAX_AMOUNT = 1.3;
    let amount = u_params.progress * (MAX_AMOUNT - MIN_AMOUNT) + MIN_AMOUNT;
    let cylinderCenter = amount;
    
    let cylinderAngle = 2.0 * 3.14159; // actually we just need a math formula for curl
    // A simplified page curl math:
    let dimensions = vec2<f32>(textureDimensions(t_scene_a));
    let uv = (floor(screen_position.xy) + vec2<f32>(0.5)) / dimensions;
    let point_dist = uv.x - cylinderCenter;
    
    if (point_dist < -u_params.radius) {
        // Flat on the left, show scene B (next page)
        return stabilize_color(load_scene_b(uv));
    } else if (point_dist > u_params.radius) {
        // Flat on the right, show scene A (current page)
        return stabilize_color(load_scene_a(uv));
    } else {
        // Curling part (cylinder)
        // Polynomial approximation keeps the page fold deterministic across
        // shader math libraries while retaining the cylindrical silhouette.
        let normalized = clamp(point_dist / u_params.radius, -1.0, 1.0);
        let normalized_squared = normalized * normalized;
        let theta = normalized + normalized * normalized_squared *
            (0.16666667 + normalized_squared * (0.075 + normalized_squared * 0.04464286));
        
        // Calculate the UV coordinate of the folded page
        let fold_x = cylinderCenter - u_params.radius * theta;
        
        if (fold_x > 1.0 || fold_x < 0.0) {
            // Out of bounds, show background (B)
            return stabilize_color(load_scene_b(uv));
        }
        
        let curl_uv = vec2<f32>(fold_x, uv.y);
        let color_a = textureSampleLevel(t_scene_a, s_sampler, curl_uv, 0.0);
        
        // Add some shadow / highlight based on cylinder normal
        let theta_squared = theta * theta;
        let shadow = clamp(1.0 - theta_squared * 0.5 + theta_squared * theta_squared * 0.04166667, 0.0, 1.0);
        
        // Slightly darken the curled part to give 3D volume
        return stabilize_color(vec4<f32>(color_a.rgb * (0.6 + 0.4 * shadow), color_a.a));
    }
}
