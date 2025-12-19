use shared::{math::*, transform::Transform};
use std::ops::Range;
use std::sync::Arc;
use wgpu::BufferUsages;
use winit::window::Window;

use crate::renderer::{
    Buffer, BufferDesc, Glyph, MaterialInstanceDesc, MaterialPipeline, MaterialPipelineDesc,
    MeshLoadDesc, PassTarget, RenderData, RenderDevice, Resource, ResourceHandle, ResourcePool,
    SkeletalMeshVertex, SpriteInstanceData, StaticInstanceData, StaticMesh, StaticMeshVertex,
    Texture, TextureDesc,
    animation::{AnimationInstance, Pose},
    render_data::SubmitJob,
    resources::get_handle,
};

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformBufferData {
    view_matrix: Mat4Data,
    projection_matrix: Mat4Data,
    camera_position: Vec4Data,

    light_matrix: Mat4Data,
    light_direction: Vec4Data,
    light_color: Vec4Data,
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteUniformBufferData {
    pub screen_size: Vec2Data,
    pub ui_scale: f32,
    _padding: f32,
}

pub struct RenderBatch {
    pub material_instance: ResourceHandle,
    pub mesh: ResourceHandle,
    pub instance_range: Range<u32>,
}

// Generated before each draw
pub struct DrawData {
    pub static_batches: Vec<RenderBatch>,
    pub static_instances: Vec<StaticInstanceData>,

    pub skeletal_batches: Vec<RenderBatch>,
    pub skeletal_instances: Vec<StaticInstanceData>,
    pub bones: Vec<Mat4Data>,

    pub sprite_batches: Vec<RenderBatch>,
    pub sprite_instances: Vec<SpriteInstanceData>,
}

// A short-term abstraction
pub struct MaterialGroup {
    static_material_pipeline: MaterialPipeline,
    skeletal_material_pipeline: MaterialPipeline,
}

pub struct BindCollection {
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

pub struct BindEntry<'a> {
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BindingType,
    resource: wgpu::BindingResource<'a>,
}

impl RenderDevice {
    pub fn create_bind_collection<'a>(&self, entries: Vec<BindEntry<'a>>) -> BindCollection {
        // First build layout entries (we only need binding/visibility/type here)
        let layout_entries: Vec<wgpu::BindGroupLayoutEntry> = entries
            .iter()
            .map(|e| wgpu::BindGroupLayoutEntry {
                binding: e.binding,
                visibility: e.visibility,
                ty: e.ty.clone(),
                count: None,
            })
            .collect();

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &layout_entries,
                });

        let group_entries: Vec<wgpu::BindGroupEntry> = entries
            .into_iter()
            .map(|e| wgpu::BindGroupEntry {
                binding: e.binding,
                resource: e.resource,
            })
            .collect();

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &group_entries,
        });

        BindCollection {
            bind_group,
            bind_group_layout,
        }
    }
}

pub struct Renderer {
    render_device: RenderDevice,
    resource_pool: ResourcePool,

    screen_mesh: StaticMesh,

    _depth_sampler: wgpu::Sampler,
    default_sampler: wgpu::Sampler,

    shadow_map: Texture,
    depth_buffer: Texture,
    scene_texture: Texture,

    static_shadow_bind_collection: BindCollection,
    skeletal_shadow_bind_collection: BindCollection,
    shadow_material_pipeline: MaterialGroup,
    sprite_material_pipeline: MaterialPipeline,

    static_scene_bind_collection: BindCollection,
    skeletal_scene_bind_collection: BindCollection,
    scene_material_pipeline: MaterialGroup,
    sprite_bind_collection: BindCollection,

    composite_bind_collection: BindCollection,
    composite_material_pipeline: MaterialPipeline,

    camera_projection_matrix: Mat4,
    camera_transform: Transform,
    uniform_data: UniformBufferData,
    sprite_uniform_data: SpriteUniformBufferData,

    uniform_buffer: Buffer,
    static_instance_buffer: Buffer,
    skeletal_instance_buffer: Buffer,
    bone_buffer: Buffer,
    sprite_uniform_buffer: Buffer,
    sprite_instance_buffer: Buffer,

    render_data: RenderData,
}

impl Renderer {
    const SHADOW_MAP_WIDTH: u32 = 2048;
    const SHADOW_MAP_HEIGHT: u32 = 2048;

