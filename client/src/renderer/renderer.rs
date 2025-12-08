use shared::{math::*, transform::Transform};
use std::ops::Range;
use std::sync::Arc;
use wgpu::BufferUsages;
use winit::window::Window;

use crate::renderer::{
    Buffer, BufferDesc, MaterialInstanceDesc, MaterialPipeline, MaterialPipelineDesc, RenderData,
    RenderDevice, Resource, ResourceHandle, ResourcePool, SkeletalMeshVertex, StaticInstanceData,
    StaticMeshVertex, Texture, TextureDesc,
    render_data::{StaticRenderJob, SubmitJob},
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
}

// A short-term abstraction
pub struct MaterialGroup {
    static_material_pipeline: MaterialPipeline,
    skeletal_material_pipeline: MaterialPipeline,
}

pub struct Renderer {
    render_device: RenderDevice,
    resource_pool: ResourcePool,
    depth_buffer: Texture,
    _depth_sampler: wgpu::Sampler,
    default_sampler: wgpu::Sampler,

    _scene_bind_group_layout: wgpu::BindGroupLayout,
    static_scene_bind_group: wgpu::BindGroup,
    skeletal_scene_bind_group: wgpu::BindGroup,

    shadow_map: Texture,
    _shadow_bind_group_layout: wgpu::BindGroupLayout,
    static_shadow_bind_group: wgpu::BindGroup,
    skeletal_shadow_bind_group: wgpu::BindGroup,
    shadow_material_pipeline: MaterialGroup,

    scene_material_pipeline: MaterialGroup,

    camera_transform: Transform,
    uniform_buffer: Buffer,
    uniform_data: UniformBufferData,

    static_instance_buffer: Buffer,
    skeletal_instance_buffer: Buffer,

    render_data: RenderData,
}

impl Renderer {
    const SHADOW_MAP_WIDTH: u32 = 2048;
    const SHADOW_MAP_HEIGHT: u32 = 2048;

    const STATIC_INSTANCE_COUNT: usize = 512;

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

    pub async fn new(window: &Arc<Window>) -> anyhow::Result<Renderer> {
        let render_device = RenderDevice::new(&window).await?;
        let resource_pool = ResourcePool::new();

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

        let depth_buffer = Renderer::create_depth_buffer(&render_device);

        let shadow_map = render_device.create_texture(&TextureDesc {
            width: Self::SHADOW_MAP_WIDTH,
            height: Self::SHADOW_MAP_HEIGHT,
            layer_count: 1,
            format: Some(wgpu::TextureFormat::Depth32Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_dimension: wgpu::TextureViewDimension::D2,
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });

        let static_instance_buffer = render_device.create_buffer(&BufferDesc {
            size: Self::STATIC_INSTANCE_COUNT * std::mem::size_of::<StaticInstanceData>(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let skeletal_instance_buffer = render_device.create_buffer(&BufferDesc {
            size: Self::STATIC_INSTANCE_COUNT * std::mem::size_of::<StaticInstanceData>(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let uniform_buffer = render_device.create_buffer(&BufferDesc {
            size: std::mem::size_of::<UniformBufferData>(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let scene_bind_group_layout =
            render_device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Scene"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Depth,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                            count: None,
                        },
                    ],
                });

        let shadow_bind_group_layout =
            render_device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Shadow"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let static_shadow_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/static_shadow.wgsl").into(),
                    ),
                });

        let skeletal_shadow_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/skeletal_shadow.wgsl").into(),
                    ),
                });

        let shadow_material_pipeline = MaterialGroup {
            static_material_pipeline: render_device.create_material_pipeline(
                &MaterialPipelineDesc {
                    vertex_shader: &static_shadow_shader,
                    fragment_shader: None,
                    bind_group_layouts: &[&shadow_bind_group_layout],
                    layout_entries: &[],
                    vertex_layout: &StaticMeshVertex::desc(),
                    push_contant_ranges: &[],
                },
            ),
            skeletal_material_pipeline: render_device.create_material_pipeline(
                &MaterialPipelineDesc {
                    vertex_shader: &skeletal_shadow_shader,
                    fragment_shader: None,
                    bind_group_layouts: &[&shadow_bind_group_layout],
                    layout_entries: &[],
                    vertex_layout: &SkeletalMeshVertex::desc(),
                    push_contant_ranges: &[],
                },
            ),
        };

