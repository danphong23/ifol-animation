struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    // Calculate luminance
    let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    
    // Only keep bright pixels
    if (luminance > 0.7) {
        return color;
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
}
