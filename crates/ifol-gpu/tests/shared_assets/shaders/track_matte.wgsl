struct TrackMatteUniform {
    matte_type: f32, // 0 = Alpha Matte, 1 = Inverted Alpha Matte, 2 = Luma Matte, 3 = Inverted Luma Matte
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_base: texture_2d<f32>;
@group(0) @binding(1) var s_base: sampler;
@group(0) @binding(2) var t_matte: texture_2d<f32>;
@group(0) @binding(3) var s_matte: sampler;

@group(1) @binding(0) var<uniform> u_params: TrackMatteUniform;

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
    let base_color = textureSample(t_base, s_base, in.uv);
    let matte_color = textureSample(t_matte, s_matte, in.uv);
    
    // Calculate mask factor based on matte_type
    var mask: f32 = 1.0;
    let mode = i32(u_params.matte_type + 0.5);
    
    if (mode == 0) {
        // Alpha Matte
        mask = matte_color.a;
    } else if (mode == 1) {
        // Inverted Alpha Matte
        mask = 1.0 - matte_color.a;
    } else if (mode == 2) {
        // Luma Matte (Relative Luminance)
        let luma = dot(matte_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        mask = luma;
    } else {
        // Inverted Luma Matte
        let luma = dot(matte_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        mask = 1.0 - luma;
    }

    let final_alpha = base_color.a * mask * u_params.opacity;
    return vec4<f32>(base_color.rgb, final_alpha);
}
