use crate::{iterator::SpiralIterator, shape::Shape};
use pumpkin_plugin_api::Server;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Running,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTask {
    pub world: String,
    pub shape: Shape,
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,
    pub total_chunks: u64,
    pub processed_chunks: u64,
    pub current_chunk_x: i32,
    pub current_chunk_z: i32,
    pub iterator: SpiralIterator,
    pub status: TaskStatus,
    pub start_time_ms: u64,
    pub last_report_time_ms: u64,
    pub last_report_chunks: u64,
    pub current_cps: f64,
    pub total_elapsed_ms: u64,
    pub last_step_time_ms: u64,
}

impl GenerationTask {
    pub fn new(world: String, shape: Shape, center_x: i32, center_z: i32, radius: i32, now_ms: u64) -> Self {
        let total_chunks = shape.count_chunks(center_x, center_z, radius);
        let iterator = SpiralIterator::new(center_x, center_z, radius, shape);
        let center_chunk_x = center_x >> 4;
        let center_chunk_z = center_z >> 4;

        Self {
            world,
            shape,
            center_x,
            center_z,
            radius,
            total_chunks,
            processed_chunks: 0,
            current_chunk_x: center_chunk_x,
            current_chunk_z: center_chunk_z,
            iterator,
            status: TaskStatus::Running,
            start_time_ms: now_ms,
            last_report_time_ms: now_ms,
            last_report_chunks: 0,
            current_cps: 0.0,
            total_elapsed_ms: 0,
            last_step_time_ms: now_ms,
        }
    }

    /// Advances the generation task by up to `batch_size` chunks.
    pub fn step(&mut self, server: &Server, batch_size: usize, now_ms: u64) -> bool {
        if self.status != TaskStatus::Running {
            return false;
        }

        if self.last_step_time_ms > 0 && now_ms > self.last_step_time_ms {
            self.total_elapsed_ms += now_ms - self.last_step_time_ms;
        }
        self.last_step_time_ms = now_ms;

        for _ in 0..batch_size {
            if let Some((cx, cz)) = self.iterator.next_chunk() {
                self.current_chunk_x = cx;
                self.current_chunk_z = cz;

                let block_x = cx * 16 + 8;
                let block_z = cz * 16 + 8;

                // Load chunk via Pumpkin's native chunk ticket system
                server.execute_command(
                    &format!("forceload add {} {}", block_x, block_z),
                    pumpkin_plugin_api::wit::pumpkin::plugin::server::CommandSender::Console,
                );
                // Unforce immediately so the chunk is saved/cached normally without unbounded forced list
                server.execute_command(
                    &format!("forceload remove {} {}", block_x, block_z),
                    pumpkin_plugin_api::wit::pumpkin::plugin::server::CommandSender::Console,
                );

                self.processed_chunks += 1;

                if self.processed_chunks >= self.total_chunks {
                    self.status = TaskStatus::Completed;
                    return true;
                }
            } else {
                self.status = TaskStatus::Completed;
                return true;
            }
        }

        false
    }

    /// Updates rolling Chunks Per Second (CPS) metrics.
    pub fn update_cps(&mut self, now_ms: u64) {
        if self.last_report_time_ms == 0 {
            self.last_report_time_ms = now_ms;
            self.last_report_chunks = self.processed_chunks;
            return;
        }

        let interval_ms = now_ms.saturating_sub(self.last_report_time_ms);
        if interval_ms >= 500 {
            let chunks_diff = self.processed_chunks.saturating_sub(self.last_report_chunks);
            let seconds = (interval_ms as f64) / 1000.0;
            let instant_cps = (chunks_diff as f64) / seconds;

            if self.current_cps == 0.0 {
                self.current_cps = instant_cps;
            } else {
                self.current_cps = self.current_cps * 0.3 + instant_cps * 0.7;
            }

            self.last_report_time_ms = now_ms;
            self.last_report_chunks = self.processed_chunks;
        }
    }

    /// Effective Chunks Per Second rate.
    pub fn effective_cps(&self) -> f64 {
        if self.current_cps > 0.05 {
            self.current_cps
        } else if self.total_elapsed_ms > 0 {
            (self.processed_chunks as f64) / ((self.total_elapsed_ms as f64) / 1000.0)
        } else {
            0.0
        }
    }

    /// Formatted ETA string (H:MM:SS) or "N/A".
    pub fn format_eta(&self) -> String {
        let remaining_chunks = self.total_chunks.saturating_sub(self.processed_chunks);
        let cps = self.effective_cps();

        if remaining_chunks == 0 {
            return "0:00:00".to_string();
        }

        if cps > 0.01 {
            let remaining_seconds = ((remaining_chunks as f64) / cps).round() as u64;
            let hours = remaining_seconds / 3600;
            let minutes = (remaining_seconds % 3600) / 60;
            let seconds = remaining_seconds % 60;
            format!("{}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            "N/A".to_string()
        }
    }

    /// Formatted progress message matching Chunky style.
    pub fn format_progress(&self) -> String {
        let percent = if self.total_chunks > 0 {
            (self.processed_chunks as f64 / self.total_chunks as f64) * 100.0
        } else {
            100.0
        };
        let eta = self.format_eta();
        let cps = self.effective_cps();

        format!(
            "&6[Chunky] &rTask running for &b{}&r. Processed: &e{}&r chunks (&e{:.2}%&r), ETA: &e{}&r, Rate: &a{:.1} cps&r, Current: &7{}, {}&r",
            self.world, self.processed_chunks, percent, eta, cps, self.current_chunk_x, self.current_chunk_z
        )
    }

    /// Formatted completion message matching Chunky style.
    pub fn format_finished(&self) -> String {
        let total_seconds = (self.total_elapsed_ms as f64 / 1000.0).round() as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let time_str = if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{}:{:02}", minutes, seconds)
        };
        let avg_cps = if self.total_elapsed_ms > 0 {
            (self.processed_chunks as f64) / (self.total_elapsed_ms as f64 / 1000.0)
        } else {
            0.0
        };

        format!(
            "&6[Chunky] &rTask finished for &b{}&r. Processed: &a{}&r chunks in &e{}&r (&a{:.1} cps&r).",
            self.world, self.processed_chunks, time_str, avg_cps
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eta_calculation() {
        let mut task = GenerationTask::new("world".to_string(), Shape::Square, 0, 0, 160, 1000);
        task.current_cps = 10.0;
        task.processed_chunks = 0;
        let eta = task.format_eta();
        assert_ne!(eta, "N/A");
    }

    #[test]
    fn test_progress_string() {
        let mut task = GenerationTask::new("world".to_string(), Shape::Square, 0, 0, 160, 1000);
        task.processed_chunks = 50;
        task.current_cps = 25.5;
        let line = task.format_progress();
        assert!(line.contains("Task running for"));
        assert!(line.contains("25.5 cps"));
    }
}
