// Video NV12 Bi-planar Color Conversion Shader (BT.709)
// Plane 0: Y channel (R8Unorm)
// Plane 1: UV interleaved channel (Rg8Unorm)

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = array<vec2f, 4>(
        vec2f(-1.0,  1.0),
        vec2f(-1.0, -1.0),
        vec2f( 1.0,  1.0),
        vec2f( 1.0, -1.0)
    );
    out.position = vec4f(pos[in_vertex_index], 0.0, 1.0);
    out.uv = pos[in_vertex_index] * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

struct VideoColorParams {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    gamma: f32,
};

@group(0) @binding(0) var y_texture: texture_2d<f32>;
@group(0) @binding(1) var uv_texture: texture_2d<f32>;
@group(0) @binding(2) var video_sampler: sampler;
@group(0) @binding(3) var<uniform> params: VideoColorParams;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let uv = in.uv;
    
    // Sample Y (Luma) and UV (Chroma)
    let y_val = textureSample(y_texture, video_sampler, uv).r;
    let uv_val = textureSample(uv_texture, video_sampler, uv).rg;
    
    let y = y_val;
    let u = uv_val.r - 0.5;
    let v = uv_val.g - 0.5;
    
    // BT.709 standard matrix conversion
    var r = y + 1.5748 * v;
    var g = y - 0.1873 * u - 0.4681 * v;
    var b = y + 1.8556 * u;
    
    var color = vec3f(r, g, b);
    
    // Contrast and Brightness
    color = (color - 0.5) * params.contrast + 0.5 + params.brightness;
    
    // Saturation
    let luma = dot(color, vec3f(0.2126, 0.7152, 0.0722));
    color = mix(vec3f(luma), color, params.saturation);
    
    // Gamma correction
    color = pow(max(color, vec3f(0.0)), vec3f(1.0 / params.gamma));
    
    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}
