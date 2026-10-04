use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    #[default]
    Square,
    Circle,
    Diamond,
    Star,
}

impl Shape {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "square" | "rect" | "box" => Some(Shape::Square),
            "circle" | "round" | "disc" => Some(Shape::Circle),
            "diamond" => Some(Shape::Diamond),
            "star" => Some(Shape::Star),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Shape::Square => "square",
            Shape::Circle => "circle",
            Shape::Diamond => "diamond",
            Shape::Star => "star",
        }
    }

    /// Checks if a block position (x, z) falls inside this shape with the given center and radius.
    pub fn contains_block(&self, center_x: i32, center_z: i32, x: i32, z: i32, radius: i32) -> bool {
        if radius <= 0 {
            return x == center_x && z == center_z;
        }

        let dx = (x - center_x).abs() as f64;
        let dz = (z - center_z).abs() as f64;
        let r = radius as f64;

        match self {
            Shape::Square => dx <= r && dz <= r,
            Shape::Circle => (dx * dx + dz * dz) <= (r * r),
            Shape::Diamond => (dx + dz) <= r,
            Shape::Star => {
                if dx == 0.0 && dz == 0.0 {
                    true
                } else {
                    let term_x = (dx / r).powf(2.0 / 3.0);
                    let term_z = (dz / r).powf(2.0 / 3.0);
                    (term_x + term_z) <= 1.0
                }
            }
        }
    }

    /// Checks if a chunk at (chunk_x, chunk_z) is included in the shape.
    /// Uses the center of the chunk (offset +8 blocks in x and z).
    pub fn contains_chunk(&self, center_x: i32, center_z: i32, chunk_x: i32, chunk_z: i32, radius: i32) -> bool {
        let block_x = chunk_x * 16 + 8;
        let block_z = chunk_z * 16 + 8;
        self.contains_block(center_x, center_z, block_x, block_z, radius)
    }

    /// Calculates the chunk coordinate bounding box [min_cx, max_cx, min_cz, max_cz] for this shape and radius.
    pub fn chunk_bounds(&self, center_x: i32, center_z: i32, radius: i32) -> (i32, i32, i32, i32) {
        let min_bx = center_x - radius;
        let max_bx = center_x + radius;
        let min_bz = center_z - radius;
        let max_bz = center_z + radius;

        let min_cx = min_bx >> 4;
        let max_cx = max_bx >> 4;
        let min_cz = min_bz >> 4;
        let max_cz = max_bz >> 4;

        (min_cx, max_cx, min_cz, max_cz)
    }

    /// Counts the total number of chunks inside this shape with the given center and radius.
    pub fn count_chunks(&self, center_x: i32, center_z: i32, radius: i32) -> u64 {
        let (min_cx, max_cx, min_cz, max_cz) = self.chunk_bounds(center_x, center_z, radius);
        let mut count = 0u64;

        for cx in min_cx..=max_cx {
            for cz in min_cz..=max_cz {
                if self.contains_chunk(center_x, center_z, cx, cz, radius) {
                    count += 1;
                }
            }
        }

        count.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_names() {
        assert_eq!(Shape::from_name("square"), Some(Shape::Square));
        assert_eq!(Shape::from_name("Circle"), Some(Shape::Circle));
        assert_eq!(Shape::from_name("DIAMOND"), Some(Shape::Diamond));
        assert_eq!(Shape::from_name("star"), Some(Shape::Star));
        assert_eq!(Shape::from_name("unknown"), None);
    }

    #[test]
    fn test_square_contains() {
        let shape = Shape::Square;
        assert!(shape.contains_block(0, 0, 100, 100, 100));
        assert!(!shape.contains_block(0, 0, 101, 100, 100));
    }

    #[test]
    fn test_circle_contains() {
        let shape = Shape::Circle;
        assert!(shape.contains_block(0, 0, 70, 70, 100)); // 70^2 + 70^2 = 9800 <= 10000
        assert!(!shape.contains_block(0, 0, 80, 80, 100)); // 80^2 + 80^2 = 12800 > 10000
    }

    #[test]
    fn test_chunk_counting() {
        let shape = Shape::Square;
        let count = shape.count_chunks(0, 0, 160);
        assert!(count > 0);
    }
}
