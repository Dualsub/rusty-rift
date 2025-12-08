use shared::math::*;

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StaticInstanceData {
    pub(crate) model_matrix: Mat4Data,
    pub(crate) color: Vec4Data,
    pub(crate) tex_coord: Vec2Data,
    pub(crate) tex_scale: Vec2Data,
}
