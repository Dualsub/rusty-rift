use glam::{Quat, Vec3, Vec3Swizzles};
use shared::{
    math::*,
    physics::{BodyId, BodySettings, BodyState, CollisionLayer, CollisionShape, PhysicsWorld},
    transform::Transform,
};

use crate::{
    input::{InputAction, InputState},
    renderer::{
        Renderer, ResourceHandle, StaticRenderJob,
        animation::{AnimationInstance, Pose},
        render_data::SkeletalRenderJob,
        resources::get_handle,
    },
};

type CTransform = Transform;

struct CRenderable {
    pub mesh: ResourceHandle,
    pub material: ResourceHandle,
    pub render_offset: Mat4,
    pub color: Vec4,
    pub tex_coord: Vec2,
    pub tex_scale: Vec2,
}

impl Default for CRenderable {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            material: Default::default(),
            render_offset: Mat4::IDENTITY,
            color: Vec4::ONE,
            tex_coord: Vec2::ZERO,
            tex_scale: Vec2::ONE,
        }
    }
}

#[derive(Default)]
struct CAnimator {
    pub pose: Pose,
    pub animation_states: [AnimationInstance; 2],
    pub time: f32,
}

#[derive(Default)]
struct CPhysicsProxy {
    pub body_id: Option<BodyId>,
    pub current_state: Option<BodyState>,
    pub previous_state: Option<BodyState>,
}

#[derive(Default)]
struct CPlayerMovement {
    pub velocity: Vec3,
}

#[derive(Default)]
struct EPlayer {
    pub transform: CTransform,
    pub physics_proxy: CPhysicsProxy,
    pub renderable: CRenderable,
    pub animator: CAnimator,
    pub movement: CPlayerMovement,
}

pub struct Game {
    player: EPlayer,
}

impl Game {
    pub fn new() -> Self {
        Self {
            player: Default::default(),
        }
    }

    pub fn initialize(&mut self, physics_world: &mut PhysicsWorld) {
        let player_position = Vec3::new(0.0, 0.0, 0.0);
        let player_body_id = physics_world.create_rigid_body(&BodySettings {
            position: player_position.xz(),
            velocity: Vec2::ZERO,
            layer: CollisionLayer::Player,
            shape: &CollisionShape::Circle { radius: 32.0 },
            listen_to_contact_events: true,
        });
        let current_state = physics_world.get_state(player_body_id);
        self.player = EPlayer {
            transform: CTransform {
                position: player_position,
                ..Default::default()
            },
            physics_proxy: CPhysicsProxy {
                body_id: Some(player_body_id),
                current_state: current_state,
                previous_state: current_state,
            },
            renderable: CRenderable {
                mesh: get_handle("Brute"),
                material: get_handle("BruteMaterial"),
                render_offset: Mat4::from_rotation_x(-90.0_f32.to_radians()),
                ..Default::default()
            },
            animator: Default::default(),
            ..Default::default()
        }
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

        self.player.animator.pose = renderer.create_pose(mesh);

        renderer.load_animation(
            "Brute_Idle",
            include_bytes!("../../assets/champions/brute/animations/Brute_Idle.dat"),
        );

        renderer.load_animation(
            "Brute_Run",
            include_bytes!("../../assets/champions/brute/animations/Brute_Run.dat"),
        );
    }

    pub fn update(&mut self, dt: f32, input_state: &InputState) {
        // Player
        {
            let transform = &mut self.player.transform;
            let physics_proxy = &mut self.player.physics_proxy;

            if let Some(state) = physics_proxy.current_state {
                transform.position = state.position.at_y(0.0);
            }

            let movement = &mut self.player.movement;
            let animator = &mut self.player.animator;

            const SPEED: f32 = 300.0;
            let mut input_velocity = Vec3::ZERO;
            input_velocity += if input_state.is_down(InputAction::W) {
                Vec3::new(0.0, 0.0, -SPEED)
            } else {
                Vec3::new(0.0, 0.0, 0.0)
            };
            input_velocity += if input_state.is_down(InputAction::Q) {
                Vec3::new(0.0, 0.0, SPEED)
            } else {
                Vec3::new(0.0, 0.0, 0.0)
            };

            movement.velocity = movement
                .velocity
                .lerp(input_velocity, (15.0 * dt).clamp(0.0, 1.0));

            let blend = movement.velocity.length() / SPEED;
            animator.time += movement.velocity.z.signum() * dt;
            animator.animation_states = [
                AnimationInstance {
                    animation: get_handle("Brute_Idle"),
                    blend_weight: 1.0 - blend,
                    time: animator.time,
                    looping: true,
                },
                AnimationInstance {
                    animation: get_handle("Brute_Run"),
                    blend_weight: blend,
                    time: animator.time,
                    looping: true,
                },
            ];
        }
    }

    pub fn fixed_update(&mut self, dt: f32, physics_world: &mut PhysicsWorld) {
        if let Some(body_id) = self.player.physics_proxy.body_id {
            physics_world.set_velocity(body_id, self.player.movement.velocity.xz());
        }

        physics_world.step_simulation(dt);

        if let Some(body_id) = self.player.physics_proxy.body_id {
            self.player.physics_proxy.previous_state = self.player.physics_proxy.current_state;
            self.player.physics_proxy.current_state = physics_world.get_state(body_id);
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer) {
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

        // Player
        {
            let transform = &self.player.transform;
            let renderable = &self.player.renderable;
            let animator = &mut self.player.animator;

            renderer.accumulate_pose(&animator.animation_states, &mut animator.pose);

            renderer.submit(&SkeletalRenderJob {
                transform: transform.to_matrix() * renderable.render_offset,
                material: renderable.material,
                mesh: renderable.mesh,
                tex_coord: renderable.tex_coord,
                tex_scale: renderable.tex_scale,
                color: renderable.color,
                pose: Some(&animator.pose),
            });
        }

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
