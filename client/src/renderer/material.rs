use wgpu::DepthStencilState;

use crate::renderer::RenderDevice;

pub struct MaterialPipelineDesc<'a> {
    pub vertex_shader: &'a wgpu::ShaderModule,
    pub fragment_shader: Option<&'a wgpu::ShaderModule>,
    pub bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
    pub layout_entries: &'a [wgpu::BindGroupLayoutEntry],
    pub vertex_layout: &'a wgpu::VertexBufferLayout<'static>,
    pub push_contant_ranges: &'a [wgpu::PushConstantRange],
    pub pass_target: PassTarget,
}

pub enum PassTarget {
    Scene,
    Composite,
}

impl Default for PassTarget {
    fn default() -> Self {
        Self::Scene
    }
}

pub struct MaterialPipeline {
    pub _pipeline_layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub bindgroup_layout: Option<wgpu::BindGroupLayout>,
}

impl MaterialPipeline {}

impl RenderDevice {
    pub fn create_material_pipeline(&self, desc: &MaterialPipelineDesc) -> MaterialPipeline {
        let mut bind_group_layouts = desc.bind_group_layouts.to_vec();
        let mut extra_bind_group_layout: Option<wgpu::BindGroupLayout> = None;

        if !desc.layout_entries.is_empty() {
            let layout = self
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: desc.layout_entries,
                });

            extra_bind_group_layout = Some(layout.clone());
            bind_group_layouts.push(extra_bind_group_layout.as_ref().unwrap());
        }

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bind_group_layouts.as_slice(),
                push_constant_ranges: desc.push_contant_ranges,
            });

        const SCENE_COLOR_TARGETS: [Option<wgpu::ColorTargetState>; 1] =
            [Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })];

        let composite_color_targets = [Some(wgpu::ColorTargetState {
            format: self.config.format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let default_depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: desc.vertex_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[desc.vertex_layout.clone()],
                },
                fragment: match desc.fragment_shader {
                    Some(fragment_shader) => Some(wgpu::FragmentState {
                        module: fragment_shader,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: match desc.pass_target {
                            PassTarget::Scene => &SCENE_COLOR_TARGETS,
                            PassTarget::Composite => &composite_color_targets,
                        },
                    }),
                    None => None,
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: match desc.pass_target {
                        PassTarget::Scene => Some(wgpu::Face::Back),
                        PassTarget::Composite => None,
                    },
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: match desc.pass_target {
                    PassTarget::Scene => Some(default_depth_stencil),
                    PassTarget::Composite => None,
                },
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });

        MaterialPipeline {
            _pipeline_layout: pipeline_layout,
            pipeline,
            bindgroup_layout: extra_bind_group_layout,
        }
    }
}

pub struct MaterialInstanceDesc<'a> {
    pub entires: &'a [wgpu::BindGroupEntry<'a>],
}

pub struct MaterialInstance {
    pub bind_group: wgpu::BindGroup,
}

impl RenderDevice {
    pub fn create_material_instance(
        &self,
        pipeline: &MaterialPipeline,
        desc: &MaterialInstanceDesc,
    ) -> MaterialInstance {
        let bindgroup = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.bindgroup_layout.as_ref().unwrap(), // We should not be creating a material instance of no layout
            entries: desc.entires,
        });

        MaterialInstance {
            bind_group: bindgroup,
        }
    }
}