    const STATIC_INSTANCE_COUNT: usize = 512;
    const BONE_COUNT: usize = Self::STATIC_INSTANCE_COUNT * 64;
    const SRPITE_INSTANCE_COUNT: usize = 2046;

    pub const SPRITE_SCREEN_REFERENCE: Vec2 = Vec2::new(1920.0, 1080.0);
    pub const QUAD_MESH: ResourceHandle = get_handle("quad");
    pub const WHITE_SPRITE_MATERIAL: ResourceHandle = get_handle("white_sprite_material");

    fn create_default_resources(
        render_device: &RenderDevice,
        sprite_material_pipeline: &MaterialPipeline,
        sampler: &wgpu::Sampler,
        resource_pool: &mut ResourcePool,
    ) {
        let texture = render_device.create_texture(&TextureDesc {
            width: 1,
            height: 1,
            layer_count: 1,
            mip_level_count: 1,
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            bytes_per_channel: 1,
            channel_count: 4,
            pixels: vec![255u8, 255u8, 255u8, 255u8],
            ..Default::default()
        });

        let white_sprite_material = render_device.create_material_instance(
            &sprite_material_pipeline, // Need to be looked over later
            &MaterialInstanceDesc {
                entires: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            },
        );

        resource_pool.add_resource(
            Self::WHITE_SPRITE_MATERIAL,
            Resource::MaterialInstance(white_sprite_material),
        );
    }

    fn create_depth_buffer(render_device: &RenderDevice) -> Texture {
        render_device.create_texture(&TextureDesc {
            width: render_device.config.width.max(1),
            height: render_device.config.height.max(1),
            layer_count: 1,
            format: Some(wgpu::TextureFormat::Depth32Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        })
    }

    fn create_scene_texture(render_device: &RenderDevice) -> Texture {
        render_device.create_texture(&TextureDesc {
            width: render_device.config.width.max(1),
            height: render_device.config.height.max(1),
            layer_count: 1,
            format: Some(wgpu::TextureFormat::Rgba16Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_dimension: wgpu::TextureViewDimension::D2,
            ..Default::default()
        })
    }

    fn create_samplers(render_device: &RenderDevice) -> (wgpu::Sampler, wgpu::Sampler) {
        let default_sampler = render_device
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

        let depth_sampler = render_device
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::Less),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            });

