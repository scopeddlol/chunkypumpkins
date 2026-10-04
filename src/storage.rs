use crate::{config::ChunkyConfig, selection::Selection, task::GenerationTask};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedState {
    pub selection: Selection,
    pub task: Option<GenerationTask>,
    pub config: ChunkyConfig,
}

pub struct Storage;

impl Storage {
    pub fn load(data_folder: &str) -> SavedState {
        let path = Path::new(data_folder).join("state.json");
        if !path.exists() {
            return SavedState::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<SavedState>(&content) {
                Ok(state) => {
                    tracing::info!("Loaded Chunky state from {:?}", path);
                    state
                }
                Err(err) => {
                    tracing::warn!("Failed to parse state file {:?}: {}", path, err);
                    SavedState::default()
                }
            },
            Err(err) => {
                tracing::warn!("Failed to read state file {:?}: {}", path, err);
                SavedState::default()
            }
        }
    }

    pub fn save(data_folder: &str, state: &SavedState) {
        let folder = Path::new(data_folder);
        if let Err(err) = fs::create_dir_all(folder) {
            tracing::error!("Failed to create data folder {:?}: {}", folder, err);
            return;
        }

        let path = folder.join("state.json");
        match serde_json::to_string_pretty(state) {
            Ok(json) => {
                if let Err(err) = fs::write(&path, json) {
                    tracing::error!("Failed to write state file {:?}: {}", path, err);
                }
            }
            Err(err) => {
                tracing::error!("Failed to serialize state: {}", err);
            }
        }
    }
}
