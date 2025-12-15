// Vertex shader

struct Instance {
    position_and_scale: vec4<f32>,
    color: vec4<f32>,
    tex_bounds: vec4<f32>,
}

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
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0) var<storage, read> instance_buffer: array<Instance>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let position = vec4<f32>(in.position, 1.0);

    let instance = instance_buffer[in.instance_index];

    var out: VertexOutput;
    out.tex_coords = vec3<f32>(instance.tex_bounds.xy + in.uvs.xy * instance.tex_bounds.zw, in.uvs.z);
    out.clip_position = vec4<f32>(instance.position_and_scale.xy + position.xy * instance.position_and_scale.zw, 0.0, 1.0);
    out.color = in.color * instance.color;

    return out;
}

// Fragment shader

@group(1) @binding(0) var texture: texture_2d_array<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color =  textureSample(
        texture,
        texture_sampler,
        in.tex_coords.xy,
        i32(in.tex_coords.z)
    ) * in.color;

    return color;
}