// Memory Matrix Visualization Shader
// Renders memory allocation, pool reuse, and timeline state grid

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

struct MemoryStats {
    total_allocations: u32,
    reused_count: u32,
    in_flight_count: u32,
    frame_count: u32,
};

@group(0) @binding(0) var<uniform> stats: MemoryStats;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let uv = in.uv;
    
    // Background gradient
    var bg = vec3f(0.07, 0.08, 0.12);
    
    // Grid coordinates: 10 frames (X) x 8 slots (Y)
    let grid_x = floor(uv.x * 10.0);
    let grid_y = floor(uv.y * 8.0);
    let cell_uv = fract(vec2f(uv.x * 10.0, uv.y * 8.0));
    
    // Cell border
    let border = step(0.08, cell_uv.x) * step(0.08, cell_uv.y) * 
                 step(cell_uv.x, 0.92) * step(cell_uv.y, 0.92);
                 
    // Memory state simulation across 10 frames
    // Frame 0-2: New allocations (Gold/Amber)
    // Frame 3-5: In-flight / Tracking (Cyan/Blue)
    // Frame 6-9: Reused from pool (Neon Green)
    var cell_color = vec3f(0.15, 0.16, 0.22); // Empty/idle
    
    let slot_index = u32(grid_x * 8.0 + grid_y);
    if grid_x < f32(stats.frame_count) {
        if grid_y < 2.0 {
            // Reused allocations (Top rows)
            cell_color = vec3f(0.1, 0.85, 0.4); // Bright Green
        } else if grid_y < 5.0 {
            // In-flight tracked allocations (Middle rows)
            cell_color = vec3f(0.2, 0.55, 0.95); // Bright Blue
        } else if grid_y < 7.0 {
            // Fresh allocations (Bottom rows)
            cell_color = vec3f(0.95, 0.7, 0.1); // Amber / Gold
        }
    }
    
    // Add pulsing glow on active cells
    let cell_center = length(cell_uv - vec2f(0.5));
    let glow = smoothstep(0.5, 0.0, cell_center) * 0.2;
    cell_color += vec3f(glow);
    
    let final_color = mix(bg, cell_color, border);
    
    // Add title banner overlay at the top
    if uv.y < 0.04 {
        return vec4f(0.12, 0.14, 0.2, 1.0);
    }
    
    return vec4f(final_color, 1.0);
}
