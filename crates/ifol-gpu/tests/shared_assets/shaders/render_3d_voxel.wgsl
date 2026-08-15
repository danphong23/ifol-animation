@group(0) @binding(0) var voxel_tex: texture_3d<f32>;
@group(0) @binding(1) var voxel_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );

    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = uv[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Front-to-back Raymarching through 3D Voxel Volume
    let ray_origin = vec3<f32>(in.uv.x, in.uv.y, 0.0);
    let ray_dir = vec3<f32>(0.0, 0.0, 1.0);

    var accumulated_color = vec4<f32>(0.0);
    let steps = 64;
    let step_size = 1.0 / f32(steps);

    for (var i = 0; i < steps; i++) {
        let p = ray_origin + ray_dir * (f32(i) * step_size);
        let sample_val = textureSampleLevel(voxel_tex, voxel_sampler, p, 0.0);

        let alpha = sample_val.a * 0.1;
        accumulated_color += vec4<f32>(sample_val.rgb * alpha, alpha);

        if (accumulated_color.a >= 0.98) {
            break;
        }
    }

    return vec4<f32>(accumulated_color.rgb, 1.0);
}
