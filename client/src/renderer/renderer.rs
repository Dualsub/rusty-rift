use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::renderer::{MaterialPipeline, RenderDevice, StaticMesh, StaticMeshVertex};

pub struct Renderer {
    pub render_device: RenderDevice,
    // Temporary
    pub pipeline: MaterialPipeline,
    pub mesh: StaticMesh,
}

impl Renderer {
    pub async fn new(window: &Arc<Window>) -> anyhow::Result<Renderer> {
        let render_device = RenderDevice::new(&window).await?;

        let shader = render_device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../res/shaders/triangle.wgsl").into(),
                ),
            });

        let pipeline_layout =
            render_device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let pipeline =
            render_device
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[StaticMeshVertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: render_device.config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        unclipped_depth: false,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                    cache: None,
                });

        let material_pipeline = MaterialPipeline {
            pipeline,
            pipeline_layout,
        };

        const VERTICES: &[StaticMeshVertex] = &[
            StaticMeshVertex {
                position: [-0.0868241, 0.49240386, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [0.0, 0.0, 0.0],
                color: [0.0, 0.5, 0.5, 1.0],
            }, // A
            StaticMeshVertex {
                position: [-0.49513406, 0.06958647, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [0.0, 0.0, 0.0],
                color: [0.5, 0.5, 0.0, 1.0],
            }, // B
            StaticMeshVertex {
                position: [-0.21918549, -0.44939706, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [0.0, 0.0, 0.0],
                color: [0.5, 0.5, 0.5, 1.0],
            }, // C
            StaticMeshVertex {
                position: [0.35966998, -0.3473291, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 0.5, 1.0],
            }, // D
            StaticMeshVertex {
                position: [0.44147372, 0.2347359, 0.0],
                normal: [0.0, 0.0, 0.0],
                uvs: [0.0, 0.0, 0.0],
                color: [0.5, 0.0, 0.0, 1.0],
            }, // E
        ];

        const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

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

        Ok(Renderer {
            render_device,
            pipeline: material_pipeline,
            mesh: mesh,
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

            render_pass.set_pipeline(&self.pipeline.pipeline);
            render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.mesh.index_count, 0, 0..1);
        }

        render_device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
