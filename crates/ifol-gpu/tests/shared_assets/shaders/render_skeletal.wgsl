struct Bone {
    transform: mat4x4<f32>,
}

struct BoneUniform {
    bones: array<mat4x4<f32>, 4>,
}

@group(0) @binding(0) var<uniform> bone_data: BoneUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) v_idx: u32,
) -> VertexOutput {
    // 4 body parts attached to 4 bones:
    // 0: Torso (Bone 0)
    // 1: Head (Bone 1)
    // 2: Arm (Bone 2)
    // 3: Leg (Bone 3)
    
    let part_idx = v_idx / 6u;
    let local_v = v_idx % 6u;
    
    var local_pos = vec2<f32>(0.0);
    if (local_v == 0u) { local_pos = vec2<f32>(-0.1, -0.15); }
    else if (local_v == 1u) { local_pos = vec2<f32>( 0.1, -0.15); }
    else if (local_v == 2u) { local_pos = vec2<f32>( 0.1,  0.15); }
    else if (local_v == 3u) { local_pos = vec2<f32>(-0.1, -0.15); }
    else if (local_v == 4u) { local_pos = vec2<f32>( 0.1,  0.15); }
    else if (local_v == 5u) { local_pos = vec2<f32>(-0.1,  0.15); }
    
    let bone_mat = bone_data.bones[part_idx];
    let world_pos = bone_mat * vec4<f32>(local_pos, 0.0, 1.0);
    
    var out: VertexOutput;
    out.position = world_pos;
    
    // Assign different colors to body parts
    if (part_idx == 0u) { out.color = vec3<f32>(0.2, 0.6, 0.9); }      // Torso - Blue
    else if (part_idx == 1u) { out.color = vec3<f32>(0.9, 0.8, 0.3); } // Head - Yellow
    else if (part_idx == 2u) { out.color = vec3<f32>(0.9, 0.3, 0.3); } // Arm - Red
    else { out.color = vec3<f32>(0.3, 0.8, 0.4); }                     // Leg - Green
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