        let static_vertex_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/static.wgsl").into(),
                    ),
                });
        let skeletal_vertex_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/skeletal.wgsl").into(),
                    ),
                });
        let fragment_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/scene.wgsl").into(),
                    ),
                });

        let scene_material_pipeline = MaterialGroup {
            static_material_pipeline: render_device.create_material_pipeline(
                &MaterialPipelineDesc {
                    bind_group_layouts: &[&scene_bind_group_layout],
                    push_contant_ranges: &[],
                    vertex_shader: &static_vertex_shader,
                    fragment_shader: Some(&fragment_shader),
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
                },
            ),
            skeletal_material_pipeline: render_device.create_material_pipeline(
                &MaterialPipelineDesc {
                    bind_group_layouts: &[&scene_bind_group_layout],
                    push_contant_ranges: &[],
                    vertex_shader: &skeletal_vertex_shader,
                    fragment_shader: Some(&fragment_shader),
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
                    vertex_layout: &SkeletalMeshVertex::desc(),
                },
            ),
        };

        let static_scene_bind_group =
            render_device
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &scene_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: static_instance_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&depth_sampler),
                        },
                    ],
                });

        let skeletal_scene_bind_group =
            render_device
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &scene_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: skeletal_instance_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&depth_sampler),
                        },
                    ],
                });

        let static_shadow_bind_group =
            render_device
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &shadow_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: static_instance_buffer.buffer.as_entire_binding(),
                        },
                    ],
                });

        let skeletal_shadow_bind_group =
            render_device
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &shadow_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: skeletal_instance_buffer.buffer.as_entire_binding(),
                        },
                    ],
                });

        Ok(Renderer {
            render_device,
            resource_pool,
            default_sampler,
            depth_buffer,
            _depth_sampler: depth_sampler,
            _scene_bind_group_layout: scene_bind_group_layout,
            static_scene_bind_group,
            skeletal_scene_bind_group,
            shadow_map,
            shadow_material_pipeline,
            static_shadow_bind_group,
            skeletal_shadow_bind_group,
            _shadow_bind_group_layout: shadow_bind_group_layout,
            camera_transform: Transform {
                position: Vec3 {
                    x: 0.0,
                    y: 400.0,
                    z: 0.0,
                },
                rotation: Quat::from_rotation_x(f32::to_radians(-30.0)),
                ..Default::default()
            },
            render_data: RenderData::new(),
            uniform_buffer,
            uniform_data: UniformBufferData {
                view_matrix: Mat4::IDENTITY.to_data(),
                projection_matrix: Mat4::IDENTITY.to_data(),
                camera_position: [0.0, 0.0, 0.0, 0.0],
                light_matrix: Mat4::IDENTITY.to_data(),
                light_direction: [0.0, -1.0, -1.0, 0.0],
                light_color: [1.0, 1.0, 1.0, 1.0],
            },
            static_instance_buffer,
            skeletal_instance_buffer,
            scene_material_pipeline,
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

            self.depth_buffer = Renderer::create_depth_buffer(&render_device);
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
        let projection_matrix = Mat4::perspective_rh(
            f32::to_radians(40.0),
            self.render_device.config.width as f32 / self.render_device.config.height as f32,
            1.0,
            2000.0,
        );
        self.uniform_data.projection_matrix = projection_matrix.to_data();

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
            projection_matrix,
            Vec3::from_slice(&self.uniform_data.light_direction),
        )
        .to_data();

        self.render_device.write_buffer(
            &self.uniform_buffer,
            bytemuck::bytes_of(&self.uniform_data),
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
                &[&self.static_shadow_bind_group],
                &draw_data.static_batches,
            );

            self.render_batches(
                &mut render_pass,
                &self.shadow_material_pipeline.skeletal_material_pipeline,
                &[&self.skeletal_shadow_bind_group],
                &draw_data.skeletal_batches,
            );
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scene Pass"),
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
                &[&self.static_scene_bind_group],
                &draw_data.static_batches,
            );

            self.render_batches(
                &mut render_pass,
                &self.scene_material_pipeline.skeletal_material_pipeline,
                &[&self.skeletal_scene_bind_group],
                &draw_data.skeletal_batches,
            );
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

    pub fn set_lighting_color(&mut self, color: Vec3) {
        self.uniform_data.light_color = [color.x, color.y, color.z, 1.0];
    }

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

    pub fn create_material(&mut self, name: &'static str, texture_bytes: &[u8]) -> ResourceHandle {
        let handle = get_handle(name);
        let texture = self
            .render_device
            .load_texture(texture_bytes)
            .expect("Failed to load texture");

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

    pub fn submit<T: SubmitJob>(&mut self, job: &T) {
        self.render_data.submit(job);
    }
}
