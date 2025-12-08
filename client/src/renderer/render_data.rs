use std::{collections::HashMap, ops::Range};

use shared::math::*;

use crate::renderer::{DrawData, ResourceHandle, StaticInstanceData, renderer::RenderBatch};

pub trait SubmitJob {
    fn submit(&self, render_data: &mut RenderData);
}

#[derive(Default)]
struct InstancedRenderJob<T> {
    instances: Vec<T>,
}

type StaticInstancedRenderJob = InstancedRenderJob<StaticInstanceData>;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default)]
struct BatchKey {
    material: ResourceHandle,
    mesh: ResourceHandle,
}

pub struct StaticRenderJob {
    pub transform: Mat4,
    pub material: ResourceHandle,
    pub mesh: ResourceHandle,
    pub color: Vec4,
    pub tex_coord: Vec2,
    pub tex_scale: Vec2,
}

impl Default for StaticRenderJob {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY,
            material: 0,
            mesh: 0,
            color: Vec4::ONE,
            tex_coord: Vec2::ZERO,
            tex_scale: Vec2::ONE,
        }
    }
}

impl SubmitJob for StaticRenderJob {
    fn submit(&self, render_data: &mut RenderData) {
        let key = BatchKey {
            mesh: self.mesh,
            material: self.material,
        };

        let instanced_job = render_data.static_jobs.entry(key).or_default();
        instanced_job.instances.push(StaticInstanceData {
            model_matrix: self.transform.to_data(),
            color: self.color.to_data(),
            tex_coord: self.tex_coord.to_data(),
            tex_scale: self.tex_scale.to_data(),
        });
    }
}

pub struct SkeletalRenderJob {
    pub transform: Mat4,
    pub material: ResourceHandle,
    pub mesh: ResourceHandle,
    pub color: Vec4,
    pub tex_coord: Vec2,
    pub tex_scale: Vec2,
}

impl Default for SkeletalRenderJob {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY,
            material: 0,
            mesh: 0,
            color: Vec4::ONE,
            tex_coord: Vec2::ZERO,
            tex_scale: Vec2::ONE,
        }
    }
}

impl SubmitJob for SkeletalRenderJob {
    fn submit(&self, render_data: &mut RenderData) {
        let key = BatchKey {
            mesh: self.mesh,
            material: self.material,
        };

        let instanced_job = render_data.skeletal_jobs.entry(key).or_default();
        instanced_job.instances.push(StaticInstanceData {
            model_matrix: self.transform.to_data(),
            color: self.color.to_data(),
            tex_coord: self.tex_coord.to_data(),
            tex_scale: self.tex_scale.to_data(),
        });
    }
}

type JobMap = HashMap<BatchKey, StaticInstancedRenderJob>;

pub struct RenderData {
    static_jobs: JobMap,
    skeletal_jobs: JobMap,
}

impl RenderData {
    pub fn new() -> Self {
        Self {
            static_jobs: HashMap::new(),
            skeletal_jobs: HashMap::new(),
        }
    }

    pub fn submit<T: SubmitJob>(&mut self, job: &T) {
        job.submit(self);
    }

    // NOTE: The instance data from the jobs is moved into the draw data when built, so
    // the jobs stay allocated and the instance vectors are not reallocated every frame.
    // They can however be explicitly reset with the reset method.

    fn build_batches(jobs: &mut JobMap) -> (Vec<RenderBatch>, Vec<StaticInstanceData>) {
        let batch_count = jobs.len();
        let instance_count = jobs.iter().map(|(_, job)| job.instances.len()).sum();

        let mut batches: Vec<RenderBatch> = Vec::with_capacity(batch_count);
        let mut instances: Vec<StaticInstanceData> = Vec::with_capacity(instance_count);

        for (key, job) in jobs.iter_mut() {
            let start = instances.len() as u32;

            // Instances are moved
            instances.append(&mut job.instances);

            let end = instances.len() as u32;

            batches.push(RenderBatch {
                material_instance: key.material,
                mesh: key.mesh,
                instance_range: Range { start, end },
            });
        }

        (batches, instances)
    }

    pub fn build_draw_data(&mut self) -> DrawData {
        let (static_batches, static_instances) = Self::build_batches(&mut self.static_jobs);
        let (skeletal_batches, skeletal_instances) = Self::build_batches(&mut self.skeletal_jobs);

        DrawData {
            static_batches,
            static_instances,
            skeletal_batches,
            skeletal_instances,
        }
    }

    pub fn reset(&mut self) {
        self.static_jobs.clear();
    }
}
