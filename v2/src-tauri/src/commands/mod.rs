//! Desktop-only Tauri command handlers.
//!
//! Shared command handlers live in `shield_optimizer_core::commands`; this
//! module keeps commands that depend on desktop filesystem paths, platform-tools
//! installation, the desktop updater, or host-network scanning.

pub mod app_report;
pub mod apps;
pub mod audio;
pub mod backup;
pub mod boot_tweaks;
pub mod console;
pub mod devices;
pub mod diagnostics;
pub mod display;
pub mod files;
pub mod health;
pub mod install;
pub mod loader;
pub mod media_apps;
pub mod mirror;
pub mod quick_install;
pub mod quit;
pub mod recording;
pub mod safety_overrides;
pub mod scan;
pub mod sideload;
pub mod system;
pub mod update;

pub use shield_optimizer_core::commands::quote_shell_arg;
#[cfg(test)]
pub use shield_optimizer_core::commands::test_support;
pub use shield_optimizer_core::commands::AppState;
pub use shield_optimizer_core::commands::{
    input, launcher, optimize, reboot, recovery, screenshot, snapshot, tuning,
};
