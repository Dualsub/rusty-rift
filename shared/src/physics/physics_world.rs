use std::collections::BTreeMap;

use crate::{
    math::Vec2,
    physics::collision::CollisionShape,
    pool::{Pool, PoolIndex},
};

const GRID_CELL_SIZE: f32 = 160.0;
type GridCellIndex = (i32, i32);
type Grid = BTreeMap<GridCellIndex, Vec<BodyId>>;

pub fn get_grid_cell_index(position: Vec2) -> GridCellIndex {
    (
        (position.x / GRID_CELL_SIZE).floor() as i32,
        (position.y / GRID_CELL_SIZE).floor() as i32,
    )
}

pub type BodyId = PoolIndex;

struct Body {
    position: Vec2,
    velocity: Vec2,
    shape: CollisionShape,
}

impl Body {
    pub fn correct(&mut self, correction: Vec2) {
        self.position += correction;
    }
}

pub struct BodySettings<'a> {
    pub position: Vec2,
    pub velocity: Vec2,
    pub shape: &'a CollisionShape,
}

pub struct BodyState {
    pub position: Vec2,
    pub velocity: Vec2,
}

pub struct PhysicsWorld {
    bodies: Pool<Body>,
    grid: Grid,
}

impl PhysicsWorld {
    const NUM_SIMULATION_ITERATIONS: u32 = 4;

    pub fn new() -> Self {
        Self {
            bodies: Pool::new(),
            grid: BTreeMap::new(),
        }
    }

    pub fn create_rigid_body(&mut self, settings: &BodySettings) -> BodyId {
        self.bodies.push(Body {
            position: settings.position,
            velocity: settings.velocity,
            shape: settings.shape.clone(),
        })
    }

    pub fn get_state(&self, id: BodyId) -> Option<BodyState> {
        self.bodies.get(id).map(|b| BodyState {
            position: b.position,
            velocity: b.velocity,
        })
    }

    pub fn get_shape(&self, id: BodyId) -> Option<CollisionShape> {
        self.bodies.get(id).map(|b| b.shape)
    }

    fn build_grid(&mut self) {
        self.grid.clear();
        for (body_id, body) in self.bodies.iter() {
            // We check the four corners of the AABB. This works as long as the AABB is not larger then a cell
            let (extent_min, extent_max) = body.shape.get_aabb();
            for position in [
                body.position,
                body.position + extent_min,
                body.position + extent_max,
                body.position + Vec2::new(extent_min.x, extent_max.y),
                body.position + Vec2::new(extent_max.x, extent_min.y),
            ] {
                let cell_index = get_grid_cell_index(position);
                let bodies = self.grid.entry(cell_index).or_default();

                if bodies.len() > 32 {
                    log::warn!(
                        "The number of bodies in one cell is high({}), consider not doing linear search.",
                        bodies.len()
                    )
                }

                // This linear search will be fast for few elements
                if !bodies.contains(&body_id) {
                    bodies.push(body_id);
                }
            }
        }
    }

    fn get_collision_pairs(&self) -> Vec<(BodyId, BodyId)> {
        let mut pairs = Vec::new();
        for cell_bodies in self.grid.values() {
            for i in 0..cell_bodies.len() {
                for j in (i + 1)..cell_bodies.len() {
                    let body_i = cell_bodies[i];
                    let body_j = cell_bodies[j];

                    // We only push pairs were this is true, if it is false,
                    // it will be true when checking the other way around for another cell
                    if body_i.index() < body_j.index() {
                        pairs.push((body_i, body_j));
                    }
                }
            }
        }

        pairs
    }

    pub fn step_simulation(&mut self, dt: f32) {
        for (_, body) in self.bodies.iter_mut() {
            body.position += body.velocity * dt;
        }

        self.build_grid();
        let collision_pairs: Vec<_> = self.get_collision_pairs();

        for _ in 0..Self::NUM_SIMULATION_ITERATIONS {
            for (body_id1, body_id2) in collision_pairs.iter() {
                let body1 = self.bodies.get(*body_id1).unwrap();
                let body2 = self.bodies.get(*body_id2).unwrap();

                let (penetration, normal) =
                    body1
                        .shape
                        .get_overlap(body1.position, &body2.shape, body2.position);
                let correction = penetration * 0.5 * normal;

                if penetration > 0.0 {
                    self.bodies.get_mut(*body_id1).unwrap().correct(-correction);
                    self.bodies.get_mut(*body_id2).unwrap().correct(correction);
                }
            }
        }
    }
}