        (default_sampler, depth_sampler)
    }

    fn create_shadow_map(render_device: &RenderDevice) -> Texture {
        render_device.create_texture(&TextureDesc {
            width: Self::SHADOW_MAP_WIDTH,
            height: Self::SHADOW_MAP_HEIGHT,
            layer_count: 1,
            format: Some(wgpu::TextureFormat::Depth32Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_dimension: wgpu::TextureViewDimension::D2,
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        })
    }

    fn create_meshes(render_device: &RenderDevice) -> (StaticMesh, StaticMesh) {
        let screen_vertices: [StaticMeshVertex; 3] = [
            StaticMeshVertex {
                position: [-1.0, -1.0, 0.0],
                uvs: [0.0, 2.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
            },
            StaticMeshVertex {
                position: [3.0, -1.0, 0.0],
                uvs: [2.0, 2.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
            },
            StaticMeshVertex {
                position: [-1.0, 3.0, 0.0],
                uvs: [0.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
            },
        ];

        let screen_mesh = render_device
            .create_mesh(&MeshLoadDesc {
                vertex_data: bytemuck::cast_slice(screen_vertices.as_slice()).to_vec(),
                indices: vec![0, 1, 2],
                ..Default::default()
            })
            .expect("Could not create fullscreen mesh");

        let quad_vertices: [StaticMeshVertex; 4] = [
            // bottom-left
            StaticMeshVertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uvs: [0.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            // bottom-right
            StaticMeshVertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uvs: [1.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            // top-right
            StaticMeshVertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uvs: [1.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            // top-left
            StaticMeshVertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uvs: [0.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        let quad_mesh = render_device
            .create_mesh(&MeshLoadDesc {
                vertex_data: bytemuck::cast_slice(quad_vertices.as_slice()).to_vec(),
                indices: vec![0, 1, 2, 0, 2, 3],
                ..Default::default()
            })
            .expect("Could not create quad mesh");

        (screen_mesh, quad_mesh)
    }

    fn create_storage_buffers(render_device: &RenderDevice) -> (Buffer, Buffer, Buffer, Buffer) {
        let size = Self::STATIC_INSTANCE_COUNT * std::mem::size_of::<StaticInstanceData>();

        let static_instance_buffer = render_device.create_buffer(&BufferDesc {
            size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let skeletal_instance_buffer = render_device.create_buffer(&BufferDesc {
            size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bone_buffer = render_device.create_buffer(&BufferDesc {
            size: Self::BONE_COUNT * std::mem::size_of::<Mat4Data>(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let sprite_instance_buffer = render_device.create_buffer(&BufferDesc {
            size: Self::SRPITE_INSTANCE_COUNT * std::mem::size_of::<SpriteInstanceData>(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        (
            static_instance_buffer,
            skeletal_instance_buffer,
            bone_buffer,
            sprite_instance_buffer,
        )
    }

    fn create_uniform_buffers(render_device: &RenderDevice) -> (Buffer, Buffer) {
        (
            render_device.create_buffer(&BufferDesc {
                size: std::mem::size_of::<UniformBufferData>(),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }),
            render_device.create_buffer(&BufferDesc {
                size: std::mem::size_of::<SpriteUniformBufferData>(),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }),
        )
    }

    fn create_bind_collections(
        render_device: &RenderDevice,
        uniform_buffer: &Buffer,
        shadow_map: &Texture,
        depth_sampler: &wgpu::Sampler,
        static_instance_buffer: &Buffer,
        skeletal_instance_buffer: &Buffer,
        bone_buffer: &Buffer,
    ) -> (
        BindCollection,
        BindCollection,
        BindCollection,
        BindCollection,
    ) {
        let static_scene = render_device.create_bind_collection(vec![
            BindEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: uniform_buffer.buffer.as_entire_binding(),
            },
            BindEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: static_instance_buffer.buffer.as_entire_binding(),
            },
            BindEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                resource: wgpu::BindingResource::TextureView(&shadow_map.view),
            },
            BindEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                resource: wgpu::BindingResource::Sampler(depth_sampler),
            },
        ]);

        let skeletal_scene = render_device.create_bind_collection(vec![
            BindEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: uniform_buffer.buffer.as_entire_binding(),
            },
            BindEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: skeletal_instance_buffer.buffer.as_entire_binding(),
            },
            BindEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                resource: wgpu::BindingResource::TextureView(&shadow_map.view),
            },
            BindEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                resource: wgpu::BindingResource::Sampler(depth_sampler),
            },
            BindEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: bone_buffer.buffer.as_entire_binding(),
            },
        ]);

        let static_shadow = render_device.create_bind_collection(vec![
            BindEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: uniform_buffer.buffer.as_entire_binding(),
            },
            BindEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: static_instance_buffer.buffer.as_entire_binding(),
            },
        ]);

        let skeletal_shadow = render_device.create_bind_collection(vec![
            BindEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: uniform_buffer.buffer.as_entire_binding(),
            },
            BindEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: skeletal_instance_buffer.buffer.as_entire_binding(),
            },
            BindEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: bone_buffer.buffer.as_entire_binding(),
            },
        ]);

        return (static_scene, skeletal_scene, static_shadow, skeletal_shadow);
    }

    fn create_sprite_pipeline(
        render_device: &RenderDevice,
        uniform_buffer: &Buffer,
        instance_buffer: &Buffer,
    ) -> (BindCollection, MaterialPipeline) {
        let bind_collection = render_device.create_bind_collection(vec![
            BindEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: uniform_buffer.buffer.as_entire_binding(),
            },
            BindEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resource: instance_buffer.buffer.as_entire_binding(),
            },
        ]);

        let sprite_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("SpriteShader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/sprite.wgsl").into(),
                    ),
                });

        let material_pipeline = render_device.create_material_pipeline(&MaterialPipelineDesc {
            vertex_shader: &sprite_shader,
            fragment_shader: Some(&sprite_shader),
            bind_group_layouts: &[&bind_collection.bind_group_layout],
            layout_entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            vertex_layout: &StaticMeshVertex::desc(),
            push_contant_ranges: &[],
            pass_target: PassTarget::Composite,
        });

        return (bind_collection, material_pipeline);
    }

    fn create_composite_pipeline(
        render_device: &RenderDevice,
        scene_texture: &Texture,
        sampler: &wgpu::Sampler,
    ) -> (BindCollection, MaterialPipeline) {
        let bind_collection = render_device.create_bind_collection(vec![
            BindEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                resource: wgpu::BindingResource::TextureView(&scene_texture.view),
            },
            BindEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ]);

        let composite_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("CompositeShader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/composite.wgsl").into(),
                    ),
                });

        let material_pipeline = render_device.create_material_pipeline(&MaterialPipelineDesc {
            vertex_shader: &composite_shader,
            fragment_shader: Some(&composite_shader),
            bind_group_layouts: &[&bind_collection.bind_group_layout],
            layout_entries: &[],
            vertex_layout: &StaticMeshVertex::desc(),
            push_contant_ranges: &[],
            pass_target: PassTarget::Composite,
        });

        return (bind_collection, material_pipeline);
    }

    fn create_shadow_material_pipelines(
        render_device: &RenderDevice,
        static_bind_group_layout: &wgpu::BindGroupLayout,
        skeletal_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> MaterialGroup {
        let static_shadow_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("StaticShadowShader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/static_shadow.wgsl").into(),
                    ),
                });

        let skeletal_shadow_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("SkeletalShadowShader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/skeletal_shadow.wgsl").into(),
                    ),
                });

        MaterialGroup {
            static_material_pipeline: render_device.create_material_pipeline(
                &MaterialPipelineDesc {
                    vertex_shader: &static_shadow_shader,
                    fragment_shader: None,
                    bind_group_layouts: &[static_bind_group_layout],
                    layout_entries: &[],
                    vertex_layout: &StaticMeshVertex::desc(),
                    push_contant_ranges: &[],
                    pass_target: PassTarget::Scene,
                },
            ),
            skeletal_material_pipeline: render_device.create_material_pipeline(
                &MaterialPipelineDesc {
                    vertex_shader: &skeletal_shadow_shader,
                    fragment_shader: None,
                    bind_group_layouts: &[skeletal_bind_group_layout],
                    layout_entries: &[],
                    vertex_layout: &SkeletalMeshVertex::desc(),
                    push_contant_ranges: &[],
                    pass_target: PassTarget::Scene,
                },
            ),
        }
    }

    fn create_scene_material_pipelines(
        render_device: &RenderDevice,
        static_bind_group_layout: &wgpu::BindGroupLayout,
        skeletal_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> MaterialGroup {
        let static_vertex_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("StaticVertexShader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/static.wgsl").into(),
                    ),
                });

        let skeletal_vertex_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("SkeletalVertexShader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/skeletal.wgsl").into(),
                    ),
                });

        let fragment_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("SceneFragmentShader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/scene.wgsl").into(),
                    ),
                });

        let material_layout_entries = [
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ];

        MaterialGroup {
            static_material_pipeline: render_device.create_material_pipeline(
                &MaterialPipelineDesc {
                    bind_group_layouts: &[static_bind_group_layout],
                    push_contant_ranges: &[],
                    pass_target: PassTarget::Scene,
                    vertex_shader: &static_vertex_shader,
                    fragment_shader: Some(&fragment_shader),
                    layout_entries: &material_layout_entries,
                    vertex_layout: &StaticMeshVertex::desc(),
                },
            ),
            skeletal_material_pipeline: render_device.create_material_pipeline(
                &MaterialPipelineDesc {
                    bind_group_layouts: &[skeletal_bind_group_layout],
                    push_contant_ranges: &[],
                    pass_target: PassTarget::Scene,
                    vertex_shader: &skeletal_vertex_shader,
                    fragment_shader: Some(&fragment_shader),
                    layout_entries: &material_layout_entries,
                    vertex_layout: &SkeletalMeshVertex::desc(),
                },
            ),
        }
    }

    pub async fn new(window: &Arc<Window>) -> anyhow::Result<Renderer> {
        let render_device = RenderDevice::new(window).await?;
        let mut resource_pool = ResourcePool::new();

        let (screen_mesh, quad_mesh) = Self::create_meshes(&render_device);
        resource_pool.add_resource(Self::QUAD_MESH, Resource::StaticMesh(quad_mesh));

        let (default_sampler, depth_sampler) = Self::create_samplers(&render_device);

        let shadow_map = Self::create_shadow_map(&render_device);
        let depth_buffer = Renderer::create_depth_buffer(&render_device);
        let scene_texture = Renderer::create_scene_texture(&render_device);

        let (static_instance_buffer, skeletal_instance_buffer, bone_buffer, sprite_instance_buffer) =
            Self::create_storage_buffers(&render_device);
        let (uniform_buffer, sprite_uniform_buffer) = Self::create_uniform_buffers(&render_device);

        let (
            static_scene_bind_collection,
            skeletal_scene_bind_collection,
            static_shadow_bind_collection,
            skeletal_shadow_bind_collection,
        ) = Self::create_bind_collections(
            &render_device,
            &uniform_buffer,
            &shadow_map,
            &depth_sampler,
            &static_instance_buffer,
            &skeletal_instance_buffer,
            &bone_buffer,
        );

        let (sprite_bind_collection, sprite_material_pipeline) = Self::create_sprite_pipeline(
            &render_device,
            &sprite_uniform_buffer,
            &sprite_instance_buffer,
        );

        let (composite_bind_collection, composite_material_pipeline) =
            Self::create_composite_pipeline(&render_device, &scene_texture, &default_sampler);

        let shadow_material_pipeline = Self::create_shadow_material_pipelines(
            &render_device,
            &static_shadow_bind_collection.bind_group_layout,
            &skeletal_shadow_bind_collection.bind_group_layout,
        );
        let scene_material_pipeline = Self::create_scene_material_pipelines(
            &render_device,
            &static_scene_bind_collection.bind_group_layout,
            &skeletal_scene_bind_collection.bind_group_layout,
        );

        Self::create_default_resources(
            &render_device,
            &sprite_material_pipeline,
            &default_sampler,
            &mut resource_pool,
        );

        Ok(Renderer {
            render_device,
            resource_pool,
            screen_mesh,
            default_sampler,
            _depth_sampler: depth_sampler,
            shadow_map,
            depth_buffer,
            scene_texture,
            static_shadow_bind_collection,
            skeletal_shadow_bind_collection,
            sprite_bind_collection,
            shadow_material_pipeline,
            sprite_material_pipeline,
            camera_transform: Transform {
                position: Vec3 {
                    x: 0.0,
                    y: 400.0,
                    z: 0.0,
                },
                rotation: Quat::from_rotation_x(f32::to_radians(-30.0)),
                ..Default::default()
            },
            camera_projection_matrix: Mat4::IDENTITY,
            render_data: RenderData::new(),
            uniform_buffer,
            sprite_uniform_buffer,
            uniform_data: UniformBufferData {
                view_matrix: Mat4::IDENTITY.to_data(),
                projection_matrix: Mat4::IDENTITY.to_data(),
                camera_position: [0.0, 0.0, 0.0, 0.0],
                light_matrix: Mat4::IDENTITY.to_data(),
                light_direction: [0.0, -1.0, -1.0, 0.0],
                light_color: [1.0, 1.0, 1.0, 1.0],
            },
            sprite_uniform_data: Default::default(),
            composite_bind_collection,
            composite_material_pipeline,
            static_instance_buffer,
            skeletal_instance_buffer,
            bone_buffer,
            sprite_instance_buffer,
            scene_material_pipeline,
            static_scene_bind_collection,
            skeletal_scene_bind_collection,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let render_device = &mut self.render_device;

            render_device.config.width = width;
            render_device.config.height = height;
            render_device
                .surface
                .configure(&render_device.device, &render_device.config);
            render_device.is_surface_configured = true;

            self.sprite_uniform_data.screen_size = [width as f32, height as f32];
            self.sprite_uniform_data.ui_scale = f32::min(
                width as f32 / Self::SPRITE_SCREEN_REFERENCE.x,
                height as f32 / Self::SPRITE_SCREEN_REFERENCE.y,
            );

            self.depth_buffer = Renderer::create_depth_buffer(&render_device);
            self.scene_texture = Renderer::create_scene_texture(&render_device);
            let (composite_bind_collection, composite_material_pipeline) =
                Self::create_composite_pipeline(
                    &render_device,
                    &self.scene_texture,
                    &self.default_sampler,
                );
            self.composite_bind_collection = composite_bind_collection;
            self.composite_material_pipeline = composite_material_pipeline;
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if !self.render_device.is_surface_configured {
            return Ok(());
        }

        self.upload_uniform_buffer();

        let draw_data = self.render_data.build_draw_data();

        self.upload_draw_data(&draw_data);

        self.draw_frame(&draw_data)
    }

    fn upload_uniform_buffer(&mut self) {
        self.uniform_data.projection_matrix = self.camera_projection_matrix.to_data();

        self.camera_transform.rotation *= Quat::from_rotation_y(f32::to_radians(0.1));
        let view_matrix = self.camera_transform.to_matrix().inverse();
        self.uniform_data.view_matrix = view_matrix.to_data();
        self.uniform_data.camera_position = [
            self.camera_transform.position.x,
            self.camera_transform.position.y,
            self.camera_transform.position.z,
            0.0,
        ];
        self.uniform_data.light_matrix = Self::compute_directional_light_vp(
            view_matrix,
            self.camera_projection_matrix,
            Vec3::from_slice(&self.uniform_data.light_direction),
        )
        .to_data();

        self.render_device.write_buffer(
            &self.uniform_buffer,
            bytemuck::bytes_of(&self.uniform_data),
            0,
        );

        self.render_device.write_buffer(
            &self.sprite_uniform_buffer,
            bytemuck::bytes_of(&self.sprite_uniform_data),
            0,
        );
    }

    fn upload_draw_data(&mut self, draw_data: &DrawData) {
        self.render_device.write_buffer(
            &self.static_instance_buffer,
            bytemuck::cast_slice(draw_data.static_instances.as_slice()),
            0,
        );

        self.render_device.write_buffer(
            &self.skeletal_instance_buffer,
            bytemuck::cast_slice(draw_data.skeletal_instances.as_slice()),
            0,
        );

        self.render_device.write_buffer(
            &self.bone_buffer,
            bytemuck::cast_slice(draw_data.bones.as_slice()),
            0,
        );

        self.render_device.write_buffer(
            &self.sprite_instance_buffer,
            bytemuck::cast_slice(draw_data.sprite_instances.as_slice()),
            0,
        );
    }

    fn draw_frame(&self, draw_data: &DrawData) -> Result<(), wgpu::SurfaceError> {
        let output = self.render_device.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.render_device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.render_batches(
                &mut render_pass,
                &self.shadow_material_pipeline.static_material_pipeline,
                &[&self.static_shadow_bind_collection.bind_group],
                &draw_data.static_batches,
            );

            self.render_batches(
                &mut render_pass,
                &self.shadow_material_pipeline.skeletal_material_pipeline,
                &[&self.skeletal_shadow_bind_collection.bind_group],
                &draw_data.skeletal_batches,
            );
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scene Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_texture.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_buffer.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.render_batches(
                &mut render_pass,
                &self.scene_material_pipeline.static_material_pipeline,
                &[&self.static_scene_bind_collection.bind_group],
                &draw_data.static_batches,
            );

            self.render_batches(
                &mut render_pass,
                &self.scene_material_pipeline.skeletal_material_pipeline,
                &[&self.skeletal_scene_bind_collection.bind_group],
                &draw_data.skeletal_batches,
            );
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Composite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Full screen quad draw
            {
                render_pass.set_pipeline(&self.composite_material_pipeline.pipeline);
                render_pass.set_bind_group(0, &self.composite_bind_collection.bind_group, &[]);
                let draw_info = self.screen_mesh.get_draw_info();
                render_pass.set_vertex_buffer(0, draw_info.vertex_slice);
                render_pass.set_index_buffer(draw_info.index_slice, wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..draw_info.index_count, 0, 0..1);
            }

            // Sprite Rendering
            {
                self.render_batches(
                    &mut render_pass,
                    &self.sprite_material_pipeline,
                    &[&self.sprite_bind_collection.bind_group],
                    &draw_data.sprite_batches,
                );
            }
        }

        self.render_device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn render_batches(
        &self,
        render_pass: &mut wgpu::RenderPass,
        material_pipeline: &MaterialPipeline,
        bind_groups: &[&wgpu::BindGroup],
        batches: &[RenderBatch],
    ) {
        render_pass.set_pipeline(&material_pipeline.pipeline);
        for batch in batches {
            let material_instance = self
                .resource_pool
                .get_material_instance(batch.material_instance)
                .unwrap();

            let mut bind_group_index: u32 = 0;
            for bind_group in bind_groups {
                render_pass.set_bind_group(bind_group_index, *bind_group, &[]);
                bind_group_index += 1;
            }

            render_pass.set_bind_group(bind_group_index, &material_instance.bind_group, &[]);

            let mesh_draw_info = self.resource_pool.get_mesh_draw_info(batch.mesh).unwrap();
            render_pass.set_vertex_buffer(0, mesh_draw_info.vertex_slice);
            render_pass.set_index_buffer(mesh_draw_info.index_slice, wgpu::IndexFormat::Uint32);

            // We can clone the range, it is very small so it is fine
            render_pass.draw_indexed(
                0..mesh_draw_info.index_count,
                0,
                batch.instance_range.clone(),
            );
        }
    }

    pub fn set_camera_position_and_orientation(&mut self, position: Vec3, orientation: Quat) {
        self.camera_transform.position = position;
        self.camera_transform.rotation = orientation;
    }

    pub fn set_camera_projection(&mut self, projection: Mat4) {
        self.camera_projection_matrix = projection;
    }

    #[allow(dead_code)]
    pub fn set_lighting_color(&mut self, color: Vec3) {
        self.uniform_data.light_color = [color.x, color.y, color.z, 1.0];
    }

    #[allow(dead_code)]
    pub fn set_lighting_direction(&mut self, direction: Vec3) {
        self.uniform_data.light_direction = [direction.x, direction.y, direction.z, 0.0];
    }

    pub fn compute_directional_light_vp(
        camera_view: Mat4,
        camera_proj: Mat4,
        light_dir: Vec3,
    ) -> Mat4 {
        let inv_view_proj = (camera_proj * camera_view).inverse();

        let clip_space_corners = [
            Vec4::new(-1.0, -1.0, 0.0, 1.0),
            Vec4::new(1.0, -1.0, 0.0, 1.0),
            Vec4::new(-1.0, 1.0, 0.0, 1.0),
            Vec4::new(1.0, 1.0, 0.0, 1.0),
            Vec4::new(-1.0, -1.0, 1.0, 1.0),
            Vec4::new(1.0, -1.0, 1.0, 1.0),
            Vec4::new(-1.0, 1.0, 1.0, 1.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
        ];

        let mut frustum_corners_world = [Vec3::ZERO; 8];
        for (i, c) in clip_space_corners.iter().enumerate() {
            let world = inv_view_proj * *c;
            frustum_corners_world[i] = (world / world.w).truncate();
        }

        let mut center = Vec3::ZERO;
        for c in &frustum_corners_world {
            center += *c;
        }
        center /= frustum_corners_world.len() as f32;

        let light_dir_norm = light_dir.normalize();
        let light_forward = -light_dir_norm;

        let world_up = if light_forward.abs().dot(Vec3::Y).abs() > 0.9 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        let light_right = world_up.cross(light_forward).normalize();
        let light_up = light_forward.cross(light_right).normalize();

        let mut radius: f32 = 0.0;
        for c in &frustum_corners_world {
            radius = radius.max((*c - center).length());
        }
        let light_pos = center + light_forward * radius * 2.0;

        let light_view = Mat4::look_at_rh(light_pos, center, light_up);

        let mut min_ls = Vec3::splat(f32::INFINITY);
        let mut max_ls = Vec3::splat(f32::NEG_INFINITY);

        for c in &frustum_corners_world {
            let v = light_view * c.extend(1.0);
            let v3 = v.truncate();

            min_ls = min_ls.min(v3);
            max_ls = max_ls.max(v3);
        }

        let left = min_ls.x;
        let right = max_ls.x;
        let bottom = min_ls.y;
        let top = max_ls.y;

        let near_z = -max_ls.z - 10.0;
        let far_z = -min_ls.z + 10.0;

        let light_proj = Mat4::orthographic_rh(left, right, bottom, top, near_z.max(0.1), far_z);

        light_proj * light_view
    }

    pub fn load_mesh(&mut self, name: &'static str, bytes: &[u8]) -> ResourceHandle {
        let handle = get_handle(name);
        let mesh = self
            .render_device
            .load_mesh(bytes)
            .expect("Failed to load mesh");

        self.resource_pool
            .add_resource(handle, Resource::StaticMesh(mesh));

        handle
    }

    pub fn load_skeletal_mesh(&mut self, name: &'static str, bytes: &[u8]) -> ResourceHandle {
        let handle = get_handle(name);
        let mesh = self
            .render_device
            .load_skeletal_mesh(bytes)
            .expect("Failed to load mesh");

        self.resource_pool
            .add_resource(handle, Resource::SkeletalMesh(mesh));

        handle
    }

    pub fn create_pose(&self, mesh: ResourceHandle) -> Pose {
        let mesh = self
            .resource_pool
            .get_skeletal_mesh(mesh)
            .expect("Failed to get mesh for creating pose");
        Pose::new(mesh.bones.len())
    }

    pub fn load_animation(&mut self, name: &'static str, bytes: &[u8]) -> ResourceHandle {
        let handle = get_handle(name);
        let animation = self
            .render_device
            .load_animation(bytes)
            .expect("Failed to load animation");

        self.resource_pool
            .add_resource(handle, Resource::Animation(animation));

        handle
    }

    pub fn load_texture(&mut self, name: &'static str, bytes: &[u8]) -> ResourceHandle {
        let handle = get_handle(name);
        let texture = self
            .render_device
            .load_texture(bytes)
            .expect("Failed to load texture");

        self.resource_pool
            .add_resource(handle, Resource::Texture(texture));

        handle
    }

    pub fn load_font(&mut self, name: &'static str, bytes: &[u8]) -> ResourceHandle {
        let handle = get_handle(name);
        let font = self
            .render_device
            .load_font(bytes)
            .expect("Failed to load font");

        self.resource_pool
            .add_resource(handle, Resource::Font(font));

        handle
    }

    pub fn create_material(
        &mut self,
        name: &'static str,
        texture_handle: ResourceHandle,
    ) -> ResourceHandle {
        let handle = get_handle(name);
        let texture = self
            .resource_pool
            .get_texture(texture_handle)
            .expect("Failed to get texture");

        let material_instance = self.render_device.create_material_instance(
            &self.scene_material_pipeline.static_material_pipeline, // Need to be looked over later
            &MaterialInstanceDesc {
                entires: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                    },
                ],
            },
        );

        self.resource_pool
            .add_resource(handle, Resource::MaterialInstance(material_instance));

        handle
    }

    #[allow(dead_code)]
    pub fn create_sprite_material(
        &mut self,
        name: &'static str,
        texture_handle: ResourceHandle,
    ) -> ResourceHandle {
        let handle = get_handle(name);
        let texture = self
            .resource_pool
            .get_texture(texture_handle)
            .expect("Failed to get texture");

        let material_instance = self.render_device.create_material_instance(
            &self.sprite_material_pipeline, // Need to be looked over later
            &MaterialInstanceDesc {
                entires: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                    },
                ],
            },
        );

        self.resource_pool
            .add_resource(handle, Resource::MaterialInstance(material_instance));

        handle
    }

    pub fn create_font_material(
        &mut self,
        name: &'static str,
        font_handle: ResourceHandle,
    ) -> ResourceHandle {
        let handle = get_handle(name);
        let font = self
            .resource_pool
            .get_font(font_handle)
            .expect("Failed to get font");

        let material_instance = self.render_device.create_material_instance(
            &self.sprite_material_pipeline, // Need to be looked over later
            &MaterialInstanceDesc {
                entires: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&font.atlas.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                    },
                ],
            },
        );

        self.resource_pool
            .add_resource(handle, Resource::MaterialInstance(material_instance));

        handle
    }

    #[allow(dead_code)]
    pub fn sample_animation(
        &self,
        animation: ResourceHandle,
        time: f32,
        looping: bool,
        out_pose: &mut Pose,
    ) -> f32 {
        let animation = self
            .resource_pool
            .get_animation(animation)
            .expect("Fialed to get animation to sample");
        animation.sample(time, looping, out_pose)
    }

    #[allow(dead_code)]
    pub fn accumulate_pose(&self, instances: &[AnimationInstance], out_pose: &mut Pose) {
        if instances.is_empty() {
            return;
        }

        let mut total_weight = 0.0;
        out_pose.transforms.fill(Default::default());

        // First instance
        if let Some(instance) = instances.first() {
            let animation = self
                .resource_pool
                .get_animation(instance.animation)
                .expect("Could not find animation for instance");
            animation.sample(instance.time, instance.looping, out_pose);
            total_weight = instance.blend_weight;
        }

        if instances.len() <= 1 {
            return;
        }

        for instance in &instances[1..instances.len()] {
            total_weight += instance.blend_weight;
            let instance_weight = (instance.blend_weight / total_weight).clamp(0.0, 1.0);
            let animation = self
                .resource_pool
                .get_animation(instance.animation)
                .expect("Could not find animation for instance");
            animation.sample_and_blend(instance.time, instance.looping, instance_weight, out_pose);
        }
    }

    #[allow(dead_code)]
    pub fn get_font_glyphs(
        &self,
        font_handle: ResourceHandle,
        text: &str,
    ) -> impl Iterator<Item = Option<&Glyph>> {
        let font = self
            .resource_pool
            .get_font(font_handle)
            .expect("Fialed to get animation to sample");

        font.get_glyphs(text)
    }

    pub fn submit<T: SubmitJob>(&mut self, job: &T) {
        self.render_data.submit(job, &self.resource_pool);
    }
}
