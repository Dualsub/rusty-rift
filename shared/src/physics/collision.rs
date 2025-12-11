use crate::math::Vec2;

#[derive(Copy, Clone)]
pub enum CollisionShape {
    Circle { radius: f32 },
}

impl CollisionShape {
    pub fn get_aabb(&self) -> (Vec2, Vec2) {
        match self {
            Self::Circle { radius } => (Vec2::new(-*radius, -*radius), Vec2::new(*radius, *radius)),
        }
    }

    pub fn get_overlap(
        &self,
        position: Vec2,
        other: &CollisionShape,
        other_position: Vec2,
    ) -> (f32, Vec2) {
        match (self, other) {
            (
                CollisionShape::Circle { radius },
                CollisionShape::Circle {
                    radius: other_radius,
                },
            ) => {
                // Overlap
                let min_distance = radius + other_radius;
                let distance_squared = position.distance_squared(other_position);
                if distance_squared > 0.0 && min_distance * min_distance > distance_squared {
                    let penetration = min_distance - distance_squared.sqrt();
                    let normal = (other_position - position).normalize_or_zero();
                    return (penetration, normal);
                }

                (0.0, Vec2::ZERO)
            }
        }
    }
}
