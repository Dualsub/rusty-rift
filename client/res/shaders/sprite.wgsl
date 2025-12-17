// Vertex shader

struct Instance {
    position_and_scale: vec4<f32>,
    color: vec4<f32>,
    tex_bounds: vec4<f32>,
    mode: u32,
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
    @location(2) @interpolate(flat) mode: u32,
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
    out.mode = instance.mode;

    return out;
}

// Fragment shader

fn median3(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}

fn screen_px_range(uv: vec2<f32>, tex_size: vec2<f32>) -> f32 {
    let unit_range = vec2<f32>(2.0, 2.0) / tex_size;
    let screen_tex_size = vec2<f32>(1.0, 1.0) / fwidth(uv);
    return max(0.5 * dot(unit_range, screen_tex_size), 1.0);
}

fn msdf_opacity_from_texel(texel: vec4<f32>, uv: vec2<f32>, tex_size: vec2<f32>) -> f32 {
    let sd = median3(texel.r, texel.g, texel.b);
    let px_dist = screen_px_range(uv, tex_size) * (sd - 0.5);
    return clamp(px_dist + 0.5, 0.0, 1.0);
}

@group(1) @binding(0) var texture: texture_2d_array<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size_u = textureDimensions(texture, 0);
    let tex_size = vec2<f32>(f32(tex_size_u.x), f32(tex_size_u.y));

    let texel = textureSample(texture, texture_sampler, in.tex_coords.xy, i32(in.tex_coords.z));

    // Always compute (keeps fwidth in uniform control flow)
    let msdf_opacity = msdf_opacity_from_texel(texel, in.tex_coords.xy, tex_size);

    // mode: 0 = sprite, 1 = msdf. Convert to 0.0/1.0 for mixing.
    let use_msdf = select(0.0, 1.0, in.mode != 0u);

    // Sprite path keeps sampled alpha. MSDF path uses computed opacity.
    let alpha = mix(texel.a, msdf_opacity, use_msdf);

    let sprite_rgb = texel.rgb * in.color.rgb;
    let msdf_rgb = in.color.rgb;
    let rgb = mix(sprite_rgb, msdf_rgb, use_msdf);

    return vec4<f32>(rgb, in.color.a * alpha);
}

