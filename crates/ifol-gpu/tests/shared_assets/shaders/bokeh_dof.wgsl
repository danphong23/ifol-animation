struct BokehUniform {
    focus_point: vec2<f32>, // Center of focus in screen coords
    focus_radius: f32,      // Sharp radius
    max_blur: f32,          // Maximum Bokeh disk size
    highlight_boost: f32,   // Bokeh blob intensity
    _pad0: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: BokehUniform;

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
    // Distance from focal center to calculate Circle of Confusion (CoC)
    let dist_from_focus = distance(in.uv, u_params.focus_point);
    let coc = clamp((dist_from_focus - u_params.focus_radius) / (1.0 - u_params.focus_radius), 0.0, 1.0);
    
    if (coc <= 0.01) {
        return textureSampleLevel(t_diffuse, s_diffuse, in.uv, 0.0);
    }

    let disk_radius = coc * u_params.max_blur * 0.012;
    var acc_color = vec4<f32>(0.0);
    var total_weight = 0.0;

    // Golden Angle Fermat spiral sampling for natural round Bokeh disks
    let golden_angle = 2.39996323; // Golden angle in radians

    for (var i = 0; i < 24; i++) {
        let theta = f32(i) * golden_angle;
        let r = sqrt(f32(i) / 24.0) * disk_radius;
        let sample_uv = in.uv + vec2<f32>(cos(theta), sin(theta)) * r;

        let sample_color = textureSampleLevel(t_diffuse, s_diffuse, sample_uv, 0.0);
        let lum = dot(sample_color.rgb, vec3<f32>(0.299, 0.587, 0.114));

        // Optical Bokeh weight: bright highlights expand with high weight
        let highlight_weight = 1.0 + pow(lum, 3.0) * u_params.highlight_boost;
        
        acc_color += sample_color * highlight_weight;
        total_weight += highlight_weight;
    }

    return acc_color / total_weight;
}
