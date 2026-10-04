use crate::shape::Shape;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub world: String,
    pub shape: Shape,
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            world: "world".to_string(),
            shape: Shape::Square,
            center_x: 0,
            center_z: 0,
            radius: 500,
        }
    }
}

impl Selection {
    pub fn estimated_chunks(&self) -> u64 {
        self.shape.count_chunks(self.center_x, self.center_z, self.radius)
    }

    pub fn display_summary(&self) -> String {
        let total_chunks = self.estimated_chunks();
        let chunk_radius = (self.radius + 15) / 16;
        format!(
            "&6--- Chunky Selection ---\n\
             &7World: &b{}\n\
             &7Shape: &e{}\n\
             &7Center: &eX: {}, Z: {}\n\
             &7Radius: &e{} &7blocks (&e{} &7chunks)\n\
             &7Estimated Chunks: &a{}",
            self.world,
            self.shape.name(),
            self.center_x,
            self.center_z,
            self.radius,
            chunk_radius,
            total_chunks
        )
    }
}
