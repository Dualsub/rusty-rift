// Vertex shader

struct UniformBuffer {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
    camera_position: vec3<f32>,
    light_matrix:mat4x4<f32>,
    light_direction: vec3<f32>,
    light_color: vec4<f32>,
};


struct Instance {
    model_matrix: mat4x4<f32>,
    color: vec4<f32>,
    tex_bounds: vec4<f32>,
    bone_offset: u32,
};

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uvs: vec3<f32>,
    @location(3) color: vec4<f32>,
    @location(4) bone_ids: vec4<i32>,
    @location(5) bone_weights: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniform_buffer: UniformBuffer;
@group(0) @binding(1) var<storage, read> instance_buffer: array<Instance>;
@group(0) @binding(2) var<storage, read> bone_buffer: array<mat4x4<f32>>;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let position = vec4<f32>(in.position, 1.0);
    let instance = instance_buffer[in.instance_index];

    var skinned_pos = vec4<f32>(0.0);
    for (var i = 0; i < 4; i++) {
        if (in.bone_ids[i] == -1) {
            continue;
        }

        skinned_pos += bone_buffer[instance.bone_offset + u32(in.bone_ids[i])] * position * in.bone_weights[i];
    }

    let mvp = uniform_buffer.light_matrix * instance_buffer[in.instance_index].model_matrix;
    out.clip_position = mvp * skinned_pos;
    return out;
}