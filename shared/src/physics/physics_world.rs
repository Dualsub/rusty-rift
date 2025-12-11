use std::collections::BTreeMap;

use crate::{
    math::Vec2,
    physics::{CollisionLayer, collision::CollisionShape},
    pool::{Pool, PoolIndex},
};

const GRID_CELL_SIZE: f32 = 160.0;
type GridCellIndex = (i32, i32);
type Grid = BTreeMap<GridCellIndex, Vec<BodyId>>;

pub fn _get_grid_cell_index(position: Vec2) -> GridCellIndex {
    (
        (position.x / GRID_CELL_SIZE).floor() as i32,
        (position.y / GRID_CELL_SIZE).floor() as i32,
    )
}

pub fn for_grid_cells_in_aabb<T: FnMut((i32, i32)) -> ()>(
    aabb_min: Vec2,
    aabb_max: Vec2,
    mut f: T,
) {
    let min_x = aabb_min.x.min(aabb_max.x);
    let max_x = aabb_min.x.max(aabb_max.x);
    let min_y = aabb_min.y.min(aabb_max.y);
    let max_y = aabb_min.y.max(aabb_max.y);

    let min_cell_x = (min_x / GRID_CELL_SIZE).floor() as i32;
    let max_cell_x = (max_x / GRID_CELL_SIZE).floor() as i32;
    let min_cell_y = (min_y / GRID_CELL_SIZE).floor() as i32;
    let max_cell_y = (max_y / GRID_CELL_SIZE).floor() as i32;

    for cy in min_cell_y..=max_cell_y {
        for cx in min_cell_x..=max_cell_x {
            f((cx, cy));
        }
    }
}

pub type BodyId = PoolIndex;

pub struct ContactEvent {
    pub other: BodyId,
    pub penetration: f32,
    pub normal: Vec2,
}

struct Body {
    position: Vec2,
    velocity: Vec2,
    layer: CollisionLayer,
    shape: CollisionShape,
    contacts: Option<Vec<ContactEvent>>, // None if not listining to contacts
}

impl Body {
    pub fn correct(&mut self, correction: Vec2) {
        self.position += correction;
    }
}

pub struct BodySettings<'a> {
    pub position: Vec2,
    pub velocity: Vec2,
    pub layer: CollisionLayer,
    pub shape: &'a CollisionShape,
    pub listen_to_contact_events: bool,
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
            layer: settings.layer,
            shape: settings.shape.clone(),
            contacts: if settings.listen_to_contact_events {
                Some(Vec::new())
            } else {
                None
            },
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

    pub fn get_contacts(&self, id: BodyId) -> Option<&[ContactEvent]> {
        self.bodies
            .get(id)
            .and_then(|b| b.contacts.as_ref().map(|c| c.as_slice()))
    }

    pub fn get_layer(&self, id: BodyId) -> Option<CollisionLayer> {
        self.bodies.get(id).map(|b| b.layer)
    }

    pub fn set_position(&mut self, id: BodyId, position: Vec2) {
        if let Some(body) = self.bodies.get_mut(id) {
            body.position = position;
        }
    }

    pub fn set_velocity(&mut self, id: BodyId, velocity: Vec2) {
        if let Some(body) = self.bodies.get_mut(id) {
            body.velocity = velocity;
        }
    }

    pub fn set_layer(&mut self, id: BodyId, layer: CollisionLayer) {
        if let Some(body) = self.bodies.get_mut(id) {
            body.layer = layer;
        }
    }

    pub fn set_shape(&mut self, id: BodyId, shape: CollisionShape) {
        if let Some(body) = self.bodies.get_mut(id) {
            body.shape = shape;
        }
    }

    fn build_grid(&mut self) {
        self.grid.clear();
        for (body_id, body) in self.bodies.iter() {
            // We check the four corners of the AABB. This works as long as the AABB is not larger then a cell
            let (extent_min, extent_max) = body.shape.get_aabb(body.position);
            for_grid_cells_in_aabb(extent_min, extent_max, |cell_index| {
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
            });
        }
    }

    fn get_collision_pairs(&self) -> Vec<(BodyId, BodyId)> {
        let mut pairs = Vec::new();
        for cell_bodies in self.grid.values() {
            for i in 0..cell_bodies.len() {
                for j in (i + 1)..cell_bodies.len() {
                    let body_i = cell_bodies[i];
                    let body_j = cell_bodies[j];

                    let b1 = self.bodies.get(body_i).unwrap();
                    let b2 = self.bodies.get(body_j).unwrap();

                    if !b1.layer.collides_with(b2.layer) {
                        continue;
                    }

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
            if let Some(contacts) = &mut body.contacts {
                contacts.clear();
            }
        }

        self.build_grid();
        let collision_pairs: Vec<_> = self.get_collision_pairs();

        for iter in 0..Self::NUM_SIMULATION_ITERATIONS {
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

                    // Record contact events only on the first iteration
                    if iter == 0 {
                        if let Some(contacts) =
                            &mut self.bodies.get_mut(*body_id1).unwrap().contacts
                        {
                            contacts.push(ContactEvent {
                                other: *body_id2,
                                penetration,
                                normal,
                            });
                        }
                        if let Some(contacts) =
                            &mut self.bodies.get_mut(*body_id2).unwrap().contacts
                        {
                            contacts.push(ContactEvent {
                                other: *body_id1,
                                penetration,
                                normal: -normal,
                            });
                        }
                    }
                }
            }
        }

        // Build for query
        self.build_grid();
    }

    pub fn query_shape(&self, position: Vec2, shape: CollisionShape) -> Vec<BodyId> {
        let (extent_min, extent_max) = shape.get_aabb(position);
        let mut result = Vec::new();

        for_grid_cells_in_aabb(extent_min, extent_max, |cell_index| {
            if let Some(cell_bodies) = self.grid.get(&cell_index) {
                for &id in cell_bodies {
                    if !result.contains(&id) {
                        result.push(id);
                    }
                }
            }
        });

        result
    }
}
