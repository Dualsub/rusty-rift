use shared::{math::*, transform::Transform};
use std::ops::Range;
use std::sync::Arc;
use wgpu::BufferUsages;
use winit::window::Window;

use crate::renderer::{
    Buffer, BufferDesc, MaterialInstanceDesc, MaterialPipeline, MaterialPipelineDesc, RenderDevice,
    Resource, ResourceId, ResourcePool, ResourcePoolType, StaticMeshVertex, Texture, TextureDesc,
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
struct InstanceData {
    model_matrix: Mat4Data,
}

struct RenderJob {
    transform: Mat4,
    material: ResourceId,
    mesh: ResourceId,
}

struct FrameData<'a> {
    static_mesh_batches: &'a [RenderBatch],
}

struct RenderBatch {
    pub material_instance_id: ResourceId,
    pub mesh_id: ResourceId,
    pub instance_range: Range<u32>,
}

pub struct Renderer {
    render_device: RenderDevice,
    resource_pool: ResourcePool,
    depth_buffer: Texture,
    _depth_sampler: wgpu::Sampler,
    _default_sampler: wgpu::Sampler,
    shadow_map: Texture,
    shadow_material_pipeline_id: ResourceId,
    shadow_material_instance_id: ResourceId,

    camera_transform: Transform,
    uniform_buffer: Buffer,
    uniform_data: UniformBufferData,
    instance_buffer: Buffer,
    instance_data: Vec<InstanceData>,
    // Temporary for dev
    material_pipeline_id: ResourceId,
    material_instance_id: ResourceId,
    mesh_id: ResourceId,
    ground_mesh_id: ResourceId,
    _texture_id: ResourceId,
}

impl Renderer {
    const SHADOW_MAP_WIDTH: u32 = 2048;
    const SHADOW_MAP_HEIGHT: u32 = 2048;

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
        let mut resource_pool = ResourcePool::new();

        let default_sampler = render_device
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
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

        const INSTANCE_COUNT: usize = 16;
        const DISTANCE: f32 = 256.0;

        let mut instance_data: Vec<InstanceData> = Default::default();
        instance_data.reserve(INSTANCE_COUNT);
        for i in 0..INSTANCE_COUNT {
            let xi = i as i32 / 4i32;
            let zi = i as i32 % 4i32;

            let position = Vec3 {
                x: (xi as f32 * DISTANCE),
                y: 0.0,
                z: (zi as f32 * DISTANCE),
            };

            log::log!(log::Level::Info, "{:?}", position);

            instance_data.push(InstanceData {
                model_matrix: Mat4::from_translation(position).to_data(),
            });
        }

