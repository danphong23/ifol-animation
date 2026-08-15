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
    let MIN_AMOUNT = -0.16;
    let MAX_AMOUNT = 1.3;
    let amount = u_params.progress * (MAX_AMOUNT - MIN_AMOUNT) + MIN_AMOUNT;
    let cylinderCenter = amount;
    
    let cylinderAngle = 2.0 * 3.14159; // actually we just need a math formula for curl
    // A simplified page curl math:
    let uv = in.uv;
    let point_dist = uv.x - cylinderCenter;
    
    if (point_dist < -u_params.radius) {
        // Flat on the left, show scene B (next page)
        return textureSample(t_scene_b, s_sampler, uv);
    } else if (point_dist > u_params.radius) {
        // Flat on the right, show scene A (current page)
        return textureSample(t_scene_a, s_sampler, uv);
    } else {
        // Curling part (cylinder)
        let theta = asin(point_dist / u_params.radius);
        
        // Calculate the UV coordinate of the folded page
        let fold_x = cylinderCenter - u_params.radius * theta;
        
        if (fold_x > 1.0 || fold_x < 0.0) {
            // Out of bounds, show background (B)
            return textureSample(t_scene_b, s_sampler, uv);
        }
        
        let curl_uv = vec2<f32>(fold_x, uv.y);
        let color_a = textureSample(t_scene_a, s_sampler, curl_uv);
        
        // Add some shadow / highlight based on cylinder normal
        let shadow = cos(theta); // 1.0 at center, 0.0 at edges
        
        // Slightly darken the curled part to give 3D volume
        return vec4<f32>(color_a.rgb * (0.6 + 0.4 * shadow), color_a.a);
    }
}
