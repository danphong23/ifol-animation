struct EchoUniform {
    velocity: vec2<f32>, // Direction & distance of trail
    decay: f32,          // Opacity decay per step (e.g. 0.65)
    hue_shift: f32,      // Color rotation per ghost step
    num_echoes: f32,     // Number of ghost trails (e.g. 5)
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@group(1) @binding(0) var<uniform> u_params: EchoUniform;

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
    var final_color = vec4<f32>(0.0);
    let count = i32(u_params.num_echoes);

    // Composite ghost trails from furthest (oldest) to closest (newest / main sprite)
    for (var i = count; i >= 0; i--) {
        let t = f32(i);
        let offset = u_params.velocity * t;
        let sample_uv = in.uv + offset;
        
        let sprite = textureSample(t_diffuse, s_diffuse, sample_uv);
        
        if (sprite.a > 0.01) {
            let alpha_factor = pow(u_params.decay, t);
            
            // Color tint shifting along the trail
            var tint = vec3<f32>(1.0);
            if (i > 0) {
                // Spectral trail: cyan to purple to blue
                let phase = t * u_params.hue_shift;
                tint = vec3<f32>(0.5 + 0.5 * cos(phase), 0.5 + 0.5 * cos(phase + 2.094), 0.5 + 0.5 * cos(phase + 4.188));
                tint = mix(vec3<f32>(1.0), tint, 0.75);
            }
            
            let layer_color = vec4<f32>(sprite.rgb * tint, sprite.a * alpha_factor);
            
            // Alpha OVER blending: C_out = C_layer + C_final * (1 - A_layer)
            final_color = vec4<f32>(
                layer_color.rgb + final_color.rgb * (1.0 - layer_color.a),
                layer_color.a + final_color.a * (1.0 - layer_color.a)
            );
        }
    }

    return final_color;
}
