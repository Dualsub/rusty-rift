// Vertex shader

struct UniformBuffer {
    // x = screen_w_px, y = screen_h_px, z = ui_scale, w unused
    screen_size_and_ui_scale: vec4<f32>,
};

struct Instance {
    position_and_scale: vec4<f32>, // xy = pos_ref_offset, zw = size_ref
    color: vec4<f32>,
    tex_bounds: vec4<f32>,         // xy = uv_min, zw = uv_extent
    mode_layer_anchor_space: vec4<u32>, // x=mode, y=layer, z=anchor, w unused here
};

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>, // unit quad 0..1 (y-up in mesh)
    @location(1) normal: vec3<f32>,
    @location(2) uvs: vec3<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) mode: u32,
    @location(3) @interpolate(flat) layer: u32,
};

@group(0) @binding(0) var<uniform> uniform_buffer: UniformBuffer;
@group(0) @binding(1) var<storage, read> instance_buffer: array<Instance>;

fn anchor_origin_px(anchor: u32, screen_px: vec2<f32>) -> vec2<f32> {
    let ax = anchor % 3u;
    let ay = anchor / 3u;
    let ox = select(0.0, select(0.5, 1.0, ax == 2u), ax != 0u);
    let oy = select(0.0, select(0.5, 1.0, ay == 2u), ay != 0u);
    return vec2<f32>(ox * screen_px.x, oy * screen_px.y);
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let instance = instance_buffer[in.instance_index];

    let screen_px = uniform_buffer.screen_size_and_ui_scale.xy;
    let ui_scale = uniform_buffer.screen_size_and_ui_scale.z;

    let mode = instance.mode_layer_anchor_space.x;
    let layer = instance.mode_layer_anchor_space.y;
    let anchor = instance.mode_layer_anchor_space.z;
    let space = instance.mode_layer_anchor_space.w;

    var out: VertexOutput;

    if (space == 0) { // Reference space
        let local01 = vec2<f32>(in.position.x, 1.0 - in.position.y);
        let anchor_px = anchor_origin_px(anchor, screen_px);
        let pos_px  = anchor_px + instance.position_and_scale.xy * ui_scale;
        let size_px = instance.position_and_scale.zw * ui_scale;
        let p_px = pos_px + local01 * size_px;
        let ndc_x = (p_px.x / screen_px.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (p_px.y / screen_px.y) * 2.0;
        out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    } else if (space == 1) { // Absolute space
        let local01 = vec2<f32>(in.position.x, 1.0 - in.position.y);
        let anchor_px = anchor_origin_px(anchor, screen_px);
        let pos_px = anchor_px + instance.position_and_scale.xy;
        let size_px = instance.position_and_scale.zw;
        let p_px = pos_px + local01 * size_px;
        let ndc_x = (p_px.x / screen_px.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (p_px.y / screen_px.y) * 2.0;
        out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    } else {
        out.clip_position = vec4<f32>(in.position.x, 1.0 - in.position.y, 0.0, 1.0);
    }

    out.tex_coords = vec3<f32>(
        instance.tex_bounds.xy + in.uvs.xy * instance.tex_bounds.zw,
        f32(layer)
    );
    out.color = in.color * instance.color;
    out.mode = mode;
    out.layer = layer;

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
    let msdf_opacity = msdf_opacity_from_texel(texel, in.tex_coords.xy, tex_size);
    let use_msdf = select(0.0, 1.0, in.mode != 0u);
    let alpha = mix(texel.a, msdf_opacity, use_msdf);

    let sprite_rgb = texel.rgb * in.color.rgb;
    let msdf_rgb = in.color.rgb;
    let rgb = mix(sprite_rgb, msdf_rgb, use_msdf);

    return vec4<f32>(rgb, in.color.a * alpha);
}

