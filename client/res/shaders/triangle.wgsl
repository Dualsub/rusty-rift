// Vertex shader

struct UniformBuffer {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
    camera_position: vec3<f32>,
    light_direction: vec3<f32>,
    light_color: vec4<f32>,
};


struct Instance {
    model_matrix: mat4x4<f32>,
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
    @location(1) normal: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> uniform_buffer: UniformBuffer;
@group(0) @binding(1)
var<storage, read> instance_buffer: array<Instance>;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = in.uvs;
    let mv = uniform_buffer.view_matrix * instance_buffer[in.instance_index].model_matrix;
    out.clip_position = uniform_buffer.projection_matrix * mv * vec4<f32>(in.position, 1.0);

    let mv3 = mat3x3<f32>(
        mv[0].xyz,
        mv[1].xyz,
        mv[2].xyz,
    );

    out.normal = normalize(mv3 * in.normal);
    return out;
}

// Fragment shader

@group(0) @binding(2)
var albedo_texture: texture_2d_array<f32>;
@group(0) @binding(3)
var albedo_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let albedo_color = textureSample(albedo_texture, albedo_sampler, in.tex_coords.xy, u32(in.tex_coords.z)).rgb;

    let visibility = 1.0;
    let light_color = uniform_buffer.light_color.rgb;
    // vector to point from the light source towards the fragment's position
    let light_dir = normalize(-uniform_buffer.light_direction);
    let view_dir = normalize(uniform_buffer.camera_position - in.clip_position.xyz);

    // diffuse shading
    let diff = max(dot(in.normal, light_dir), 0.6);
    let diffuse = diff * light_color.rgb;
    
    // specular shading
    let half_way = normalize(light_dir + view_dir);
    
    let spec = pow(max(dot(half_way, in.normal), 0.0), 16.0);
    let specular = spec * light_color * 2.0;
    
    let ambient = 0.2 * light_color;

    let color = vec3<f32>(ambient + visibility * (diff + specular) * albedo_color);
    let linear_color = pow(color, vec3<f32>(2.2));
    return vec4<f32>(linear_color, 1.0);
}
