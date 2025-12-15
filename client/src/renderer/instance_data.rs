use shared::math::*;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StaticInstanceData {
    pub(crate) model_matrix: Mat4Data,
    pub(crate) color: Vec4Data,
    pub(crate) tex_coord: Vec2Data,
    pub(crate) tex_scale: Vec2Data,
    pub(crate) data_indices: [u32; 4],
}

impl Default for StaticInstanceData {
    fn default() -> Self {
        Self {
            model_matrix: Mat4::IDENTITY.to_data(),
            color: Vec4::ONE.to_data(),
            tex_coord: Vec2::ZERO.to_array(),
            tex_scale: Vec2::ONE.to_array(),
            data_indices: [0, 0, 0, 0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstanceData {
    pub(crate) position: Vec2Data,
    pub(crate) scale: Vec2Data,
    pub(crate) color: Vec4Data,
    pub(crate) tex_coord: Vec2Data,
    pub(crate) tex_scale: Vec2Data,
}

impl Default for SpriteInstanceData {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO.to_array(),
            scale: Vec2::ONE.to_array(),
            color: Vec4::ONE.to_data(),
            tex_coord: Vec2::ZERO.to_array(),
            tex_scale: Vec2::ONE.to_array(),
        }
    }
}
