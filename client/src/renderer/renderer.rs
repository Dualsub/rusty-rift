use shared::math::*;
use std::sync::Arc;
use wgpu::BufferUsages;
use winit::window::Window;

use crate::renderer::{
    Buffer, BufferDesc, MaterialInstanceDesc, MaterialPipelineDesc, RenderDevice, Resource,
    ResourceId, ResourcePool, ResourcePoolType, StaticMeshVertex, Texture, TextureDesc,
};

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformBufferData {
    view_matrix: Mat4Data,
    projection_matrix: Mat4Data,
    camera_position: Vec4Data,
    light_direction: Vec4Data,
    light_color: Vec4Data,
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    model_matrix: Mat4Data,
}

pub struct Renderer {
    render_device: RenderDevice,
    resource_pool: ResourcePool,
    depth_buffer: Texture,
    _depth_sampler: wgpu::Sampler,
    _default_sampler: wgpu::Sampler,
    uniform_buffer: Buffer,
    uniform_data: UniformBufferData,
    instance_buffer: Buffer,
    instance_data: Vec<InstanceData>,
    // Temporary for dev
    material_pipeline_id: ResourceId,
    material_instance_id: ResourceId,
    mesh_id: ResourceId,
    _texture_id: ResourceId,
}

impl Renderer {
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
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
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
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            });

        let depth_buffer = Renderer::create_depth_buffer(&render_device);

        const INSTANCE_COUNT: usize = 256;
        const DISTANCE: f32 = 256.0;

        let mut instance_data: Vec<InstanceData> = Default::default();
        instance_data.reserve(INSTANCE_COUNT);
        for i in 0..INSTANCE_COUNT {
            let xi = (i as i32 / 8i32) - 4;
            let zi = (i as i32 % 8i32) - 4;

            instance_data.push(InstanceData {
                model_matrix: Mat4::from_translation(Vec3 {
                    x: (xi as f32 * DISTANCE),
                    y: 0.0,
                    z: (zi as f32 * DISTANCE),
                })
                .to_data(),
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
            fragment_shader: &shader,
            layout_entries: &[
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
            ],
            vertex_layout: &StaticMeshVertex::desc(),
        });

        let mesh = render_device.load_mesh(include_bytes!("../../../assets/models/test.dat"))?;

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
                ],
            },
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

        Ok(Renderer {
            render_device,
            resource_pool,
            _default_sampler: default_sampler,
            depth_buffer,
            _depth_sampler: depth_sampler,
            uniform_buffer,
            uniform_data: UniformBufferData {
                view_matrix: Mat4::IDENTITY.to_data(),
                projection_matrix: Mat4::IDENTITY.to_data(),
                camera_position: [0.0, 0.0, 0.0, 0.0],
                light_direction: [0.01, -1.0, 0.01, 0.0],
                light_color: [1.0, 1.0, 1.0, 1.0],
            },
            instance_data,
            instance_buffer,
            material_pipeline_id,
            material_instance_id,
            mesh_id,
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

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let render_device = &mut self.render_device;

        if !render_device.is_surface_configured {
            return Ok(());
        }

        // Update data
        let camera_position = Vec3 {
            x: 0.0,
            y: 200.0,
            z: 600.0,
        };

        let projection_matrix = Mat4::perspective_rh(
            f32::to_radians(60.0),
            render_device.config.width as f32 / render_device.config.height as f32,
            0.1,
            2000.0,
        );
        self.uniform_data.projection_matrix = projection_matrix.to_data();

        let view_matrix = Mat4::from_translation(camera_position).inverse();
        self.uniform_data.view_matrix = view_matrix.to_data();
        self.uniform_data.camera_position =
            [camera_position.x, camera_position.y, camera_position.z, 0.0];

        render_device.write_buffer(
            &self.uniform_buffer,
            bytemuck::bytes_of(&self.uniform_data),
            0,
        );

        for i in 0..self.instance_data.len() {
            let mut model_matrix = Mat4::from_cols_array(&self.instance_data[i].model_matrix);
            model_matrix *= Mat4::from_rotation_y(f32::to_radians(0.0));
            self.instance_data[i].model_matrix = model_matrix.to_data();
        }

        render_device.write_buffer(
            &self.instance_buffer,
            bytemuck::cast_slice(self.instance_data.as_slice()),
            0,
        );

        // Render
        let output = render_device.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            render_device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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

            let material_pipeline = self
                .resource_pool
                .get_material_pipeline(self.material_pipeline_id)
                .unwrap();

            let material_instance = self
                .resource_pool
                .get_material_instance(self.material_instance_id)
                .unwrap();

            let mesh = self.resource_pool.get_mesh(self.mesh_id).unwrap();

            render_pass.set_pipeline(&material_pipeline.pipeline);
            render_pass.set_bind_group(0, &material_instance.bindgroup, &[]);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.buffer.slice(..));
            render_pass.set_index_buffer(
                mesh.index_buffer.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..mesh.index_count, 0, 0..self.instance_data.len() as u32);
        }

        render_device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
