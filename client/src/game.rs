use glam::{Quat, Vec3};
use shared::{
    math::*,
    physics::{BodyId, BodySettings, BodyState, CollisionShape, PhysicsWorld},
};

use crate::renderer::{
    Renderer, StaticRenderJob,
    animation::{AnimationInstance, Pose},
    render_data::SkeletalRenderJob,
    resources::get_handle,
};

const SPHERE_COUNT: usize = 32;
const SPHERE_RADIUS: f32 = 30.0;

fn random_range(seed: &mut u32, min: f32, max: f32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    let t = (*seed as f32) / (u32::MAX as f32);
    min + (max - min) * t
}

pub struct Game {
    turn: f32,
    pose: Pose,
    animation_time: f32,

    sphere_ids: Vec<BodyId>,
    sphere_states: Vec<Option<BodyState>>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            turn: 0.0,
            pose: Pose::new(0),
            animation_time: 0.0,
            sphere_ids: Vec::new(),
            sphere_states: Vec::new(),
        }
    }

    pub fn initialize(&mut self, physics_world: &mut PhysicsWorld) {
        self.sphere_ids.clear();
        self.sphere_states.clear();

        let mut seed: u32 = 0x1234_5678;
        let center = Vec2::ZERO;

        for _ in 0..SPHERE_COUNT {
            let position = Vec2::new(
                random_range(&mut seed, -400.0, 400.0),
                random_range(&mut seed, -400.0, 400.0),
            );

            let to_center = center - position;

            let speed = random_range(&mut seed, 50.0, 1500.0);

            let velocity = if to_center.length_squared() > 0.0001 {
                to_center.normalize() * speed
            } else {
                Vec2::new(0.0, speed)
            };

            let body_id = physics_world.create_rigid_body(&BodySettings {
                position,
                velocity,
                shape: &CollisionShape::Circle {
                    radius: SPHERE_RADIUS,
                },
            });

            self.sphere_ids.push(body_id);
            self.sphere_states.push(None);
        }

        log::info!(
            "Initialized {} spheres (toward center)",
            self.sphere_ids.len()
        );
    }

    pub fn load_resources(&mut self, renderer: &mut Renderer) {
        renderer.create_material("Grid", include_bytes!("../../assets/textures/grid.dat"));
        renderer.load_mesh("Floor", include_bytes!("../../assets/models/floor.dat"));
        renderer.load_mesh("Sphere", include_bytes!("../../assets/models/sphere.dat"));

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

    pub fn update(&mut self, dt: f32) {
        self.turn += 0.0 * dt;
        self.animation_time += dt;
    }

    pub fn fixed_update(&mut self, dt: f32, physics_world: &mut PhysicsWorld) {
        physics_world.step_simulation(dt);

        for (index, body_id) in self.sphere_ids.iter().enumerate() {
            self.sphere_states[index] = physics_world.get_state(*body_id);
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer) {
        for state in &self.sphere_states {
            if let Some(body_state) = state {
                renderer.submit(&StaticRenderJob {
                    transform: Mat4::from_translation(body_state.position.at_y(100.0))
                        * Mat4::from_scale(Vec3::ONE * SPHERE_RADIUS),
                    mesh: get_handle("Sphere"),
                    material: get_handle("Grid"),
                    ..Default::default()
                });
            }
        }

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

        // Camera
        {
            let camera_target = glam::vec3(0.0, 120.0, 0.0);

            const CAMERA_RADIUS: f32 = 1844.8713602850469_f32;
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
}
