// Vertex shader

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uvs: vec3<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let position = vec4<f32>(in.position, 1.0);

    var out: VertexOutput;
    out.tex_coords = in.uvs.xy;
    out.clip_position = position;

    return out;
}

// Fragment shader

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color =  textureSample(
        scene_texture,
        scene_sampler,
        in.tex_coords.xy
    );

    return vec4<f32>(color);
}