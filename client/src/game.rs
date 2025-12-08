use glam::{Quat, Vec3, vec3a};
use shared::math::*;

use crate::renderer::{
    Renderer, StaticRenderJob, render_data::SkeletalRenderJob, resources::get_handle,
};

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

    pub fn load_resources(&mut self, renderer: &mut Renderer) {
        renderer.load_skeletal_mesh("Brute", include_bytes!("../../assets/models/brute.dat"));
        renderer.load_mesh("Floor", include_bytes!("../../assets/models/floor.dat"));
        renderer.create_material("Grid", include_bytes!("../../assets/textures/grid.dat"));
    }

    pub fn render(&self, renderer: &mut Renderer) {
        renderer.submit(&SkeletalRenderJob {
            transform: Mat4::IDENTITY,
            material: get_handle("Grid"),
            mesh: get_handle("Brute"),
            ..Default::default()
        });

        renderer.submit(&StaticRenderJob {
            transform: Mat4::from_scale_rotation_translation(
                Vec3 {
                    x: 0.4,
                    y: 0.4,
                    z: 0.4,
                },
                Quat::IDENTITY,
                Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ),
            material: get_handle("Grid"),
            mesh: get_handle("Floor"),
            color: Vec4::new(0.651, 0.541, 0.392, 1.0),
            tex_scale: Vec2::ONE * 10.0,
            ..Default::default()
        });

        // Lighting
        {
            renderer.set_lighting_direction(
                Vec3 {
                    x: self.turn.sin(),
                    y: -2.0,
                    z: self.turn.cos(),
                }
                .normalize(),
            );
        }

        // Camera
        {
            let camera_target = glam::vec3(0.0, 150.0, 0.0);

            const CAMERA_RADIUS: f32 = 300.0;
            const CAMERA_ANGLE: f32 = f32::to_radians(30.0);

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
}
