use std::{collections::HashMap, ops::Range};

use shared::math::*;

use crate::renderer::{DrawData, ResourceIndex, StaticInstanceData, renderer::RenderBatch};

trait SubmitJob {
    fn submit(&self, render_data: &mut RenderData);
}

#[derive(Default)]
struct InstancedRenderJob<T> {
    instances: Vec<T>,
}

type StaticInstancedRenderJob = InstancedRenderJob<StaticInstanceData>;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default)]
struct BatchKey {
    material: ResourceIndex,
    mesh: ResourceIndex,
}

pub struct StaticRenderJob {
    pub transform: Mat4,
    pub material: ResourceIndex,
    pub mesh: ResourceIndex,
}

impl SubmitJob for StaticRenderJob {
    fn submit(&self, render_data: &mut RenderData) {
        let key = BatchKey {
            mesh: self.mesh,
            material: self.material,
        };

        if render_data.static_jobs.contains_key(&key) {
            let instanced_job = render_data.static_jobs.get_mut(&key).unwrap();
            instanced_job.instances.push(StaticInstanceData {
                model_matrix: self.transform.to_data(),
            });
        } else {
            render_data.static_jobs.insert(
                key,
                StaticInstancedRenderJob {
                    instances: vec![StaticInstanceData {
                        model_matrix: self.transform.to_data(),
                    }],
                },
            );
        }
    }
}

pub struct RenderData {
    static_jobs: HashMap<BatchKey, StaticInstancedRenderJob>,
}

impl RenderData {
    pub fn new() -> Self {
        Self {
            static_jobs: HashMap::new(),
        }
    }

    pub fn submit<T: SubmitJob>(&mut self, job: &T) {
        job.submit(self);
    }

    pub fn build_draw_data(&mut self) -> DrawData {
        let mut static_batches: Vec<RenderBatch> = Vec::new();
        let mut static_instances: Vec<StaticInstanceData> = Vec::new();

        for (key, job) in self.static_jobs.iter_mut() {
            let start = static_instances.len() as u32;
            static_instances.append(&mut job.instances);
            let end = static_instances.len() as u32;

            static_batches.push(RenderBatch {
                material_instance_id: key.material,
                mesh_id: key.mesh,
                instance_range: Range { start, end },
            });
        }

        DrawData {
            static_batches,
            static_instances,
        }
    }
}
