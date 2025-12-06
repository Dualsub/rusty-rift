use glam::{Quat, Vec3};

use crate::renderer::Renderer;

pub struct Game {
    turn: f32,
}

impl Game {
    pub fn new() -> Self {
        Self { turn: 0.0 }
    }

    pub fn update(&mut self, dt: f32) {
        self.turn += 1.0 * dt;
    }

    pub fn render(&self, renderer: &mut Renderer) {
        let camera_target = glam::vec3(768.0, 0.0, 512.0);

        const CAMERA_RADIUS: f32 = 1200.0;
        const CAMERA_ANGLE: f32 = f32::to_radians(56.0);

        let camera_position = camera_target
            + Vec3 {
                x: 0.0,
                y: CAMERA_ANGLE.sin(),
                z: CAMERA_ANGLE.cos(),
            } * CAMERA_RADIUS;
        let camera_orientation = Quat::from_rotation_x(-CAMERA_ANGLE);

        renderer.set_camera_position_and_orientation(camera_position, camera_orientation);
    }
}
