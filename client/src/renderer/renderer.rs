use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::renderer::{
    MaterialPipeline, RenderDevice, StaticMesh, StaticMeshVertex, Texture,
    material::{MaterialInstance, MaterialInstanceDesc, MaterialPipelineDesc},
};

pub struct Renderer {
    pub render_device: RenderDevice,
    pub _default_sampler: wgpu::Sampler,
    // Temporary for dev
    pub material_pipeline: MaterialPipeline,
    pub material_instance: MaterialInstance,
    pub mesh: StaticMesh,
    pub _texture: Texture,
}

impl Renderer {
    pub async fn new(window: &Arc<Window>) -> anyhow::Result<Renderer> {
        let render_device = RenderDevice::new(&window).await?;

        let default_sampler = render_device
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
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
        });

        const VERTICES: &[StaticMeshVertex] = &[
            StaticMeshVertex {
                position: [-0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [0.0, 0.0, 0.0],
                color: [1.0, 0.0, 0.0, 1.0],
            }, // bottom-left
            StaticMeshVertex {
                position: [0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [1.0, 0.0, 0.0],
                color: [0.0, 1.0, 0.0, 1.0],
            }, // bottom-right
            StaticMeshVertex {
                position: [0.5, 0.5, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [1.0, 1.0, 0.0],
                color: [0.0, 0.0, 1.0, 1.0],
            }, // top-right
            StaticMeshVertex {
                position: [-0.5, 0.5, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [0.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            }, // top-left
        ];

        const INDICES: &[u32] = &[
            0, 1, 2, // first triangle
            0, 2, 3, // second triangle
        ];

        let mesh = StaticMesh {
            vertex_buffer: render_device.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ),
            index_buffer: render_device.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(INDICES),
                    usage: wgpu::BufferUsages::INDEX,
                },
            ),
            index_count: INDICES.len() as u32,
        };

        let texture =
            render_device.load_texture(include_bytes!("../../../assets/textures/grid.dat"))?;

        let material_instance = render_device.create_material_instance(
            &material_pipeline,
            &MaterialInstanceDesc {
                entires: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&default_sampler),
                    },
                ],
            },
        );

        Ok(Renderer {
            render_device,
            _default_sampler: default_sampler,
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
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let render_device = &mut self.render_device;

        if !render_device.is_surface_configured {
            return Ok(());
        }

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
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.material_pipeline.pipeline);
            render_pass.set_bind_group(0, &self.material_instance.bindgroup, &[]);
            render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.mesh.index_count, 0, 0..1);
        }

        render_device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
