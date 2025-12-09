use shared::math::*;

use crate::renderer::RenderDevice;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, Debug)]
pub struct AnimationFrame {
    pub position: Vec3,
    pub rotation: Quat,
}

impl AnimationFrame {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position)
    }
}

impl Default for AnimationFrame {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

unsafe impl bytemuck::Pod for AnimationFrame {}

pub struct Animation {
    pub frames: Vec<AnimationFrame>,
    pub times: Vec<f32>,
}

impl Animation {
    pub fn get_frame_count(&self) -> usize {
        self.times.len()
    }
}

#[derive(Default)]
pub struct AnimationLoadDesc {
    pub frames: Vec<AnimationFrame>,
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
        let mut frames: Vec<AnimationFrame> = Vec::new();
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
