use crate::shape::Shape;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiralIterator {
    pub center_x: i32,
    pub center_z: i32,
    pub center_chunk_x: i32,
    pub center_chunk_z: i32,
    pub radius: i32,
    pub shape: Shape,
    pub ring: i32,
    pub leg: u8,
    pub step: i32,
    pub max_ring: i32,
    pub done: bool,
}

impl SpiralIterator {
    pub fn new(center_x: i32, center_z: i32, radius: i32, shape: Shape) -> Self {
        let center_chunk_x = center_x >> 4;
        let center_chunk_z = center_z >> 4;

        let (min_cx, max_cx, min_cz, max_cz) = shape.chunk_bounds(center_x, center_z, radius);
        let max_dx = (max_cx - center_chunk_x).abs().max((min_cx - center_chunk_x).abs());
        let max_dz = (max_cz - center_chunk_z).abs().max((min_cz - center_chunk_z).abs());
        let max_ring = max_dx.max(max_dz);

        Self {
            center_x,
            center_z,
            center_chunk_x,
            center_chunk_z,
            radius,
            shape,
            ring: 0,
            leg: 0,
            step: 0,
            max_ring,
            done: false,
        }
    }

    /// Advances the spiral iterator and returns the next raw (chunk_x, chunk_z) on the spiral grid.
    fn next_raw_coordinate(&mut self) -> Option<(i32, i32)> {
        if self.done || self.ring > self.max_ring {
            self.done = true;
            return None;
        }

        if self.ring == 0 {
            self.ring = 1;
            self.leg = 0;
            self.step = 0;
            return Some((self.center_chunk_x, self.center_chunk_z));
        }

        let r = self.ring;
        let (u, v) = match self.leg {
            0 => (r, -r + 1 + self.step),
            1 => (r - 1 - self.step, r),
            2 => (-r, r - 1 - self.step),
            3 => (-r + 1 + self.step, -r),
            _ => (0, 0),
        };

        let chunk_x = self.center_chunk_x + u;
        let chunk_z = self.center_chunk_z + v;

        // Advance step
        self.step += 1;
        if self.step >= 2 * r {
            self.step = 0;
            self.leg += 1;
            if self.leg >= 4 {
                self.leg = 0;
                self.ring += 1;
                if self.ring > self.max_ring {
                    self.done = true;
                }
            }
        }

        Some((chunk_x, chunk_z))
    }

    /// Returns the next chunk in the spiral that falls within the selection shape.
    pub fn next_chunk(&mut self) -> Option<(i32, i32)> {
        while let Some((cx, cz)) = self.next_raw_coordinate() {
            if self.shape.contains_chunk(self.center_x, self.center_z, cx, cz, self.radius) {
                return Some((cx, cz));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_spiral_no_duplicates_and_exact_match() {
        let shape = Shape::Square;
        let radius = 48; // 3 chunks radius
        let mut iter = SpiralIterator::new(0, 0, radius, shape);

        let mut seen = HashSet::new();
        let mut count = 0;

        while let Some((cx, cz)) = iter.next_chunk() {
            assert!(seen.insert((cx, cz)), "Duplicate chunk ({}, {})", cx, cz);
            count += 1;
        }

        let expected_count = shape.count_chunks(0, 0, radius);
        assert_eq!(count as u64, expected_count);
    }

    #[test]
    fn test_spiral_circle() {
        let shape = Shape::Circle;
        let radius = 64;
        let mut iter = SpiralIterator::new(0, 0, radius, shape);

        let mut count = 0;
        let mut seen = HashSet::new();
        while let Some((cx, cz)) = iter.next_chunk() {
            assert!(seen.insert((cx, cz)));
            count += 1;
        }

        let expected_count = shape.count_chunks(0, 0, radius);
        assert_eq!(count as u64, expected_count);
    }

    #[test]
    fn test_serialization_resume() {
        let shape = Shape::Square;
        let mut iter = SpiralIterator::new(0, 0, 80, shape);

        // Advance 10 chunks
        for _ in 0..10 {
            iter.next_chunk();
        }

        let json = serde_json::to_string(&iter).unwrap();
        let mut resumed: SpiralIterator = serde_json::from_str(&json).unwrap();

        // Ensure next chunks match
        let next1 = iter.next_chunk();
        let next2 = resumed.next_chunk();
        assert_eq!(next1, next2);
    }
}
