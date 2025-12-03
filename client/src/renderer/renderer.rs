use std::sync::Arc;
use wgpu::{BufferUsages, util::DeviceExt};
use winit::window::Window;

use crate::renderer::{
    BufferDesc, MaterialPipeline, RenderDevice, StaticMesh, StaticMeshVertex, Texture,
    buffer::Buffer,
    material::{MaterialInstance, MaterialInstanceDesc, MaterialPipelineDesc},
    mesh::MeshLoadDesc,
    texture::TextureDesc,
};

#[repr(C)]
// This is so we can store this in a buffer
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformBufferData {
    model_matrix: [[f32; 4]; 4],
    view_matrix: [[f32; 4]; 4],
    projection_matrix: [[f32; 4]; 4],
}

pub struct Renderer {
    pub render_device: RenderDevice,
    pub depth_buffer: Texture,
    pub _depth_sampler: wgpu::Sampler,
    pub _default_sampler: wgpu::Sampler,
    pub uniform_buffer: Buffer,
    pub uniform_data: UniformBufferData,
    // Temporary for dev
    pub material_pipeline: MaterialPipeline,
    pub material_instance: MaterialInstance,
    pub mesh: StaticMesh,
    pub _texture: Texture,
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
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&default_sampler),
                    },
                ],
            },
        );

        Ok(Renderer {
            render_device,
            _default_sampler: default_sampler,
            depth_buffer,
            _depth_sampler: depth_sampler,
            uniform_buffer,
            uniform_data: UniformBufferData {
                model_matrix: glam::Mat4::IDENTITY.to_cols_array_2d(),
                view_matrix: glam::Mat4::IDENTITY.to_cols_array_2d(),
                projection_matrix: glam::Mat4::IDENTITY.to_cols_array_2d(),
            },
            material_pipeline,
            material_instance,
            mesh,
            _texture: texture,
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
        let projection_matrix = glam::Mat4::perspective_rh(
            f32::to_radians(60.0),
            render_device.config.width as f32 / render_device.config.height as f32,
            0.1,
            2000.0,
        );
        self.uniform_data.projection_matrix = projection_matrix.to_cols_array_2d();

        let view_matrix = glam::Mat4::from_translation(glam::Vec3 {
            x: 0.0,
            y: -100.0,
            z: -300.0,
        });
        self.uniform_data.view_matrix = view_matrix.to_cols_array_2d();

        let mut model_matrix = glam::Mat4::from_cols_array_2d(&self.uniform_data.model_matrix);
        model_matrix *= glam::Mat4::from_rotation_y(f32::to_radians(1.0));
        self.uniform_data.model_matrix = model_matrix.to_cols_array_2d();

        render_device.write_buffer(
            &self.uniform_buffer,
            bytemuck::cast_slice(&[self.uniform_data]),
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

            render_pass.set_pipeline(&self.material_pipeline.pipeline);
            render_pass.set_bind_group(0, &self.material_instance.bindgroup, &[]);
            render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.buffer.slice(..));
            render_pass.set_index_buffer(
                self.mesh.index_buffer.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..self.mesh.index_count, 0, 0..1);
        }

        render_device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
