// Fragment shader

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

const PI: f32 = 3.14159265;

fn g_schlick_ggx(n_dot_x: f32, k: f32) -> f32 {
    let denom = n_dot_x * (1.0 - k) + k;
    return n_dot_x / max(denom, 1e-4);
}

fn stylized_ggx_pbr(
    N_in: vec3<f32>,
    V_in: vec3<f32>,
    L_in: vec3<f32>,
    albedo: vec3<f32>,
    roughness: f32,
    metallic: f32,
    light_color: vec3<f32>,
    ambient_top: vec3<f32>,
    ambient_bottom: vec3<f32>,
    visibility: f32,
) -> vec3<f32> {
    // Normalize inputs
    let N = normalize(N_in);
    let V = normalize(V_in);
    let L = normalize(L_in);

    let H = normalize(V + L);

    let NdotL = max(dot(N, L), 0.0);
    let NdotV = max(dot(N, V), 0.0);
    let NdotH = max(dot(N, H), 0.0);
    let VdotH = max(dot(V, H), 0.0);

    // ---------------------------------------------------------------------
    // 1. Stylized wrapped diffuse
    // ---------------------------------------------------------------------
    let wrap_amount: f32 = 0.4;
    let wrapped = clamp((NdotL + wrap_amount) / (1.0 + wrap_amount), 0.0, 1.0);

    let softness: f32 = 0.8;
    let diffuse_term = pow(wrapped, softness);

    // ---------------------------------------------------------------------
    // 2. Hemispheric ambient
    // ---------------------------------------------------------------------
    let up = N.y * 0.5 + 0.5; // [-1,1] -> [0,1]
    let ambient_dir_color = mix(ambient_bottom, ambient_top, up);
    let ambient_strength: f32 = 0.4;
    let ambient = ambient_dir_color * ambient_strength;

    let diffuse_light = light_color * diffuse_term * visibility;
    let base_diffuse = albedo * (diffuse_light + ambient);

    // ---------------------------------------------------------------------
    // 3. GGX specular
    // ---------------------------------------------------------------------
    let a = roughness * roughness;
    let a2 = a * a;

    // Normal distribution function (GGX / Trowbridge-Reitz)
    let denom_d = (NdotH * NdotH) * (a2 - 1.0) + 1.0;
    let D = a2 / (PI * denom_d * denom_d + 1e-4);

    // Geometry term (Smith with Schlick-GGX)
    var k = a + 1.0;
    k = (k * k) / 8.0;

    let Gv = g_schlick_ggx(NdotV, k);
    let Gl = g_schlick_ggx(NdotL, k);
    let G = Gv * Gl;

    // Fresnel (Schlick approximation)
    let F0 = mix(vec3<f32>(0.04, 0.04, 0.04), albedo, metallic);
    let one_minus_vh = 1.0 - VdotH;
    let one_minus_vh5 = one_minus_vh * one_minus_vh * one_minus_vh * one_minus_vh * one_minus_vh;
    let F = F0 + (vec3<f32>(1.0, 1.0, 1.0) - F0) * one_minus_vh5;

    let numer = D * G * F;
    let denom_spec = max(4.0 * NdotL * NdotV, 1e-4);
    var spec_brdf = numer / denom_spec;

    // Light * NdotL
    var specular = spec_brdf * light_color * NdotL * visibility;

    // Stylized control: boost and clamp a bit
    let spec_boost: f32 = 1.0;
    let spec_max_clamp: f32 = 5.0;
    specular *= spec_boost;
    specular = clamp(specular, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(spec_max_clamp, spec_max_clamp, spec_max_clamp));

    // Soft compression to avoid insane hotspots
    specular = specular / (specular + vec3<f32>(1.0, 1.0, 1.0));

    // ---------------------------------------------------------------------
    // 4. Rim light for champions
    // ---------------------------------------------------------------------
    // let ndotv = NdotV;
    // var rim = 1.0 - ndotv;

    // let rim_power: f32 = 2.5;     // higher = thinner rim
    // let rim_intensity: f32 = 0.5; // overall strength
    // rim = pow(rim, rim_power);

    // // Make rim depend somewhat on lighting so dark backsides don't glow too much
    // rim *= (0.3 + 0.7 * diffuse_term);

    // let rim_color = normalize(light_color + vec3<f32>(0.3, 0.3, 0.3));
    // let rim_light = rim_color * rim * rim_intensity;

    // ---------------------------------------------------------------------
    // 5. Final color
    // ---------------------------------------------------------------------
    var color = base_diffuse + specular;
    color = clamp(color, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));

    return color;
}

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
    visibility = mix(0.4, 1.0, visibility);

    let albedo =  textureSample(
        albedo_texture,
        albedo_sampler,
        in.tex_coords.xy,
        i32(in.tex_coords.z)
    ).rgb * in.color.rgb;

    let light_color = uniform_buffer.light_color.rgb;

    let N = normalize(in.world_normal);
    let V = normalize(uniform_buffer.camera_position - in.world_position);
    let L = normalize(-uniform_buffer.light_direction);

    let roughness: f32 = 0.8;
    let metallic: f32 = 0.0;
    let ambient_top    = vec3<f32>(0.35, 0.50, 0.80);
    let ambient_bottom = vec3<f32>(0.30, 0.25, 0.20);

    let color = stylized_ggx_pbr(
        N,
        V,
        L,
        albedo,
        roughness,
        metallic,
        light_color,
        ambient_top,
        ambient_bottom,
        visibility
    );

    // gamma correction to sRGB
    let mapped_color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));

    return vec4<f32>(mapped_color, 1.0);
}

