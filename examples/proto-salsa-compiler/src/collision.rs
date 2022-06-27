use std::{fmt::Display, sync::Arc};

use crate::compiler::Compiler;

// Using i64 because float equality doesn't exist in Rust.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AABBCollision {
    pub min_x: i64,
    pub min_y: i64,
    pub min_z: i64,
    pub max_x: i64,
    pub max_y: i64,
    pub max_z: i64,
}

impl Default for AABBCollision {
    fn default() -> Self {
        Self {
            min_x: i64::MAX,
            min_y: i64::MAX,
            min_z: i64::MAX,
            max_x: i64::MIN,
            max_y: i64::MIN,
            max_z: i64::MIN,
        }
    }
}

impl AABBCollision {
    pub fn extend(&self, other: &AABBCollision) -> AABBCollision {
        AABBCollision {
            min_x: if self.min_x < other.min_x {
                self.min_x
            } else {
                other.min_x
            },
            min_y: if self.min_y < other.min_y {
                self.min_y
            } else {
                other.min_y
            },
            min_z: if self.min_z < other.min_z {
                self.min_z
            } else {
                other.min_z
            },
            max_x: if self.max_x > other.max_x {
                self.max_x
            } else {
                other.max_x
            },
            max_y: if self.max_y > other.max_y {
                self.max_y
            } else {
                other.max_y
            },
            max_z: if self.max_z > other.max_z {
                self.max_z
            } else {
                other.max_z
            },
        }
    }
}

impl Display for AABBCollision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "minX: {}, minY: {}, minZ: {}, maxX {}, maxY {}, maxZ {}",
            self.min_x, self.min_y, self.min_z, self.max_x, self.max_y, self.max_z
        )
    }
}

pub fn compile_aabb(
    _db: &dyn Compiler,
    min_x: Arc<String>,
    min_y: Arc<String>,
    min_z: Arc<String>,
    max_x: Arc<String>,
    max_y: Arc<String>,
    max_z: Arc<String>,
) -> AABBCollision {
    AABBCollision {
        // Should handle this parsing much better.
        min_x: min_x.parse::<i64>().unwrap(),
        min_y: min_y.parse::<i64>().unwrap(),
        min_z: min_z.parse::<i64>().unwrap(),
        max_x: max_x.parse::<i64>().unwrap(),
        max_y: max_y.parse::<i64>().unwrap(),
        max_z: max_z.parse::<i64>().unwrap(),
    }
}