        let instance_buffer = render_device.create_buffer(&BufferDesc {
            size: INSTANCE_COUNT * 16 * std::mem::size_of::<f32>(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let uniform_buffer = render_device.create_buffer(&BufferDesc {
            size: std::mem::size_of::<UniformBufferData>(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = [
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
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ];

        let shadow_shader =
            render_device
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../res/shaders/shadow.wgsl").into(),
                    ),
                });

        let shadow_material_pipeline =
            render_device.create_material_pipeline(&MaterialPipelineDesc {
                vertex_shader: &shadow_shader,
                fragment_shader: None,
                bind_group_layouts: &[],
                layout_entries: &bind_group_layout[0..2], // We only need the first two bindings
                vertex_layout: &StaticMeshVertex::desc(),
                push_contant_ranges: &[],
            });

        let shadow_material_instance = render_device.create_material_instance(
            &shadow_material_pipeline,
            &MaterialInstanceDesc {
                entires: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: instance_buffer.buffer.as_entire_binding(),
                    },
                ],
            },
        );

        let shader = render_device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../res/shaders/triangle.wgsl").into(),
                ),
            });

        let material_pipeline = render_device.create_material_pipeline(&MaterialPipelineDesc {
            bind_group_layouts: &[],
            push_contant_ranges: &[],
            vertex_shader: &shader,
            fragment_shader: Some(&shader),
            layout_entries: &bind_group_layout,
            vertex_layout: &StaticMeshVertex::desc(),
        });

        let mesh = render_device.load_mesh(include_bytes!("../../../assets/models/test.dat"))?;
        let ground_mesh =
            render_device.load_mesh(include_bytes!("../../../assets/models/floor.dat"))?;

        let texture =
            render_device.load_texture(include_bytes!("../../../assets/textures/grid.dat"))?;

        let material_instance = render_device.create_material_instance(
            &material_pipeline,
            &MaterialInstanceDesc {
                entires: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: instance_buffer.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&default_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&depth_sampler),
                    },
                ],
            },
        );

        let shadow_material_pipeline_id = resource_pool.add_resource(
            Resource::MaterialPipeline(shadow_material_pipeline),
            ResourcePoolType::Scene,
        );

        let shadow_material_instance_id = resource_pool.add_resource(
            Resource::MaterialInstance(shadow_material_instance),
            ResourcePoolType::Scene,
        );

        let material_pipeline_id = resource_pool.add_resource(
            Resource::MaterialPipeline(material_pipeline),
            ResourcePoolType::Scene,
        );

        let material_instance_id = resource_pool.add_resource(
            Resource::MaterialInstance(material_instance),
            ResourcePoolType::Scene,
        );

        let texture_id =
            resource_pool.add_resource(Resource::Texture(texture), ResourcePoolType::Scene);

        let mesh_id = resource_pool.add_resource(Resource::Mesh(mesh), ResourcePoolType::Scene);
        let ground_mesh_id =
            resource_pool.add_resource(Resource::Mesh(ground_mesh), ResourcePoolType::Scene);

        Ok(Renderer {
            render_device,
            resource_pool,
            _default_sampler: default_sampler,
            depth_buffer,
            _depth_sampler: depth_sampler,
            shadow_map,
            shadow_material_pipeline_id,
            shadow_material_instance_id,
            camera_transform: Transform {
                position: Vec3 {
                    x: 0.0,
                    y: 400.0,
                    z: 0.0,
                },
                rotation: Quat::from_rotation_x(f32::to_radians(-30.0)),
                ..Default::default()
            },
            uniform_buffer,
            uniform_data: UniformBufferData {
                view_matrix: Mat4::IDENTITY.to_data(),
                projection_matrix: Mat4::IDENTITY.to_data(),
                camera_position: [0.0, 0.0, 0.0, 0.0],
                light_matrix: Mat4::IDENTITY.to_data(),
                light_direction: [0.0, -1.0, -1.0, 0.0],
                light_color: [1.0, 1.0, 1.0, 1.0],
            },
            instance_data,
            instance_buffer,
            material_pipeline_id,
            material_instance_id,
            mesh_id,
            ground_mesh_id,
            _texture_id: texture_id,
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

    fn draw_frame(&self) -> Result<(), wgpu::SurfaceError> {
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

            let batches = [
                RenderBatch {
                    mesh_id: self.mesh_id,
                    material_instance_id: self.shadow_material_instance_id,
                    instance_range: 0..(self.instance_data.len() - 1) as u32,
                },
                RenderBatch {
                    mesh_id: self.ground_mesh_id,
                    material_instance_id: self.shadow_material_instance_id,
                    instance_range: (self.instance_data.len() - 1) as u32
                        ..self.instance_data.len() as u32,
                },
            ];

            self.render_batches(
                &mut render_pass,
                self.shadow_material_pipeline_id,
                batches.as_slice(),
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
                            r: 1.0,
                            g: 0.0,
                            b: 1.0,
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

            let batches = [
                RenderBatch {
                    mesh_id: self.mesh_id,
                    material_instance_id: self.material_instance_id,
                    instance_range: 0..(self.instance_data.len() - 1) as u32,
                },
                RenderBatch {
                    mesh_id: self.ground_mesh_id,
                    material_instance_id: self.material_instance_id,
                    instance_range: (self.instance_data.len() - 1) as u32
                        ..self.instance_data.len() as u32,
                },
            ];

            self.render_batches(
                &mut render_pass,
                self.material_pipeline_id,
                batches.as_slice(),
            );
        }

        self.render_device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if !self.render_device.is_surface_configured {
            return Ok(());
        }

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

        self.render_device.write_buffer(
            &self.instance_buffer,
            bytemuck::cast_slice(self.instance_data.as_slice()),
            0,
        );

        self.draw_frame()
    }

    fn render_batches(
        &self,
        render_pass: &mut wgpu::RenderPass,
        material_pipeline_id: ResourceId,
        batches: &[RenderBatch],
    ) {
        let material_pipeline = self
            .resource_pool
            .get_material_pipeline(material_pipeline_id)
            .unwrap();

        render_pass.set_pipeline(&material_pipeline.pipeline);
        for batch in batches {
            let material_instance = self
                .resource_pool
                .get_material_instance(batch.material_instance_id)
                .unwrap(); // Could add a default material here maybe

            render_pass.set_bind_group(0, &material_instance.bindgroup, &[]);

            let mesh = self.resource_pool.get_mesh(batch.mesh_id).unwrap();
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.buffer.slice(..));
            render_pass.set_index_buffer(
                mesh.index_buffer.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );

            // We can clone the range, it is very small so it is fine
            render_pass.draw_indexed(0..mesh.index_count, 0, batch.instance_range.clone());
        }
    }

    pub fn set_camera_position_and_orientation(&mut self, position: Vec3, orientation: Quat) {
        self.camera_transform.position = position;
        self.camera_transform.rotation = orientation;
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
}
