use glam::{Quat, Vec3};
use shared::{
    math::*,
    physics::{BodyId, BodySettings, BodyState, CollisionLayer, CollisionShape, PhysicsWorld},
};

use crate::renderer::{
    Renderer, StaticRenderJob,
    animation::{AnimationInstance, Pose},
    render_data::SkeletalRenderJob,
    resources::get_handle,
};

const SPHERE_COUNT: usize = 32;
const BASE_RADIUS: f32 = 30.0;

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
    sphere_layers: Vec<CollisionLayer>,
    sphere_radii: Vec<f32>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            turn: 0.0,
            pose: Pose::new(0),
            animation_time: 0.0,
            sphere_ids: Vec::new(),
            sphere_states: Vec::new(),
            sphere_layers: Vec::new(),
            sphere_radii: Vec::new(),
        }
    }

    pub fn initialize(&mut self, physics_world: &mut PhysicsWorld) {
        self.sphere_ids.clear();
        self.sphere_states.clear();
        self.sphere_layers.clear();
        self.sphere_radii.clear();

        let mut seed: u32 = 0x1234_5678;
        let center = Vec2::ZERO;

        let layer_sequence = [
            CollisionLayer::Player,
            CollisionLayer::Enemy,
            CollisionLayer::PlayerProjectile,
            CollisionLayer::EnemyProjectile,
        ];

        for i in 0..SPHERE_COUNT {
            let layer = layer_sequence[i % layer_sequence.len()];

            let radius = match layer {
                CollisionLayer::Player => BASE_RADIUS * 1.2,
                CollisionLayer::Enemy => BASE_RADIUS * 1.2,
                CollisionLayer::PlayerProjectile => BASE_RADIUS * 0.6,
                CollisionLayer::EnemyProjectile => BASE_RADIUS * 0.6,
                CollisionLayer::Environment => BASE_RADIUS * 1.8,
            };

            let position = Vec2::new(
                random_range(&mut seed, -400.0, 400.0),
                random_range(&mut seed, -400.0, 400.0),
            );

            let to_center = center - position;

            let (speed_min, speed_max) = match layer {
                CollisionLayer::Player | CollisionLayer::Enemy => (150.0, 450.0),
                CollisionLayer::PlayerProjectile | CollisionLayer::EnemyProjectile => {
                    (600.0, 1200.0)
                }
                CollisionLayer::Environment => (0.0, 0.0),
            };

            let speed = random_range(&mut seed, speed_min, speed_max);

            let velocity = if to_center.length_squared() > 0.0001 {
                to_center.normalize() * speed
            } else {
                Vec2::new(0.0, speed)
            };

            let body_id = physics_world.create_rigid_body(&BodySettings {
                position,
                velocity,
                layer,
                shape: &CollisionShape::Circle { radius },
                listen_to_contact_events: true,
            });

            self.sphere_ids.push(body_id);
            self.sphere_states.push(None);
            self.sphere_layers.push(layer);
            self.sphere_radii.push(radius);
        }

        log::info!(
            "Initialized {} spheres (toward center, mixed layers)",
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

    fn layer_color(layer: CollisionLayer) -> glam::Vec4 {
        match layer {
            CollisionLayer::Player => glam::Vec4::new(0.2, 0.4, 1.0, 1.0),
            CollisionLayer::Enemy => glam::Vec4::new(1.0, 0.25, 0.25, 1.0),
            CollisionLayer::PlayerProjectile => glam::Vec4::new(0.2, 1.0, 0.8, 1.0),
            CollisionLayer::EnemyProjectile => glam::Vec4::new(1.0, 0.7, 0.2, 1.0),
            CollisionLayer::Environment => glam::Vec4::new(0.5, 0.5, 0.5, 1.0),
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer) {
        for (index, state) in self.sphere_states.iter().enumerate() {
            if let Some(body_state) = state {
                let layer = self.sphere_layers[index];
                let radius = self.sphere_radii[index];
                let color = Self::layer_color(layer);

                renderer.submit(&StaticRenderJob {
                    transform: Mat4::from_translation(body_state.position.at_y(100.0))
                        * Mat4::from_scale(Vec3::ONE * radius),
                    mesh: get_handle("Sphere"),
                    material: get_handle("Grid"),
                    color,
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
