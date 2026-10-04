use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkyConfig {
    /// How often (in seconds) the progress update line is broadcast.
    pub quiet_interval_seconds: u64,
    /// Number of chunks generated per game tick.
    pub chunks_per_tick: usize,
    /// Number of chunks processed between world saves.
    pub save_interval_chunks: u64,
}

impl Default for ChunkyConfig {
    fn default() -> Self {
        Self {
            quiet_interval_seconds: 2,
            chunks_per_tick: 4,
            save_interval_chunks: 500,
        }
    }
}
