pub mod commands;
pub mod config;
pub mod iterator;
pub mod manager;
pub mod selection;
pub mod shape;
pub mod storage;
pub mod task;

use manager::{current_time_ms, manager};
use pumpkin_plugin_api::{
    permission::{Permission, PermissionDefault},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    scheduler::SchedulerExt,
    Context, Plugin, PluginMetadata,
};

pub struct ChunkyPlugin;

impl Plugin for ChunkyPlugin {
    fn new() -> Self {
        ChunkyPlugin
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "chunky".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["pop4959".into(), "Antigravity".into()],
            description: "Chunky chunk pre-generation plugin for PumpkinMC".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let data_folder = context.get_data_folder();
        {
            let mut mgr = manager().lock().unwrap();
            mgr.set_data_folder(data_folder);
        }

        // Register Chunky permission node with required namespace prefix "chunky:"
        let perm = Permission {
            node: "chunky:command".to_string(),
            description: "Allows using Chunky chunk pre-generation commands".to_string(),
            default: PermissionDefault::Allow,
            children: Vec::new(),
        };
        context.register_permission(&perm)?;

        // Register /chunky command tree
        let command = commands::build_chunky_command();
        context.register_command(command, "chunky:command");

        // Schedule 1-tick repeating task to drive the pregeneration loop and progress broadcaster
        context.schedule_repeating_task(1, 1, |server| {
            let now_ms = current_time_ms();
            manager().lock().unwrap().tick(&server, now_ms);
        });

        tracing::info!("Chunky pre-generation plugin for PumpkinMC loaded successfully!");
        Ok(())
    }

    fn on_unload(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let server = context.get_server();
        {
            let mut mgr = manager().lock().unwrap();
            let _ = mgr.pause_task(&server);
        }
        tracing::info!("Chunky unloaded.");
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(ChunkyPlugin);
