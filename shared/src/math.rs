use std::ops::Neg;

pub use glam::{Mat4, Quat, UVec2, Vec2, Vec2Swizzles, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};

pub type Mat4Data = [f32; 16];
pub type Vec4Data = [f32; 4];
pub type Vec2Data = [f32; 2];

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

impl ToData<Vec2Data> for Vec2 {
    fn to_data(&self) -> Vec2Data {
        self.to_array()
    }
}

pub trait NLerp<T> {
    fn nlerp(&self, rhs: T, s: f32) -> T;
}

impl NLerp<Quat> for Quat {
    fn nlerp(&self, rhs: Quat, s: f32) -> Quat {
        let mut rhs = rhs;
        let dot = rhs.dot(*self);
        if dot < 0.0 {
            rhs = rhs.neg();
        }

        self.lerp(rhs, s).normalize()
    }
}

pub trait Vec2To3 {
    fn at_y(&self, y: f32) -> Vec3;
}

impl Vec2To3 for Vec2 {
    fn at_y(&self, y: f32) -> Vec3 {
        Vec3::new(self.x, y, self.y)
    }
}
