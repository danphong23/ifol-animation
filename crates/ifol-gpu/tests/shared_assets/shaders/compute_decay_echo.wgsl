// Compute Decay & Dispersion for Motion Echo (TC105)
// Applies temporal decay, radial blur & chromatic trail to feedback texture

@group(0) @binding(0) var src_feedback: texture_2d<f32>;
@group(0) @binding(1) var dst_feedback: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var feedback_sampler: sampler;

struct EchoParams {
    decay_rate: f32,
    dispersion: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(3) var<uniform> params: EchoParams;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(dst_feedback);
    if gid.x >= dims.x || gid.y >= dims.y {
        return;
    }
    
    let uv = vec2f(f32(gid.x) / f32(dims.x), f32(gid.y) / f32(dims.y));
    let center = vec2f(0.5);
    let dir = (uv - center) * params.dispersion;
    
    // Chromatic dispersion trail
    let r = textureSampleLevel(src_feedback, feedback_sampler, uv - dir * 1.5, 0.0).r;
    let g = textureSampleLevel(src_feedback, feedback_sampler, uv - dir, 0.0).g;
    let b = textureSampleLevel(src_feedback, feedback_sampler, uv, 0.0).b;
    let a = textureSampleLevel(src_feedback, feedback_sampler, uv, 0.0).a;
    
    let decayed_color = vec4f(r, g, b, a) * params.decay_rate;
    textureStore(dst_feedback, vec2u(gid.xy), decayed_color);
}
