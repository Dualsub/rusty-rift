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
    @location(2) color: vec4<f32>,
    @location(3) light_space_position: vec3<f32>,
};

@group(0) @binding(0) var<uniform> uniform_buffer: UniformBuffer;
@group(0) @binding(1) var<storage, read> instance_buffer: array<Instance>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let position = vec4<f32>(in.position, 1.0);

    let model = instance_buffer[in.instance_index].model_matrix;
    let mv = uniform_buffer.view_matrix * model;

    var out: VertexOutput;
    out.tex_coords = in.uvs;
    out.clip_position = uniform_buffer.projection_matrix * mv * position;
    out.color = in.color;

    // normal in view space
    let mv3 = mat3x3<f32>(
        mv[0].xyz,
        mv[1].xyz,
        mv[2].xyz,
    );
    out.normal = normalize(mv3 * in.normal);

    // light-space position (shadow map coords)
    let pos_from_light = uniform_buffer.light_matrix * model * position;
    let ndc = pos_from_light.xyz / pos_from_light.w;
    out.light_space_position = vec3f(
        ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5, 0.5),         
        ndc.z                                    
    );

    return out;
}

// Fragment shader
@group(0) @binding(2) var shadow_map: texture_depth_2d;
@group(0) @binding(3) var shadow_sampler: sampler_comparison;

@group(1) @binding(0) var albedo_texture: texture_2d_array<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // PCF shadow
    var visibility = 0.0;
    let dims = vec2<f32>(textureDimensions(shadow_map).xy);

    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = vec2<f32>(f32(x), f32(y)) / dims;
            visibility += textureSampleCompare(
                shadow_map,
                shadow_sampler,
                in.light_space_position.xy + offset,
                in.light_space_position.z - 0.007
            );
        }
    }

    visibility /= 9.0;
    visibility = mix(0.5, 1.0, visibility);

    let albedo_color =  textureSample(
        albedo_texture,
        albedo_sampler,
        in.tex_coords.xy,
        u32(in.tex_coords.z)
    ).rgb;

    let light_color = uniform_buffer.light_color.rgb;

    // light direction in view space (matches in.normal)
    let light_dir_view = normalize(
        (uniform_buffer.view_matrix * vec4<f32>(-uniform_buffer.light_direction, 0.0)).xyz
    );

    // diffuse shading
    let diff = max(dot(in.normal, light_dir_view), 0.0);
    let ambient = 0.2 * light_color;

    // simple lambert + ambient
    let color = ambient + visibility * diff * albedo_color;

    // gamma correction to sRGB
    let mapped_color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
}

