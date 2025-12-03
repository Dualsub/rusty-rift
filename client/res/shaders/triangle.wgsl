// Vertex shader

struct UniformBuffer {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
};

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uvs: vec3<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> uniform_buffer: UniformBuffer;
@group(0) @binding(1)
var<storage, read> instance_buffer: array<mat4x4<f32>>;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = in.uvs;
    out.clip_position = uniform_buffer.projection_matrix * uniform_buffer.view_matrix * instance_buffer[in.instance_index] * vec4<f32>(in.position, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(2)
var t_diffuse: texture_2d_array<f32>;
@group(0) @binding(3)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords.xy, u32(in.tex_coords.z));
}
