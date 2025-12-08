// Vertex shader

struct UniformBuffer {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
    camera_position: vec3<f32>,
    light_matrix: mat4x4<f32>,
    light_direction: vec3<f32>,
    light_color: vec4<f32>,
};

struct Instance {
    model_matrix: mat4x4<f32>,
    color: vec4<f32>,
    tex_bounds: vec4<f32>,
}

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
    @location(0) tex_coords: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light_space_position: vec3<f32>,
    @location(4) world_position: vec3<f32>,
};

@group(0) @binding(0) var<uniform> uniform_buffer: UniformBuffer;
@group(0) @binding(1) var<storage, read> instance_buffer: array<Instance>;

fn get_bone_debug_color(bone_id : i32) -> vec3<f32> {
    let n = u32(bone_id) * 1664525u + 1013904223u;
    let r = f32((n >>  0u) & 255u) / 255.0;
    let g = f32((n >>  8u) & 255u) / 255.0;
    let b = f32((n >> 16u) & 255u) / 255.0;

    return vec3<f32>(r, g, b);
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let position = vec4<f32>(in.position, 1.0);

    let instance = instance_buffer[in.instance_index];

    let model = instance.model_matrix;

    // World-space position
    let world_pos = model * position;

    // View-space position
    let view_pos = uniform_buffer.view_matrix * world_pos;

    var bone_debug_color = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0; i < 4; i++) {
        let id = in.bone_ids[i];
        let weight = in.bone_weights[i];
        bone_debug_color = bone_debug_color + get_bone_debug_color(id) * weight;
    }

    var out: VertexOutput;
    out.tex_coords = vec3<f32>(instance.tex_bounds.xy + in.uvs.xy * instance.tex_bounds.zw, in.uvs.z);
    out.clip_position = uniform_buffer.projection_matrix * view_pos;
    // out.color = in.color * instance.color;
    out.color = vec4<f32>(bone_debug_color, 1.0);

    // World-space normal (ignoring non-uniform scale issues for now)
    let model3 = mat3x3<f32>(
        model[0].xyz,
        model[1].xyz,
        model[2].xyz,
    );
    out.world_normal = normalize(model3 * in.normal);
    out.world_position = world_pos.xyz;

    // light-space position (shadow map coords) – unchanged
    let pos_from_light = uniform_buffer.light_matrix * world_pos;
    let ndc = pos_from_light.xyz / pos_from_light.w;
    out.light_space_position = vec3f(
        ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5, 0.5),
        ndc.z
    );

    return out;
}