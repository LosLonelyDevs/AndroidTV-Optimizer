//! Shield Optimizer v2 — desktop Tauri entry point.
//!
//! Shared engine and driver-generic commands live in `shield_optimizer_core`.
//! This crate keeps the desktop-only subprocess ADB driver, installer/updater,
//! host-network scan, and file-path based commands.

// clippy 1.99 added `double_must_use`, which fires on the futures that
// `#[async_trait]` generates for every trait method. The attribute is the
// macro's, not ours, so there is nothing to change at the call sites.
#![allow(clippy::double_must_use)]
pub mod activity_log;
pub mod adb;
pub mod commands;
pub mod session;
pub use shield_optimizer_core::engine;

use std::path::PathBuf;
use std::sync::Arc;

use adb::SubprocessAdb;
use commands::quit::{self, AdbQuitControl};
use commands::{
    app_report, apps, audio, backup, boot_tweaks, console, devices, diagnostics, display, files,
    health, install, loader, media_apps, mirror, quick_install, recording, safety_overrides, scan,
    sideload, system, update, AppState,
};
use shield_optimizer_core::adb::{AdbDriver, AdbError, AdbOutput, AdbResult};
use shield_optimizer_core::commands::{apps as core_apps, health as core_health};
use shield_optimizer_core::commands::{
    input, launcher, logcat, optimize, reboot, recovery, screenshot, shell, snapshot, tuning,
};
use shield_optimizer_core::license::Entitlement;
use tauri::Manager;

/// Resolve the OS-appropriate app data root (snapshots live in a `snapshots`
/// subdirectory).
///
/// macOS: `~/Library/Application Support/ShieldOptimizer`
/// Linux: `~/.local/share/ShieldOptimizer`
/// Windows: `%LOCALAPPDATA%/ShieldOptimizer`
fn default_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ShieldOptimizer")
}

/// Driver used when no adb binary could be discovered at startup. Every call
/// returns the actionable `BinaryNotFound` error so the UI tells the user
/// exactly what to do.
struct NoAdbDriver;

#[async_trait::async_trait]
impl AdbDriver for NoAdbDriver {
    async fn raw(&self, _args: &[&str]) -> AdbResult<AdbOutput> {
        Err(AdbError::BinaryNotFound)
    }

    async fn shell(&self, _serial: &str, _command: &str) -> AdbResult<AdbOutput> {
        Err(AdbError::BinaryNotFound)
    }
}

fn default_state(app_lists: engine::AppListBundle, data_dir: PathBuf) -> AppState {
    let adb: Arc<dyn AdbDriver> = match adb::cached_adb_binary() {
        Some(path) => {
            tracing::info!(adb = %path.display(), "adb located");
            Arc::new(SubprocessAdb::new(path))
        }
        None => {
            tracing::warn!("no adb binary located; commands will return BinaryNotFound");
            Arc::new(NoAdbDriver)
        }
    };
    // The desktop app has no paywall; every feature is unlocked.
    AppState::new(adb, app_lists, data_dir).with_entitlement(Entitlement::Pro)
}

