use shared::math::*;

use crate::renderer::{RenderDevice, ResourceHandle, SkeletalMesh};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, Debug)]
pub struct LocalBoneTransform {
    pub position: Vec3,
    pub rotation: Quat,
}

impl LocalBoneTransform {
    #[allow(dead_code)]
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position)
    }
}

impl Default for LocalBoneTransform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

pub struct Pose {
    pub transforms: Vec<LocalBoneTransform>,
}

impl Pose {
    pub fn new(bone_count: usize) -> Self {
        let mut transforms = Vec::new();
        transforms.resize(bone_count, Default::default());
        Self { transforms }
    }

    #[allow(dead_code)]
    pub fn get_matrix(&self, bone_index: usize) -> Mat4 {
        self.transforms[bone_index].to_matrix()
    }

    #[allow(dead_code)]
    pub fn blend(a: &Pose, b: &Pose, alpha: f32, out: &mut Pose) {
        assert_eq!(a.transforms.len(), out.transforms.len());
        assert_eq!(b.transforms.len(), out.transforms.len());

        for bone_index in 0..out.transforms.len() {
            out.transforms[bone_index].position = a.transforms[bone_index]
                .position
                .lerp(b.transforms[bone_index].position, alpha);
            out.transforms[bone_index].rotation = a.transforms[bone_index]
                .rotation
                .nlerp(b.transforms[bone_index].rotation, alpha);
        }
    }
}

impl SkeletalMesh {
    #[allow(dead_code)]
    pub fn get_bone_matrices(&self, pose: &Pose, out_matrices: &mut [Mat4Data]) {
        // First pass: Calculate hierachy. Since the bones are in topological order,
        // the parent will always be calculated before the children.
        for bone_info in self.bones.iter() {
            let bone_index = bone_info.id as usize;

            let parent_transform = if bone_info.parent_id != -1 {
                let parent_bone_index = bone_info.parent_id as usize;
                Mat4::from_cols_array(&out_matrices[parent_bone_index])
            } else {
                Mat4::IDENTITY
            };

            out_matrices[bone_index] = (parent_transform * pose.get_matrix(bone_index)).to_data();
        }

        // Second pass: Calculate global transforms
        for bone_info in self.bones.iter() {
            let bone_index = bone_info.id as usize;
            out_matrices[bone_index] = (Mat4::from_cols_array(&out_matrices[bone_index])
                * Mat4::from_cols_array(&self.bones[bone_index].offset_matrix))
            .to_data();
        }
    }
}

pub struct Animation {
    pub frames: Vec<LocalBoneTransform>,
    pub times: Vec<f32>,
}

impl Animation {
    #[allow(dead_code)]
    pub fn get_frame_count(&self) -> usize {
        self.times.len()
    }

    #[allow(dead_code)]
    pub fn get_bone_count(&self) -> usize {
        self.frames.len() / self.get_frame_count()
    }

    #[allow(dead_code)]
    pub fn get_duration(&self) -> f32 {
        self.times.last().cloned().unwrap_or(0.0)
    }

