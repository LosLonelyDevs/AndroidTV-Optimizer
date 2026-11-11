//! Health report and display-mode commands.

use serde::Serialize;
use tauri::State;

use crate::adb::{parse_diskstats, DiskStats};

use super::AppState;
pub use atv_optimizer_core::commands::health::{media_report, resource_sample};

/// Single payload for the Health Report view — everything the UI needs in one
/// round trip.
#[derive(Serialize)]
pub struct HealthReport {
    #[serde(flatten)]
    pub base: atv_optimizer_core::commands::health::HealthReport,
    pub disk: DiskStats,
}

/// How many packages the storage table shows.
const TOP_STORAGE: usize = 25;

/// Extend the shared health report with the desktop storage breakdown.
#[tauri::command]
pub async fn health_report(
    state: State<'_, AppState>,
    serial: String,
) -> Result<HealthReport, String> {
    health_report_for(state.inner(), &serial).await
}

/// `app_list_for_device` — return the merged app list for a given device type.
/// Read-only; doesn't touch the device. Used by the Profile view.
#[tauri::command]
pub async fn app_list_for_device(
    state: State<'_, AppState>,
    device_type: crate::engine::DeviceType,
) -> Result<Vec<crate::engine::types::AppEntry>, String> {
    Ok(state.app_lists.for_device(device_type))
}

#[derive(Serialize)]
pub struct DeviceReport {
    pub serial: String,
    pub name: String,
    /// `Some(report)` on success, `None` if the per-device health call failed.
    pub report: Option<HealthReport>,
    pub error: Option<String>,
}

/// `report_all` — iterate every authorized device and run a health report on
/// each. Mirrors v1's main-menu "Report All" (§2.1). Returns one entry per
/// device, including failures so the UI can show them inline.
#[tauri::command]
pub async fn report_all(state: State<'_, AppState>) -> Result<Vec<DeviceReport>, String> {
    let devices = crate::commands::devices::list_devices_impl(state.inner()).await?;
    let mut out = Vec::with_capacity(devices.len());
    for d in devices {
        if !matches!(d.status, crate::engine::types::DeviceStatus::Device) {
            out.push(DeviceReport {
                serial: d.serial.clone(),
                name: d.name.clone(),
                report: None,
                error: Some(format!("device not authorized (state: {:?})", d.status)),
            });
            continue;
        }
        // Run inline (not joined) to avoid hammering the same adb daemon with
        // many parallel dumpsys calls — keeps load and timeout risk low.
        match health_report_for(state.inner(), &d.serial).await {
            Ok(report) => out.push(DeviceReport {
                serial: d.serial,
                name: d.name,
                report: Some(report),
                error: None,
            }),
            Err(e) => out.push(DeviceReport {
                serial: d.serial,
                name: d.name,
                report: None,
                error: Some(e),
            }),
        }
    }
    Ok(out)
}

/// Shared implementation for `health_report` and `report_all`. Takes
/// `&AppState` so both callers avoid juggling Tauri's State lifetime.
async fn health_report_for(state: &AppState, serial: &str) -> Result<HealthReport, String> {
    let base = atv_optimizer_core::commands::health::health_report_for(state, serial).await?;
    let adb = state.adb_snapshot().await;
    let disk_text = adb
        .shell(serial, "dumpsys diskstats")
        .await
        .map(|out| out.stdout)
        .unwrap_or_default();
    let mut disk = parse_diskstats(&disk_text);
    disk.packages.truncate(TOP_STORAGE);
    Ok(HealthReport { base, disk })
}
