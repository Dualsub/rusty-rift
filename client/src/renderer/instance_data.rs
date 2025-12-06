use shared::math::*;

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StaticInstanceData {
    pub(crate) model_matrix: Mat4Data,
}
