use crate::{
    config::ChunkyConfig,
    selection::Selection,
    storage::{SavedState, Storage},
    task::{GenerationTask, TaskStatus},
};
use pumpkin_plugin_api::Server;
use std::sync::{Mutex, OnceLock};

pub fn current_time_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub struct ChunkyManager {
    pub data_folder: String,
    pub selection: Selection,
    pub current_task: Option<GenerationTask>,
    pub config: ChunkyConfig,
    pub last_broadcast_ms: u64,
}

impl ChunkyManager {
    pub fn new() -> Self {
        Self {
            data_folder: "plugins/data/Chunky".to_string(),
            selection: Selection::default(),
            current_task: None,
            config: ChunkyConfig::default(),
            last_broadcast_ms: 0,
        }
    }

    pub fn set_data_folder(&mut self, folder: String) {
        self.data_folder = folder;
        self.reload();
    }

    pub fn reload(&mut self) {
        let state = Storage::load(&self.data_folder);
        self.selection = state.selection;
        self.config = state.config;
        if self.current_task.is_none() {
            self.current_task = state.task;
        }
    }

    pub fn save(&self) {
        let state = SavedState {
            selection: self.selection.clone(),
            task: self.current_task.clone(),
            config: self.config.clone(),
        };
        Storage::save(&self.data_folder, &state);
    }

    pub fn start_task(&mut self, now_ms: u64) -> Result<String, String> {
        if let Some(ref task) = self.current_task {
            if task.status == TaskStatus::Running {
                return Err(format!(
                    "&c[Chunky] A task is already running for '{}'. Use /chunky pause or /chunky cancel first.",
                    task.world
                ));
            } else if task.status == TaskStatus::Paused {
                return Err(format!(
                    "&e[Chunky] A paused task exists for '{}' ({}/{} chunks). Use /chunky continue to resume, or /chunky cancel to discard.",
                    task.world, task.processed_chunks, task.total_chunks
                ));
            }
        }

        let task = GenerationTask::new(
            self.selection.world.clone(),
            self.selection.shape,
            self.selection.center_x,
            self.selection.center_z,
            self.selection.radius,
            now_ms,
        );

        let total_chunks = task.total_chunks;
        let world = task.world.clone();

        self.current_task = Some(task);
        self.last_broadcast_ms = now_ms;
        self.save();

        Ok(format!(
            "&6[Chunky] &rTask started for &b{}&r (&a{}&r chunks).",
            world, total_chunks
        ))
    }

    pub fn pause_task(&mut self, server: &Server) -> Result<String, String> {
        if let Some(ref mut task) = self.current_task {
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Paused;
                let world_name = task.world.clone();
                let cx = task.current_chunk_x;
                let cz = task.current_chunk_z;

                if let Some(w) = server.get_world_by_name(&world_name) {
                    let _ = w.save();
                }
                self.save();

                Ok(format!(
                    "&6[Chunky] &rTask paused for &b{}&r at &7{}, {}&r. Use &e/chunky continue&r to resume.",
                    world_name, cx, cz
                ))
            } else {
                Err("&c[Chunky] Task is already paused.".to_string())
            }
        } else {
            Err("&c[Chunky] No active task to pause.".to_string())
        }
    }

    pub fn continue_task(&mut self, now_ms: u64) -> Result<String, String> {
        if let Some(ref mut task) = self.current_task {
            if task.status == TaskStatus::Paused {
                task.status = TaskStatus::Running;
                task.last_step_time_ms = now_ms;
                task.last_report_time_ms = now_ms;
                task.last_report_chunks = task.processed_chunks;
                let world_name = task.world.clone();
                let cx = task.current_chunk_x;
                let cz = task.current_chunk_z;
                self.save();

                Ok(format!(
                    "&6[Chunky] &rTask resumed for &b{}&r from &7{}, {}&r.",
                    world_name, cx, cz
                ))
            } else if task.status == TaskStatus::Running {
                Err("&c[Chunky] Task is already running.".to_string())
            } else {
                Err("&c[Chunky] Task is not in a paused state.".to_string())
            }
        } else {
            Err("&c[Chunky] There is no task to continue. Use /chunky start to begin.".to_string())
        }
    }

    pub fn cancel_task(&mut self, server: &Server) -> Result<String, String> {
        if let Some(task) = self.current_task.take() {
            server.execute_command(
                "forceload remove all",
                pumpkin_plugin_api::wit::pumpkin::plugin::server::CommandSender::Console,
            );
            self.save();
            Ok(format!("&c[Chunky] &rTask cancelled for &b{}&r.", task.world))
        } else {
            Err("&c[Chunky] No active or paused task to cancel.".to_string())
        }
    }

    pub fn get_progress_message(&self) -> String {
        if let Some(ref task) = self.current_task {
            match task.status {
                TaskStatus::Running => task.format_progress(),
                TaskStatus::Paused => {
                    let percent = if task.total_chunks > 0 {
                        (task.processed_chunks as f64 / task.total_chunks as f64) * 100.0
                    } else {
                        100.0
                    };
                    format!(
                        "&6[Chunky] &rTask paused for &b{}&r. Processed: &e{}/{}&r chunks (&e{:.2}%&r). Use &e/chunky continue&r to resume.",
                        task.world, task.processed_chunks, task.total_chunks, percent
                    )
                }
                TaskStatus::Completed => {
                    "&a[Chunky] Last task completed successfully.".to_string()
                }
                TaskStatus::Cancelled => "&c[Chunky] Last task was cancelled.".to_string(),
            }
        } else {
            "&7[Chunky] No active task.".to_string()
        }
    }

    pub fn tick(&mut self, server: &Server, now_ms: u64) {
        let is_running = self
            .current_task
            .as_ref()
            .map_or(false, |t| t.status == TaskStatus::Running);

        if !is_running {
            return;
        }

        let mut completed = false;
        let mut progress_line = None;
        let mut finished_line = None;
        let mut should_save_world = false;
        let mut world_to_save = String::new();

        if let Some(ref mut task) = self.current_task {
            completed = task.step(server, self.config.chunks_per_tick, now_ms);
            task.update_cps(now_ms);

            world_to_save = task.world.clone();
            if task.processed_chunks % self.config.save_interval_chunks == 0 {
                should_save_world = true;
            }

            let quiet_ms = self.config.quiet_interval_seconds * 1000;
            if quiet_ms > 0 && now_ms.saturating_sub(self.last_broadcast_ms) >= quiet_ms {
                self.last_broadcast_ms = now_ms;
                progress_line = Some(task.format_progress());
            }

            if completed {
                finished_line = Some(task.format_finished());
                should_save_world = true;
            }
        }

        if let Some(line) = progress_line {
            server.broadcast(&line);
        }

        if should_save_world {
            if let Some(w) = server.get_world_by_name(&world_to_save) {
                let _ = w.save();
            }
        }

        if completed {
            if let Some(line) = finished_line {
                server.broadcast(&line);
            }
            self.current_task = None;
            self.save();
        }
    }
}

static MANAGER: OnceLock<Mutex<ChunkyManager>> = OnceLock::new();

pub fn manager() -> &'static Mutex<ChunkyManager> {
    MANAGER.get_or_init(|| Mutex::new(ChunkyManager::new()))
}
