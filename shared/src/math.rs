pub use glam::{Mat4, Vec2, Vec3};

pub type Mat4Data = [f32; 16];

pub trait ToData<T> {
    fn to_data(&self) -> T;
}

impl ToData<Mat4Data> for Mat4 {
    fn to_data(&self) -> Mat4Data {
        self.to_cols_array()
    }
}
