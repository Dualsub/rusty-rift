use glam::{Quat, Vec3};

use crate::renderer::Renderer;

pub struct Game {}

impl Game {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update() {}
    pub fn render(&self, renderer: &mut Renderer) {
        let camera_target = Vec3::ZERO;

        const CAMERA_RADIUS: f32 = 2000.0;
        const CAMERA_ANGLE: f32 = f32::to_radians(56.0);

        let camera_position = camera_target
            + Vec3 {
                x: CAMERA_ANGLE.sin(),
                y: CAMERA_ANGLE.cos(),
                z: 0.0,
            } * CAMERA_RADIUS;
        let camera_orientation = Quat::from_rotation_x(-CAMERA_ANGLE);

        renderer.set_camera_position_and_orientation(camera_position, camera_orientation);
    }
}
