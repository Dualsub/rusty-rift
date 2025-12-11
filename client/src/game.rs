use glam::{Quat, Vec3};
use shared::math::*;

use crate::renderer::{
    Renderer, StaticRenderJob,
    animation::{AnimationInstance, Pose},
    render_data::SkeletalRenderJob,
    resources::get_handle,
};

pub struct Game {
    turn: f32,
    pose: Pose,
    animation_time: f32,
}

impl Game {
    pub fn new() -> Self {
        Self {
            turn: 0.0,
            pose: Pose::new(0),
            animation_time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.turn += 0.0 * dt;
        self.animation_time += dt;
    }

    pub fn load_resources(&mut self, renderer: &mut Renderer) {
        renderer.create_material("Grid", include_bytes!("../../assets/textures/grid.dat"));
        renderer.load_mesh("Floor", include_bytes!("../../assets/models/floor.dat"));

        renderer.create_material(
            "BruteMaterial",
            include_bytes!(
                "../../assets/champions/brute/textures/MaleBruteA_Body_diffuse1_ncl1_1.dat"
            ),
        );

        let mesh = renderer.load_skeletal_mesh(
            "Brute",
            include_bytes!("../../assets/champions/brute/Brute.dat"),
        );

        self.pose = renderer.create_pose(mesh);

        renderer.load_animation(
            "Brute_Idle",
            include_bytes!("../../assets/champions/brute/animations/Brute_Idle.dat"),
        );

        renderer.load_animation(
            "Brute_Run",
            include_bytes!("../../assets/champions/brute/animations/Brute_Run.dat"),
        );
    }

    pub fn render(&mut self, renderer: &mut Renderer) {
        let blend = (self.animation_time).sin() * 0.5 + 0.5;

        renderer.accumulate_pose(
            &[
                AnimationInstance {
                    animation: get_handle("Brute_Idle"),
                    blend_weight: blend,
                    time: self.animation_time,
                    looping: true,
                },
                AnimationInstance {
                    animation: get_handle("Brute_Run"),
                    blend_weight: 1.0 - blend,
                    time: self.animation_time,
                    looping: true,
                },
            ],
            &mut self.pose,
        );

        renderer.submit(&SkeletalRenderJob {
            transform: Mat4::from_rotation_x(-90.0_f32.to_radians())
                * Mat4::from_rotation_z(-self.turn),
            material: get_handle("BruteMaterial"),
            mesh: get_handle("Brute"),
            pose: Some(&self.pose),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
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
                    x: 0.0, //self.turn.sin(),
                    y: -2.0,
                    z: 0.0, //self.turn.cos(),
                }
                .normalize(),
            );
        }

        // Camera
        {
            let camera_target = glam::vec3(0.0, 120.0, 0.0);

            const CAMERA_RADIUS: f32 = 1844.8713602850469_f32;
            const CAMERA_ANGLE: f32 = f32::to_radians(56.0);
            // const CAMERA_RADIUS: f32 = 500.0_f32;
            // const CAMERA_ANGLE: f32 = f32::to_radians(30.0);

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
