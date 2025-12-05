pub use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

pub type Mat4Data = [f32; 16];
pub type Vec4Data = [f32; 4];

pub trait ToData<T> {
    fn to_data(&self) -> T;
}

impl ToData<Mat4Data> for Mat4 {
    fn to_data(&self) -> Mat4Data {
        self.to_cols_array()
    }
}

impl ToData<Vec4Data> for Vec4 {
    fn to_data(&self) -> Vec4Data {
        self.to_array()
    }
}
