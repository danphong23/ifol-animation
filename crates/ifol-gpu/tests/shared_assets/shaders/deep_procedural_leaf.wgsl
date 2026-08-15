// Deep SubGraph DAG - Leaf Compute Shader
// Generates high-density procedural plasma texture in Level 4

@group(0) @binding(0) var output_tex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output_tex);
    if gid.x >= dims.x || gid.y >= dims.y {
        return;
    }
    
    let uv = vec2f(f32(gid.x) / f32(dims.x), f32(gid.y) / f32(dims.y));
    
    // Multi-octave procedural plasma
    let p = uv * 8.0;
    let v1 = sin(p.x + 1.5);
    let v2 = sin(p.y + 2.0);
    let v3 = sin(p.x + p.y + 0.5);
    let cx = p.x + 0.5 * sin(p.y / 5.0);
    let cy = p.y + 0.5 * cos(p.x / 3.0);
    let v4 = sin(sqrt(cx * cx + cy * cy + 1.0));
    
    let plasma = (v1 + v2 + v3 + v4) * 0.25;
    
    // Vibrant cyan-magenta palette
    let r = sin(plasma * 3.14159) * 0.5 + 0.5;
    let g = sin(plasma * 3.14159 + 2.094) * 0.5 + 0.5;
    let b = sin(plasma * 3.14159 + 4.188) * 0.5 + 0.5;
    
    textureStore(output_tex, vec2u(gid.xy), vec4f(r, g, b, 1.0));
}
