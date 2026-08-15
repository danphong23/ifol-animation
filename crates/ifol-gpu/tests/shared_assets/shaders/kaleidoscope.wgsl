struct KaleidoscopeUniform {
    segments: f32, // number of slices
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

@group(1) @binding(0) var<uniform> u_params: KaleidoscopeUniform;

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
    if (u_params.segments <= 1.0) {
        return textureSample(t_diffuse, s_diffuse, in.uv);
    }
    
    // Shift UV so center is (0,0)
    let centered_uv = in.uv - vec2<f32>(0.5);
    
    // Convert to polar
    let radius = length(centered_uv);
    let angle = atan2(centered_uv.y, centered_uv.x);
    
    // Segment angle
    let segment_angle = (3.14159265359 * 2.0) / u_params.segments;
    
    // Wrap angle into segment
    // To make it reflect/mirror instead of repeat, we use abs(mod - half)
    // First normalize angle to 0..2PI
    var a = angle;
    if (a < 0.0) {
        a += 3.14159265359 * 2.0;
    }
    
    // Fold angle
    let folded_angle = abs((a % segment_angle) - (segment_angle / 2.0));
    
    // Convert back to cartesian
    let final_uv = vec2<f32>(
        cos(folded_angle) * radius,
        sin(folded_angle) * radius
    ) + vec2<f32>(0.5);
    
    return textureSample(t_diffuse, s_diffuse, final_uv);
}
