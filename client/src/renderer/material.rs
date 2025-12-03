use crate::renderer::RenderDevice;

pub struct MaterialPipelineDesc<'a> {
    pub vertex_shader: &'a wgpu::ShaderModule,
    pub fragment_shader: &'a wgpu::ShaderModule,
    pub bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
    pub layout_entries: &'a [wgpu::BindGroupLayoutEntry],
    pub vertex_layout: &'a wgpu::VertexBufferLayout<'static>,
    pub push_contant_ranges: &'a [wgpu::PushConstantRange],
}

pub struct MaterialPipeline {
    pub _pipeline_layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub bindgroup_layout: wgpu::BindGroupLayout,
}

impl MaterialPipeline {}

impl RenderDevice {
    pub fn create_material_pipeline(&self, desc: &MaterialPipelineDesc) -> MaterialPipeline {
        let bindgroup_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: desc.layout_entries,
                });

        let mut bind_group_layouts = desc.bind_group_layouts.to_vec();
        bind_group_layouts.push(&bindgroup_layout);

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bind_group_layouts.as_slice(),
                push_constant_ranges: desc.push_contant_ranges,
            });

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
                fragment: Some(wgpu::FragmentState {
                    module: desc.fragment_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.config.format,
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

        MaterialPipeline {
            _pipeline_layout: pipeline_layout,
            pipeline,
            bindgroup_layout,
        }
    }
}

pub struct MaterialInstanceDesc<'a> {
    pub entires: &'a [wgpu::BindGroupEntry<'a>],
}

pub struct MaterialInstance {
    pub bindgroup: wgpu::BindGroup,
}

impl RenderDevice {
    pub fn create_material_instance(
        &self,
        pipeline: &MaterialPipeline,
        desc: &MaterialInstanceDesc,
    ) -> MaterialInstance {
        let bindgroup = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.bindgroup_layout,
            entries: desc.entires,
        });

        MaterialInstance { bindgroup }
    }
}
