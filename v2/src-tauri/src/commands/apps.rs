use super::AppState;
use crate::adb::{parse_uptime_secs, parse_usage_stats, AppUsage};
use crate::engine::{classify_safety, is_valid_package_name, Safety};
use serde::Serialize;
pub use shield_optimizer_core::commands::apps::ActionResult;
use std::collections::HashMap;
use tauri::State;

/// The version of an installed package, read from `dumpsys package <pkg>`.
/// `version_code` is what Android compares for upgrade/downgrade; `version_name`
/// is the human string shown in the UI.
#[derive(Serialize)]
pub struct InstalledVersion {
    pub version_code: Option<i64>,
    pub version_name: Option<String>,
}

/// `installed_package_versions` — for each package in `packages`, the version
/// currently installed on the device (absent from the map when not installed).
/// Powers the Install-APK tab's Upgrade / Downgrade / Reinstall labelling: the
/// picked APK's manifest version is compared against what's actually on the box.
///
/// One `dumpsys package <pkg>` per package, run in bounded-concurrency batches
/// so a folder full of installed APKs doesn't spawn dozens of shells at once.
#[tauri::command]
pub async fn installed_package_versions(
    state: State<'_, AppState>,
    serial: String,
    packages: Vec<String>,
) -> Result<HashMap<String, InstalledVersion>, String> {
    installed_package_versions_impl(state.inner(), &serial, &packages).await
}

pub async fn installed_package_versions_impl(
    state: &AppState,
    serial: &str,
    packages: &[String],
) -> Result<HashMap<String, InstalledVersion>, String> {
    /// Simultaneous `dumpsys package` calls to keep in flight.
    const CONCURRENCY: usize = 8;

    let adb = state.adb_snapshot().await;
    // Skip anything that isn't a well-formed package id before it reaches the
    // shell — this list originates from APK manifests, not `pm list`.
    let valid: Vec<&String> = packages
        .iter()
        .filter(|p| is_valid_package_name(p))
        .collect();

    let mut out = HashMap::new();
    for chunk in valid.chunks(CONCURRENCY) {
        let futs = chunk.iter().map(|&pkg| {
            let adb = adb.clone();
            async move {
                let text = adb
                    .shell(serial, &format!("dumpsys package {pkg}"))
                    .await
                    .map(|o| o.stdout)
                    .unwrap_or_default();
                (pkg.clone(), text)
            }
        });
        for (pkg, text) in futures_util::future::join_all(futs).await {
            let version_code = parse_dumpsys_version_code(&text);
            let version_name = parse_dumpsys_version_name(&text);
            // Report only packages that are actually present — an absent package
            // yields no version fields (dumpsys prints "Unable to find …").
            if version_code.is_some() || version_name.is_some() {
                out.insert(
                    pkg,
                    InstalledVersion {
                        version_code,
                        version_name,
                    },
                );
            }
        }
    }
    Ok(out)
}

/// First `versionCode=<n>` in `dumpsys package` output (the primary package
/// block lists it before any secondary users). Ignores the trailing
/// ` minSdk=… targetSdk=…` on the same line.
fn parse_dumpsys_version_code(dumpsys: &str) -> Option<i64> {
    dumpsys.lines().find_map(|l| {
        let rest = l.trim().strip_prefix("versionCode=")?;
        rest.split_whitespace().next()?.parse::<i64>().ok()
    })
}

