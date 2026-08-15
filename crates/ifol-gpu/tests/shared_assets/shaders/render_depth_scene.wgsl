// Render 3D Geometry with Depth Writing (TC103)

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
    @location(1) depth_val: f32,
};

struct ObjectUniform {
    transform: mat4x4f,
    color: vec4f,
};

@group(0) @binding(0) var<uniform> obj: ObjectUniform;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Cube 36 vertices (or simple 3D triangles at various Z depths)
    // 3 overlapping geometric planes at z = 0.2, 0.5, 0.8
    var positions = array<vec3f, 18>(
        // Front Plane (Z = 0.2)
        vec3f(-0.5,  0.5, 0.2), vec3f(-0.5, -0.5, 0.2), vec3f( 0.2,  0.5, 0.2),
        vec3f( 0.2,  0.5, 0.2), vec3f(-0.5, -0.5, 0.2), vec3f( 0.2, -0.5, 0.2),
        // Middle Plane (Z = 0.5)
        vec3f(-0.2,  0.7, 0.5), vec3f(-0.2, -0.3, 0.5), vec3f( 0.6,  0.7, 0.5),
        vec3f( 0.6,  0.7, 0.5), vec3f(-0.2, -0.3, 0.5), vec3f( 0.6, -0.3, 0.5),
        // Back Plane (Z = 0.85)
        vec3f(-0.7,  0.3, 0.85), vec3f(-0.7, -0.7, 0.85), vec3f( 0.7,  0.3, 0.85),
        vec3f( 0.7,  0.3, 0.85), vec3f(-0.7, -0.7, 0.85), vec3f( 0.7, -0.7, 0.85)
    );
    
    let p = positions[in_vertex_index];
    out.position = vec4f(p.x, p.y, p.z, 1.0);
    out.color = vec4f(1.0 - p.z * 0.5, p.z * 0.8, 0.6, 1.0);
    out.depth_val = p.z;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}