/// Every command the desktop frontend can invoke. One list, shared by the app
/// and the dev-only E2E server (`examples/e2e_server.rs`), so the harness
/// dispatches through exactly the handler table the shipped app uses.
pub fn invoke_handler<R: tauri::Runtime>(
) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        devices::list_devices,
        devices::device_profile,
        devices::connect_device,
        devices::disconnect_device,
        devices::pair_device,
        devices::forget_device,
        devices::probe_paired_connect,
        devices::connect_paired,
        scan::local_address_for,
        devices::rename_device,
        health::health_report,
        core_health::media_report,
        core_health::resource_sample,
        health::app_list_for_device,
        health::report_all,
        install::adb_status,
        install::install_adb,
        install::restart_adb,
        quit::get_stop_adb_on_quit,
        quit::set_stop_adb_on_quit,
        scan::scan_network,
        launcher::list_launchers,
        launcher::current_launcher,
        launcher::channel_provider_disabled,
        launcher::set_default_launcher,
        launcher::disable_launcher,
        launcher::set_home_any,
        launcher::disable_stock_launcher,
        launcher::disable_setup_helper,
        core_apps::disable_package,
        core_apps::enable_package,
        core_apps::force_stop,
        screenshot::take_screenshot,
        core_apps::uninstall_package,
        core_apps::reinstall_existing,
        core_apps::open_play_store,
        core_apps::package_states,
        core_apps::list_other_packages,
        apps::list_installed_packages,
        core_apps::app_memory_map,
        core_apps::app_storage_map,
        core_apps::app_apk_size,
        apps::app_usage_map,
        core_apps::safety_info,
        core_apps::process_safety_info,
        core_apps::trim_caches,
        core_apps::app_permission_state,
        core_apps::set_app_permission,
        core_apps::set_app_op,
        core_apps::get_app_op,
        input::send_text,
        input::send_key,
        input::open_settings,
        sideload::install_apk,
        sideload::list_apks_in_folder,
        sideload::inspect_apk,
        backup::backup_apk,
        backup::clone_app,
        files::list_dir,
        files::pull_file,
        files::push_file,
        files::delete_path,
        files::find_files,
        files::copy_file_to_device,
        snapshot::list_snapshots,
        snapshot::save_snapshot,
        snapshot::preview_apply,
        snapshot::apply_snapshot,
        snapshot::delete_snapshot,
        snapshot::snapshot_dir_path,
        recovery::panic_recovery,
        reboot::reboot_device,
        shell::run_shell,
        logcat::read_device_logs,
        tuning::get_tweaks,
        tuning::write_setting,
        tuning::set_display_scaling,
        tuning::get_display_scaling,
        tuning::get_private_dns,
        tuning::set_private_dns,
        optimize::prepare_optimize,
        optimize::apply_performance_settings,
        update::check_for_update,
        app_report::save_app_report,
        diagnostics::collect_diagnostics,
        diagnostics::set_debug_logging,
        diagnostics::get_debug_logging,
        diagnostics::log_dir_path,
        diagnostics::open_log_dir,
        diagnostics::log_ui_event,
        devices::ping_device,
        apps::installed_package_versions,
        safety_overrides::list_safety_overrides,
        safety_overrides::set_safety_override,
        apps::clear_app_cache,
        apps::clear_app_data,
        apps::kill_all_background,
        files::storage_usage,
        display::display_report,
        audio::audio_report,
        audio::set_surround_output,
        audio::set_surround_formats,
        media_apps::detect_media_apps,
        media_apps::stage_kodi_config,
        media_apps::grant_write_secure_settings,
        system::system_info,
        system::open_system_screen,
        system::power_action,
        system::launch_app,
        system::repair_ntp,
        system::compile_speed_profile,
        console::run_console_shell,
        console::activity_tail,
        recording::start_recording,
        recording::stop_recording,
        quick_install::list_quick_apps,
        quick_install::install_quick_app,
        quick_install::setup_shizuku,
        quick_install::shizuku_status,
        boot_tweaks::list_boot_tweaks,
        boot_tweaks::set_boot_tweak,
        boot_tweaks::reapply_boot_tweaks,
        system::set_play_protect,
        mirror::mirror_screen,
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Some Wayland/Mesa setups crash WebKitGTK's DMABUF renderer on startup
    // ("Could not create default EGL display: EGL_BAD_PARAMETER"), leaving a
    // blank window. Disabling that renderer falls back to a path that works
    // everywhere. Honor an existing value so power users can still force it on.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // stdout *and* a rolling daily file under the data dir. A packaged app has
    // nowhere for stdout to go, so the file is the only copy that survives on
    // the machines where a bug actually happens.
    let data_dir = default_data_dir();
    let log_control = diagnostics::init_logging(&data_dir);
    session::install(log_control.dir(), log_control.debug_enabled());

    let app_lists = match loader::load_embedded_app_lists() {
        Ok(lists) => {
            tracing::info!(total = lists.total(), "app lists loaded");
            lists
        }
        Err(e) => {
            // Surface the build-time mistake but don't crash the GUI — let the
            // frontend show an empty state.
            tracing::error!(error = %e, "failed to load embedded app lists");
            engine::AppListBundle::default()
        }
    };

    let state = default_state(app_lists, data_dir).with_known_names(loader::load_known_names());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(state)
        .manage(log_control)
        .manage(AdbQuitControl::default())
        .invoke_handler(invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // An update restart arrives as this exit code, ahead of `Exit`.
            tauri::RunEvent::ExitRequested {
                code: Some(tauri::RESTART_EXIT_CODE),
                ..
            } => app.state::<AdbQuitControl>().note_restart(),
            tauri::RunEvent::Exit => {
                let state = app.state::<AppState>();
                let control = app.state::<AdbQuitControl>();
                tauri::async_runtime::block_on(quit::stop_adb_server_on_quit(
                    state.inner(),
                    control.inner(),
                ));
            }
            _ => {}
        });
}
