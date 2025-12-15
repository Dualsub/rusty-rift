use std::{collections::HashMap, ops::Range};

use shared::math::*;

use crate::renderer::{
    DrawData, Renderer, ResourceHandle, ResourcePool, SpriteInstanceData, StaticInstanceData,
    animation::Pose, renderer::RenderBatch,
};

pub trait SubmitJob {
    fn submit(&self, render_data: &mut RenderData, resource_pool: &ResourcePool);
}

#[derive(Default)]
struct InstancedRenderJob<T> {
    instances: Vec<T>,
}

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
    fn submit(&self, render_data: &mut RenderData, _resource_pool: &ResourcePool) {
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
            ..Default::default()
        });
    }
}

#[allow(dead_code)]
pub struct SkeletalRenderJob<'a> {
    pub transform: Mat4,
    pub material: ResourceHandle,
    pub mesh: ResourceHandle,
    pub color: Vec4,
    pub tex_coord: Vec2,
    pub tex_scale: Vec2,
    pub pose: Option<&'a Pose>,
}

impl Default for SkeletalRenderJob<'_> {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY,
            material: 0,
            mesh: 0,
            color: Vec4::ONE,
            tex_coord: Vec2::ZERO,
            tex_scale: Vec2::ONE,
            pose: None,
        }
    }
}

impl SubmitJob for SkeletalRenderJob<'_> {
    fn submit(&self, render_data: &mut RenderData, _resource_pool: &ResourcePool) {
        let key = BatchKey {
            mesh: self.mesh,
            material: self.material,
        };

        let pose = self.pose.expect("Pose was None");

        let bone_index = render_data.bones.len();
        let bone_count = pose.transforms.len();
        // Allocate the new bones
        render_data
            .bones
            .resize(bone_index + bone_count, Mat4::IDENTITY.to_data());

        let mesh = _resource_pool
            .get_skeletal_mesh(self.mesh)
            .expect("Skeletel mesh was not found");

        // Fill them with the global matrices from the pose
        mesh.get_bone_matrices(
            pose,
            &mut render_data.bones[bone_index..bone_index + bone_count],
        );

        let instanced_job = render_data.skeletal_jobs.entry(key).or_default();
        instanced_job.instances.push(StaticInstanceData {
            model_matrix: self.transform.to_data(),
            color: self.color.to_data(),
            tex_coord: self.tex_coord.to_data(),
            tex_scale: self.tex_scale.to_data(),
            data_indices: [bone_index as u32, 0, 0, 0],
        });
    }
}

#[allow(dead_code)]
pub struct SpriteRenderJob {
    pub position: Vec2,
    pub size: Vec2,
    pub material: ResourceHandle,
    pub color: Vec4,
    pub tex_coord: Vec2,
    pub tex_scale: Vec2,
}

impl Default for SpriteRenderJob {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            size: Vec2::ONE,
            material: 0,
            color: Vec4::ONE,
            tex_coord: Vec2::ZERO,
            tex_scale: Vec2::ONE,
        }
    }
}

impl SubmitJob for SpriteRenderJob {
    fn submit(&self, render_data: &mut RenderData, _resource_pool: &ResourcePool) {
        let key = BatchKey {
            mesh: Renderer::QUAD_MESH,
            material: self.material,
        };

        let instanced_job = render_data.sprite_jobs.entry(key).or_default();
        instanced_job.instances.push(SpriteInstanceData {
            position: self.position.to_data(),
            scale: self.size.to_data(),
            color: self.color.to_data(),
            tex_coord: self.tex_coord.to_data(),
            tex_scale: self.tex_scale.to_data(),
        });
    }
}

type JobMap<T> = HashMap<BatchKey, InstancedRenderJob<T>>;

pub struct RenderData {
    static_jobs: JobMap<StaticInstanceData>,
    skeletal_jobs: JobMap<StaticInstanceData>,
    bones: Vec<Mat4Data>,
    sprite_jobs: JobMap<SpriteInstanceData>,
}

impl RenderData {
    pub fn new() -> Self {
        Self {
            static_jobs: HashMap::new(),
            skeletal_jobs: HashMap::new(),
            bones: Vec::new(),
            sprite_jobs: HashMap::new(),
        }
    }

    pub fn submit<T: SubmitJob>(&mut self, job: &T, resource_pool: &ResourcePool) {
        job.submit(self, resource_pool);
    }

    // NOTE: The instance data from the jobs is moved into the draw data when built, so
    // the jobs stay allocated and the instance vectors are not reallocated every frame.
    // They can however be explicitly reset with the reset method.

    fn build_batches<T>(jobs: &mut JobMap<T>) -> (Vec<RenderBatch>, Vec<T>) {
        let batch_count = jobs.len();
        let instance_count = jobs.iter().map(|(_, job)| job.instances.len()).sum();

        let mut batches: Vec<RenderBatch> = Vec::with_capacity(batch_count);
        let mut instances: Vec<T> = Vec::with_capacity(instance_count);

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
        let (sprite_batches, sprite_instances) = Self::build_batches(&mut self.sprite_jobs);

        let bones = self.bones.clone();
        self.bones.clear();

        DrawData {
            static_batches,
            static_instances,
            skeletal_batches,
            skeletal_instances,
            bones,
            sprite_batches,
            sprite_instances,
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.static_jobs.clear();
    }
}
