// Deep SubGraph DAG - Intermediate Filter and Composite Shader
// Combines texture from child subgraph with chromatic distortion and vignette

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

@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let uv = in.uv;
    
    // Radial distortion & chromatic aberration
    let center = vec2f(0.5);
    let dist = distance(uv, center);
    let dir = normalize(uv - center + vec2f(0.0001));
    let aberration = dir * dist * 0.03;
    
    let r = textureSample(src_texture, src_sampler, uv + aberration).r;
    let g = textureSample(src_texture, src_sampler, uv).g;
    let b = textureSample(src_texture, src_sampler, uv - aberration).b;
    
    // Vignette
    let vignette = smoothstep(0.8, 0.2, dist);
    
    return vec4f(vec3f(r, g, b) * vignette, 1.0);
}