/// First `versionName=<v>` in `dumpsys package` output.
fn parse_dumpsys_version_name(dumpsys: &str) -> Option<String> {
    dumpsys.lines().find_map(|l| {
        let v = l.trim().strip_prefix("versionName=")?.trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// Per-package usage plus the window it was observed over.
#[derive(Serialize)]
pub struct UsageReport {
    /// Device uptime in seconds, or `None` if `/proc/uptime` was unreadable.
    ///
    /// This is the honest bound on the whole dataset. `dumpsys usagestats`
    /// prints only in-memory buckets and Android rebuilds them at boot — every
    /// bucket, "yearly" included, starts at the last reboot. So an absent
    /// record means "not opened since boot", which on a box rebooted an hour
    /// ago says nothing about whether the app is used. The UI needs this to
    /// avoid flagging everything as an unused-removal candidate.
    pub window_secs: Option<u64>,
    pub entries: HashMap<String, AppUsage>,
}

/// `app_usage_map` — package → last-used + launch count, from a single
/// `dumpsys usagestats`, plus the uptime window that data covers. Powers the
/// "Review / remove if unused" signal: an app not opened over a long enough
/// window is a candidate to disable/uninstall.
#[tauri::command]
pub async fn app_usage_map(
    state: State<'_, AppState>,
    serial: String,
) -> Result<UsageReport, String> {
    app_usage_map_impl(state.inner(), &serial).await
}

pub async fn app_usage_map_impl(state: &AppState, serial: &str) -> Result<UsageReport, String> {
    let adb = state.adb_snapshot().await;
    let (stats_res, uptime_res) = tokio::join!(
        adb.shell(serial, "dumpsys usagestats"),
        adb.shell(serial, "cat /proc/uptime"),
    );
    let stats = stats_res.map_err(|e| format!("dumpsys usagestats: {e}"))?;
    // Best-effort: a missing uptime just means the UI can't calibrate staleness.
    let window_secs = uptime_res
        .ok()
        .and_then(|o| parse_uptime_secs(o.stdout.trim()));
    Ok(UsageReport {
        window_secs,
        entries: parse_usage_stats(&stats.stdout),
    })
}

/// `clear_app_cache` — `pm clear-cache <pkg>`. Drops the app's cached files
/// without touching its data, accounts, or settings; the cache rebuilds on next
/// launch, so (like `trim_caches`) nothing persists and no safety gate beyond
/// name validation is needed.
#[tauri::command]
pub async fn clear_app_cache(
    state: State<'_, AppState>,
    serial: String,
    package: String,
) -> Result<ActionResult, String> {
    clear_app_cache_impl(state.inner(), &serial, &package).await
}

pub async fn clear_app_cache_impl(
    state: &AppState,
    serial: &str,
    package: &str,
) -> Result<ActionResult, String> {
    if let Some(rejection) = reject_invalid_package(package) {
        return Ok(rejection);
    }
    run(state, serial, &format!("pm clear-cache {package}")).await
}

/// `clear_app_data` — `pm clear <pkg>`. Wipes the app's data, cache, accounts,
/// and settings — it resets the app to a fresh-install state. Destructive and
/// not reversible, so the UI gates it behind a loud confirm; here we apply the
/// same NEVER_DISABLE refusal as `disable_package`, since clearing a
/// framework/provider's data can brick the device or sign the user out
/// everywhere.
#[tauri::command]
pub async fn clear_app_data(
    state: State<'_, AppState>,
    serial: String,
    package: String,
) -> Result<ActionResult, String> {
    clear_app_data_impl(state.inner(), &serial, &package).await
}

pub async fn clear_app_data_impl(
    state: &AppState,
    serial: &str,
    package: &str,
) -> Result<ActionResult, String> {
    if let Some(rejection) = reject_invalid_package(package) {
        return Ok(rejection);
    }
    if let Safety::NeverDisable { reason, .. } = classify_safety(package) {
        return Ok(ActionResult {
            ok: false,
            message: format!("Refusing to clear data for {package}: {reason}"),
        });
    }
    run(state, serial, &format!("pm clear {package}")).await
}

/// `kill_all_background` — `am kill-all`. Asks the activity manager to kill
/// every background process it's willing to (foreground apps survive); they
/// restart on next use, so nothing persists and no safety gate is needed. The
/// device-wide companion to per-app `force_stop`.
#[tauri::command]
pub async fn kill_all_background(
    state: State<'_, AppState>,
    serial: String,
) -> Result<ActionResult, String> {
    run(&state, &serial, "am kill-all").await
}

#[derive(Serialize)]
pub struct InstalledPackage {
    #[serde(flatten)]
    pub entry: shield_optimizer_core::commands::apps::OtherPackage,
    pub catalog: bool,
}

#[tauri::command]
pub async fn list_installed_packages(
    state: State<'_, AppState>,
    serial: String,
) -> Result<Vec<InstalledPackage>, String> {
    let entries =
        shield_optimizer_core::commands::apps::list_installed_packages_impl(state.inner(), &serial)
            .await?;
    Ok(entries
        .into_iter()
        .map(|entry| {
            let catalog = state
                .app_lists
                .common
                .iter()
                .chain(&state.app_lists.shield)
                .chain(&state.app_lists.googletv)
                .any(|a| a.package == entry.package);
            InstalledPackage { entry, catalog }
        })
        .collect())
}

fn reject_invalid_package(package: &str) -> Option<ActionResult> {
    (!is_valid_package_name(package)).then(|| ActionResult {
        ok: false,
        message: format!("Invalid package name: {package:?}"),
    })
}

async fn run(state: &AppState, serial: &str, command: &str) -> Result<ActionResult, String> {
    let out = state
        .adb_snapshot()
        .await
        .shell(serial, command)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ActionResult {
        ok: out.success() && !out.shell_reported_failure(),
        message: out.combined().trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_dumpsys_version_fields() {
        let dump = "  Package [com.enai.launcher] (abc123):\n\
                     versionCode=470002 minSdk=22 targetSdk=30\n\
                     versionName=4.70.2\n";
        assert_eq!(parse_dumpsys_version_code(dump), Some(470002));
        assert_eq!(parse_dumpsys_version_name(dump).as_deref(), Some("4.70.2"));
        assert_eq!(parse_dumpsys_version_code("no version here"), None);
        assert_eq!(parse_dumpsys_version_name("no version here"), None);
    }

    #[tokio::test]
    async fn installed_package_versions_reports_installed_only() {
        use crate::commands::test_support::{state_with, MockAdb};

        let dump = "  Package [com.installed.app] (x):\n\
                     versionCode=12345 minSdk=21 targetSdk=33\n\
                     versionName=1.2.3\n";
        // Key the fixture to the installed id so the absent package falls through
        // to the mock's empty default reply and is omitted from the result.
        let state =
            state_with(MockAdb::default().on_shell("dumpsys package com.installed.app", dump));
        let map = installed_package_versions_impl(
            &state,
            "serial",
            &[
                "com.installed.app".to_string(),
                "com.absent.app".to_string(),
            ],
        )
        .await
        .unwrap();
        let v = map.get("com.installed.app").expect("installed pkg present");
        assert_eq!(v.version_code, Some(12345));
        assert_eq!(v.version_name.as_deref(), Some("1.2.3"));
        assert!(!map.contains_key("com.absent.app"));
    }

    #[tokio::test]
    async fn app_usage_map_reports_last_used_and_uptime_window() {
        use crate::commands::test_support::{state_with, MockAdb};

        let usage = "package=com.netflix.ninja lastTimeUsed=\"2026-06-01 09:00:00\" appLaunchCount=5\n\
                     package=com.unused.app lastTimeUsed=\"1969-12-31 18:00:00\" appLaunchCount=0\n";
        let state = state_with(
            MockAdb::default()
                .on_shell("dumpsys usagestats", usage)
                .on_shell("/proc/uptime", "66526.81 234153.95\n"),
        );
        let report = app_usage_map_impl(&state, "serial").await.unwrap();
        assert_eq!(
            report
                .entries
                .get("com.netflix.ninja")
                .and_then(|u| u.last_used.as_deref()),
            Some("2026-06-01 09:00:00")
        );
        assert_eq!(
            report
                .entries
                .get("com.unused.app")
                .and_then(|u| u.last_used.clone()),
            None
        );
        // The window is what lets the UI tell "not used in ages" apart from
        // "the box rebooted an hour ago and history restarted".
        assert_eq!(report.window_secs, Some(66526));
    }

    #[tokio::test]
    async fn app_usage_map_survives_unreadable_uptime() {
        use crate::commands::test_support::{state_with, MockAdb};

        // No rule for /proc/uptime — the mock errors, and usage must still load.
        let state = state_with(MockAdb::default().on_shell(
            "dumpsys usagestats",
            "package=com.netflix.ninja lastTimeUsed=\"2026-06-01 09:00:00\" appLaunchCount=5\n",
        ));
        let report = app_usage_map_impl(&state, "serial").await.unwrap();
        assert!(report.entries.contains_key("com.netflix.ninja"));
        assert_eq!(report.window_secs, None);
    }

    #[tokio::test]
    async fn clear_app_data_refuses_never_disable() {
        use crate::commands::test_support::{state_with, MockAdb};

        // Capture the shell log so we can prove no `pm clear` ever reached the
        // device for a bricking-tier package.
        let mock = MockAdb::default();
        let log = mock.shell_log();
        let state = state_with(mock);

        let r = clear_app_data_impl(&state, "serial", "com.android.systemui")
            .await
            .unwrap();
        assert!(
            !r.ok,
            "clearing data for a NEVER_DISABLE package must be refused"
        );
        assert!(
            log.lock().unwrap().is_empty(),
            "no shell command should be sent for a refused clear-data"
        );
    }

    #[tokio::test]
    async fn clear_app_cache_runs_for_normal_package() {
        use crate::commands::test_support::{state_with, MockAdb};

        let mock = MockAdb::default();
        let log = mock.shell_log();
        let state = state_with(mock);

        let r = clear_app_cache_impl(&state, "serial", "com.netflix.ninja")
            .await
            .unwrap();
        assert!(r.ok, "clear-cache on a safe package should succeed");
        assert!(
            log.lock()
                .unwrap()
                .iter()
                .any(|c| c == "pm clear-cache com.netflix.ninja"),
            "the clear-cache command should reach the device"
        );
    }
}