    #[allow(dead_code)]
    // Sample and return the new time
    pub fn sample(&self, time: f32, looping: bool, out_pose: &mut Pose) -> f32 {
        let mut t = time;
        let duration = self.get_duration();
        let frame_count = self.get_frame_count();

        assert!(duration > 0.0);
        assert!(frame_count > 0);

        if looping {
            t = t.rem_euclid(duration);
        } else {
            t = t.clamp(0.0, duration);
        }

        let mut i0 = 0;
        while i0 + 1 < frame_count && self.times[i0 + 1] < t {
            i0 += 1;
        }
        let i1 = (i0 + 1).clamp(0, frame_count - 1);

        let t0 = self.times[i0];
        let t1 = self.times[i1];
        let alpha = if t1 > t0 {
            ((t - t0) / (t1 - t0)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let bone_count = self.get_bone_count();
        for bone_index in 0..bone_count {
            let f0 = self.frames[i0 * bone_count + bone_index];
            let f1 = self.frames[i1 * bone_count + bone_index];

            out_pose.transforms[bone_index].position = f0.position.lerp(f1.position, alpha);
            out_pose.transforms[bone_index].rotation = f0.rotation.nlerp(f1.rotation, alpha);
        }

        t
    }

    #[allow(dead_code)]
    // Sample and return the new time
    pub fn sample_and_blend(
        &self,
        time: f32,
        looping: bool,
        weight: f32,
        out_pose: &mut Pose,
    ) -> f32 {
        let mut t = time;
        let duration = self.get_duration();
        let frame_count = self.get_frame_count();

        assert!(duration > 0.0);
        assert!(frame_count > 0);

        if looping {
            t = t.rem_euclid(duration);
        } else {
            t = t.clamp(0.0, duration);
        }

        let mut i0 = 0;
        while i0 + 1 < frame_count && self.times[i0 + 1] < t {
            i0 += 1;
        }
        let i1 = (i0 + 1).clamp(0, frame_count - 1);

        let t0 = self.times[i0];
        let t1 = self.times[i1];
        let alpha = if t1 > t0 {
            ((t - t0) / (t1 - t0)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let bone_count = self.get_bone_count();
        for bone_index in 0..bone_count {
            let f0 = self.frames[i0 * bone_count + bone_index];
            let f1 = self.frames[i1 * bone_count + bone_index];

            out_pose.transforms[bone_index].position = out_pose.transforms[bone_index]
                .position
                .lerp(f0.position.lerp(f1.position, alpha), weight);
            out_pose.transforms[bone_index].rotation = out_pose.transforms[bone_index]
                .rotation
                .nlerp(f0.rotation.nlerp(f1.rotation, alpha), weight);
        }

        t
    }
}

#[allow(dead_code)]
pub struct AnimationInstance {
    pub animation: ResourceHandle,
    pub time: f32,
    pub looping: bool,
    pub blend_weight: f32,
}

#[derive(Default)]
pub struct AnimationLoadDesc {
    pub frames: Vec<LocalBoneTransform>,
    pub times: Vec<f32>,
}

impl AnimationLoadDesc {
    // Might need to look over this and just do simple copies instead, but this will do for now
    pub fn load(bytes: &[u8]) -> anyhow::Result<AnimationLoadDesc> {
        let mut read_index: usize = 0;
        let mut tmp = [0u8; 4];

        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        let num_bones = u32::from_le_bytes(tmp) as usize;
        read_index += 4;
        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        let num_frames = u32::from_le_bytes(tmp) as usize;
        read_index += 4;

        let num_total_frames = num_frames * num_bones;
        let mut frames: Vec<LocalBoneTransform> = Vec::new();
        frames.resize(num_total_frames, Default::default());

        for i in 0..num_total_frames {
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            frames[i].position.x = f32::from_le_bytes(tmp);
            read_index += 4;
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            frames[i].position.y = f32::from_le_bytes(tmp);
            read_index += 4;
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            frames[i].position.z = f32::from_le_bytes(tmp);
            read_index += 4;

            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            frames[i].rotation.w = f32::from_le_bytes(tmp);
            read_index += 4;
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            frames[i].rotation.x = f32::from_le_bytes(tmp);
            read_index += 4;
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            frames[i].rotation.y = f32::from_le_bytes(tmp);
            read_index += 4;
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            frames[i].rotation.z = f32::from_le_bytes(tmp);
            read_index += 4;
        }

        let mut times: Vec<f32> = Vec::new();
        times.resize(num_frames, 0.0);
        for i in 0..num_frames {
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            times[i] = f32::from_le_bytes(tmp);
            read_index += 4;
        }

        Ok(AnimationLoadDesc { frames, times })
    }
}

impl RenderDevice {
    pub fn load_animation(&self, bytes: &[u8]) -> anyhow::Result<Animation> {
        let desc = AnimationLoadDesc::load(bytes)?;
        self.create_animation(&desc)
    }

    pub fn create_animation(&self, desc: &AnimationLoadDesc) -> anyhow::Result<Animation> {
        let frames = desc.frames.clone();
        let times = desc.times.clone();

        Ok(Animation { frames, times })
    }
}
