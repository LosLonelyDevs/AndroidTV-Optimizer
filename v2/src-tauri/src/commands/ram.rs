use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::adb::AdbDriver;
use crate::engine::ram::{self, BackgroundMode, RamReading};
use crate::engine::{is_valid_package_name, Safety};
use atv_optimizer_core::commands::apps::safety_for;
use atv_optimizer_core::commands::devices::{device_profile_impl, verified_hardware_id};
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;

use super::{quote_shell_arg, AppState};

static MUTATION: Mutex<()> = Mutex::const_new(());
const OP: &str = "RUN_ANY_IN_BACKGROUND";

#[derive(Serialize)]
pub struct RamApp {
    package: String,
    name: String,
    pss_mb: Option<f64>,
    blocked_reason: Option<String>,
    can_undo: bool,
}

#[derive(Serialize)]
pub struct RamReport {
    hardware_id: String,
    user: u32,
    sampled_at_ms: u64,
    reading: RamReading,
    apps: Vec<RamApp>,
    notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Undo {
    schema_version: u32,
    hardware_id: String,
    user: u32,
    package: String,
    uid: u32,
    previous: BackgroundMode,
}

struct Context {
    hardware_id: String,
    user: u32,
    packages: BTreeMap<String, u32>,
    eligible: BTreeMap<String, Option<String>>,
}

async fn shell(adb: &dyn AdbDriver, serial: &str, command: &str) -> Result<String, String> {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        adb.shell_bounded(serial, command),
    )
    .await
    .map_err(|_| "RAM command timed out.".to_string())?
    .map_err(|e| e.to_string())?;
    if result.exit_code != Some(0)
        || !matches!(
            result.termination,
            atv_optimizer_core::adb::driver::ShellTermination::Completed
        )
        || !result.stderr.trim().is_empty()
        || result.stdout.contains("SecurityException")
        || result.stdout.contains("Permission Denial")
        || result.stdout.lines().any(|line| {
            ["Error:", "Unknown command", "Exception"]
                .iter()
                .any(|prefix| line.trim().starts_with(prefix))
        })
    {
        return Err("TV could not complete the RAM command. No success was assumed.".into());
    }
    Ok(result.stdout)
}

async fn identity(state: &AppState, serial: &str) -> Result<String, String> {
    let device = device_profile_impl(state, serial).await?;
    device
        .properties
        .as_ref()
        .and_then(|p| verified_hardware_id(&p.serial_number))
        .map(str::to_string)
        .ok_or_else(|| "RAM actions require a verified hardware ID.".into())
}

async fn context(
    state: &AppState,
    adb: &dyn AdbDriver,
    serial: &str,
    expected: &str,
) -> Result<Context, String> {
    let hardware_id = identity(state, serial).await?;
    if hardware_id != expected {
        return Err("The connected TV changed. Refresh the RAM report.".into());
    }
    let user: u32 = shell(adb, serial, "am get-current-user")
        .await?
        .trim()
        .parse()
        .map_err(|_| "The Android user could not be verified.")?;
    let all = shell(adb, serial, &format!("pm list packages -U --user {user}")).await?;
    let packages = ram::package_uids(&all);
    if packages.is_empty() {
        return Err("Package ownership was not reported.".into());
    }
    let third_party = shell(
        adb,
        serial,
        &format!("pm list packages -3 -e -U --user {user}"),
    )
    .await?;
    let third_party = ram::package_uids(&third_party);
    let home = shell(adb, serial, &format!("cmd package resolve-activity --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.HOME")).await.ok().and_then(|s| ram::component_package(&s));
    let foreground = shell(adb, serial, "dumpsys activity activities")
        .await
        .ok()
        .and_then(|s| ram::foreground_package(&s));
    let services = shell(adb, serial, "dumpsys activity services").await.ok();
    let media = shell(adb, serial, "dumpsys media_session").await.ok();
    let accessibility = shell(
        adb,
        serial,
        &format!("settings --user {user} get secure enabled_accessibility_services"),
    )
    .await
    .ok();
    let safety_known = home.is_some()
        && foreground.is_some()
        && services
            .as_ref()
            .is_some_and(|s| s.contains("ACTIVITY MANAGER SERVICES"))
        && media.as_ref().is_some_and(|s| s.contains("MEDIA SESSION"))
        && accessibility.is_some();
    let mut eligible = BTreeMap::new();
    for (package, uid) in &packages {
        let reason = if !matches!(safety_for(state, package), Safety::Safe { .. }) {
            Some("Protected, caution, or unreviewed app".into())
        } else if !third_party.contains_key(package) {
            Some("System or disabled app".into())
        } else if packages.values().filter(|other| *other == uid).count() != 1 {
            Some("Shared app identity".into())
        } else if !safety_known {
            Some("Active app/service safety could not be verified".into())
        } else if home.as_ref() == Some(package) {
            Some("Home app".into())
        } else if media.as_ref().is_some_and(|s| s.contains(package))
            || accessibility.as_ref().is_some_and(|s| s.contains(package))
        {
            Some("Media or accessibility service".into())
        } else if services.as_ref().is_some_and(|s| {
            s.split("ServiceRecord{")
                .any(|block| block.contains(package) && block.contains("isForeground=true"))
        }) {
            Some("Foreground service (playback, VPN, or other ongoing work)".into())
        } else if foreground.as_ref() == Some(package) {
            Some("Foreground app".into())
        } else {
            None
        };
        eligible.insert(package.clone(), reason);
    }
    Ok(Context {
        hardware_id,
        user,
        packages,
        eligible,
    })
}

fn journal_dir(state: &AppState) -> PathBuf {
    state.data_dir.join("ram-undo")
}

fn journals(dir: &Path) -> Result<Vec<(PathBuf, Undo)>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let base = dir.canonicalize().map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if !path.starts_with(&base) {
            return Err("RAM undo path escapes its storage directory.".into());
        }
        let data = std::fs::read(&path).map_err(|e| e.to_string())?;
        let undo: Undo = serde_json::from_slice(&data).map_err(|_| {
            "RAM undo data is unreadable; changes are blocked to preserve recovery."
        })?;
        if undo.schema_version != 1 || !is_valid_package_name(&undo.package) {
            return Err(
                "Unsupported RAM undo data. Update the app before changing restrictions.".into(),
            );
        }
        result.push((path, undo));
    }
    Ok(result)
}

fn save_undo(dir: &Path, undo: &Undo) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = dir.join(format!("{stamp}.json"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(undo).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ram_report(
    state: State<'_, AppState>,
    serial: String,
    hardware_id: String,
) -> Result<RamReport, String> {
    report(state.inner(), &serial, &hardware_id).await
}

async fn report(state: &AppState, serial: &str, expected: &str) -> Result<RamReport, String> {
    let _lock = MUTATION.lock().await;
    let adb = state.adb_snapshot().await;
    let ctx = context(state, &*adb, serial, expected).await?;
    let mut notes = Vec::new();
    let proc = shell(&*adb, serial, "cat /proc/meminfo")
        .await
        .unwrap_or_default();
    let pressure = shell(&*adb, serial, "cat /proc/pressure/memory")
        .await
        .unwrap_or_default();
    let mem = shell(&*adb, serial, "dumpsys meminfo")
        .await
        .unwrap_or_default();
    let ps = shell(&*adb, serial, "ps -A -n -o UID,PID,NAME")
        .await
        .unwrap_or_default();
    let pss = ram::package_pss(&mem, &ps, &ctx.packages);
    let reading = ram::reading(&proc, &pressure);
    if reading.available_mb.is_none() {
        notes.push("Available RAM is unavailable on this TV.".into());
    }
    if reading.pressure_some_avg10.is_none() {
        notes.push("Memory pressure (PSI) is unavailable.".into());
    }
    if pss.is_empty() {
        notes.push("Verified per-app memory readings are unavailable.".into());
    }
    let undo = journals(&journal_dir(state))?;
    if shell(&*adb, serial, "getprop ro.serialno").await?.trim() != expected
        || shell(&*adb, serial, "am get-current-user").await?.trim() != ctx.user.to_string()
    {
        return Err("Device or Android user changed while reading RAM. Refresh to retry.".into());
    }
    let mut apps: Vec<_> = ctx
        .packages
        .keys()
        .map(|package| RamApp {
            package: package.clone(),
            name: state
                .app_lists
                .find(package)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| package.clone()),
            pss_mb: pss.get(package).copied(),
            blocked_reason: ctx.eligible.get(package).cloned().flatten(),
            can_undo: undo.iter().any(|(_, u)| {
                u.hardware_id == ctx.hardware_id && u.user == ctx.user && u.package == *package
            }),
        })
        .collect();
    apps.sort_by(|a, b| {
        b.pss_mb
            .unwrap_or(-1.0)
            .total_cmp(&a.pss_mb.unwrap_or(-1.0))
    });
    Ok(RamReport {
        hardware_id: ctx.hardware_id,
        user: ctx.user,
        sampled_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64,
        reading,
        apps,
        notes,
    })
}

#[tauri::command]
pub async fn ram_restrict(
    state: State<'_, AppState>,
    serial: String,
    hardware_id: String,
    user: u32,
    package: String,
    kept_packages: Vec<String>,
) -> Result<String, String> {
    restrict(
        state.inner(),
        &serial,
        &hardware_id,
        user,
        &package,
        &kept_packages,
    )
    .await
}

async fn restrict(
    state: &AppState,
    serial: &str,
    expected: &str,
    user: u32,
    package: &str,
    kept: &[String],
) -> Result<String, String> {
    let _lock = MUTATION.lock().await;
    if !is_valid_package_name(package) {
        return Err("Invalid package name.".into());
    }
    if kept.iter().any(|p| p == package) {
        return Err("This app is marked Keep.".into());
    }
    let adb = state.adb_snapshot().await;
    let ctx = context(state, &*adb, serial, expected).await?;
    if ctx.user != user {
        return Err("The Android user changed. Refresh the RAM report.".into());
    }
    let reason = ctx
        .eligible
        .get(package)
        .ok_or("This app is no longer installed.")?;
    if let Some(reason) = reason {
        return Err(reason.clone());
    }
    let dir = journal_dir(state);
    if journals(&dir)?
        .iter()
        .any(|(_, u)| u.hardware_id == expected && u.user == user && u.package == package)
    {
        return Err(
            "An undo record already exists. Undo it before applying another restriction.".into(),
        );
    }
    let previous = read_mode(&*adb, serial, user, package).await?;
    if matches!(previous, BackgroundMode::Ignore | BackgroundMode::Deny) {
        return Err("This app is already background restricted outside this workflow.".into());
    }
    save_undo(
        &dir,
        &Undo {
            schema_version: 1,
            hardware_id: expected.into(),
            user,
            package: package.into(),
            uid: ctx.packages[package],
            previous,
        },
    )?;
    write_mode(
        &*adb,
        serial,
        expected,
        user,
        package,
        BackgroundMode::Ignore,
    )
    .await?;
    Ok(
        "Restriction setting verified. Compare sessions to check RAM savings. Undo is saved."
            .into(),
    )
}

async fn read_mode(
    adb: &dyn AdbDriver,
    serial: &str,
    user: u32,
    package: &str,
) -> Result<BackgroundMode, String> {
    ram::background_mode(
        &shell(
            adb,
            serial,
            &format!(
                "cmd appops get --user {user} {} {OP}",
                quote_shell_arg(package)
            ),
        )
        .await?,
    )
}

async fn write_mode(
    adb: &dyn AdbDriver,
    serial: &str,
    expected: &str,
    user: u32,
    package: &str,
    mode: BackgroundMode,
) -> Result<(), String> {
    let command = guarded(
        expected,
        user,
        &format!(
            "cmd appops set --user {user} {} {OP} {}",
            quote_shell_arg(package),
            mode.shell_value()
        ),
    );
    shell(adb, serial, &command)
        .await
        .map_err(|e| format!("{e} Undo remains available; the setting may have changed."))?;
    if read_mode(adb, serial, user, package).await? != mode {
        return Err("Restriction readback did not match. Undo remains available.".into());
    }
    Ok(())
}

fn guarded(expected: &str, user: u32, command: &str) -> String {
    format!("test \"$(getprop ro.serialno)\" = {} && test \"$(am get-current-user)\" = {user} && {command}", quote_shell_arg(expected))
}

#[derive(Serialize)]
pub struct RamBenchmarkTrial {
    hardware_id: String,
    user: u32,
    package: String,
    cold: bool,
    firmware: String,
    app_version: String,
    timing: ram::LaunchTiming,
    reading: RamReading,
    app_pss_mb: Option<f64>,
}

#[tauri::command]
pub async fn ram_benchmark_trial(
    state: State<'_, AppState>,
    serial: String,
    hardware_id: String,
    user: u32,
    package: String,
    cold: bool,
) -> Result<RamBenchmarkTrial, String> {
    benchmark_trial(state.inner(), &serial, &hardware_id, user, &package, cold).await
}

async fn benchmark_trial(
    state: &AppState,
    serial: &str,
    expected: &str,
    user: u32,
    package: &str,
    cold: bool,
) -> Result<RamBenchmarkTrial, String> {
    let _lock = MUTATION.lock().await;
    if !is_valid_package_name(package) {
        return Err("Invalid package name.".into());
    }
    let adb = state.adb_snapshot().await;
    let ctx = context(state, &*adb, serial, expected).await?;
    if ctx.user != user {
        return Err("The Android user changed. Benchmark cancelled.".into());
    }
    if let Some(reason) = ctx.eligible.get(package).ok_or("App is not installed.")? {
        if reason != "Foreground app" {
            return Err(format!("Benchmark unavailable: {reason}"));
        }
    }
    let resolved = shell(&*adb, serial, &format!("cmd package resolve-activity --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.LEANBACK_LAUNCHER {}", quote_shell_arg(package))).await?;
    let component = resolved
        .lines()
        .find(|line| ram::component_package(line).as_deref() == Some(package))
        .ok_or("No TV launcher activity was reported for this app.")?
        .trim();
    // Accept only the component itself, not a surrounding resolver diagnostic.
    if component.split_whitespace().count() != 1 || !component.starts_with(&format!("{package}/")) {
        return Err("The app's launch activity could not be verified.".into());
    }
    let firmware = shell(&*adb, serial, "getprop ro.build.fingerprint")
        .await?
        .trim()
        .to_string();
    let details = shell(
        &*adb,
        serial,
        &format!("dumpsys package {}", quote_shell_arg(package)),
    )
    .await?;
    let app_version = details
        .lines()
        .find(|l| l.trim().starts_with("versionCode="))
        .ok_or("App version is unavailable; comparable trials cannot be recorded.")?
        .trim()
        .to_string();
    if firmware.is_empty() {
        return Err("Firmware identity is unavailable.".into());
    }
    let launch = format!(
        "am start -W --user {user} -n {}",
        quote_shell_arg(component)
    );
    if cold {
        shell(
            &*adb,
            serial,
            &guarded(
                expected,
                user,
                &format!("am force-stop --user {user} {}", quote_shell_arg(package)),
            ),
        )
        .await?;
    } else {
        shell(&*adb, serial, &guarded(expected, user, &launch)).await?;
    }
    shell(&*adb, serial, &guarded(expected, user, "input keyevent 3")).await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let timing =
        ram::launch_timing(&shell(&*adb, serial, &guarded(expected, user, &launch)).await?)?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let proc = shell(&*adb, serial, "cat /proc/meminfo")
        .await
        .unwrap_or_default();
    let pressure = shell(&*adb, serial, "cat /proc/pressure/memory")
        .await
        .unwrap_or_default();
    let mem = shell(&*adb, serial, "dumpsys meminfo")
        .await
        .unwrap_or_default();
    let ps = shell(&*adb, serial, "ps -A -n -o UID,PID,NAME")
        .await
        .unwrap_or_default();
    if shell(&*adb, serial, "getprop ro.serialno").await?.trim() != expected
        || shell(&*adb, serial, "am get-current-user").await?.trim() != user.to_string()
    {
        return Err("Device or Android user changed. Trial discarded.".into());
    }
    Ok(RamBenchmarkTrial {
        hardware_id: expected.into(),
        user,
        package: package.into(),
        cold,
        firmware,
        app_version,
        timing,
        reading: ram::reading(&proc, &pressure),
        app_pss_mb: ram::package_pss(&mem, &ps, &ctx.packages)
            .get(package)
            .copied(),
    })
}

#[tauri::command]
pub async fn ram_undo(
    state: State<'_, AppState>,
    serial: String,
    hardware_id: String,
    user: u32,
    package: String,
) -> Result<String, String> {
    undo(state.inner(), &serial, &hardware_id, user, &package).await
}

async fn undo(
    state: &AppState,
    serial: &str,
    expected: &str,
    user: u32,
    package: &str,
) -> Result<String, String> {
    let _lock = MUTATION.lock().await;
    let adb = state.adb_snapshot().await;
    let ctx = context(state, &*adb, serial, expected).await?;
    if ctx.user != user {
        return Err("The Android user changed. Refresh before undoing.".into());
    }
    let (path, saved) = journals(&journal_dir(state))?
        .into_iter()
        .find(|(_, u)| u.hardware_id == expected && u.user == user && u.package == package)
        .ok_or("No matching undo record exists for this TV and app.")?;
    if ctx.packages.get(package) != Some(&saved.uid)
        || ctx
            .packages
            .values()
            .filter(|uid| **uid == saved.uid)
            .count()
            != 1
    {
        return Err("App ownership changed. Undo record retained; no setting changed.".into());
    }
    let current = read_mode(&*adb, serial, user, package).await?;
    if current != saved.previous && current != BackgroundMode::Ignore {
        return Err("The setting changed outside this workflow. Undo record retained to avoid overwriting it.".into());
    }
    if current != saved.previous {
        write_mode(&*adb, serial, expected, user, package, saved.previous).await?;
    }
    std::fs::remove_file(path)
        .map_err(|e| format!("Setting restored, but undo record could not be removed: {e}"))?;
    Ok("Previous background setting restored and verified.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atv_optimizer_core::adb::sim::{self, SimulatedAdb, TransportState};
    use atv_optimizer_core::commands::loader::load_embedded_app_lists;
    use std::sync::Arc;
    const ID: &str = "RAM-TEST-TV";
    const APP: &str = "com.netflix.ninja";

    fn setup() -> (tempfile::TempDir, AppState, Arc<SimulatedAdb>) {
        let dir = tempfile::tempdir().unwrap();
        let adb = Arc::new(SimulatedAdb::empty());
        let mut device = sim::load_profile(&sim::fixtures_dir().join("shield-tv-pro")).unwrap();
        device.serial = ID.into();
        device.props.insert("ro.serialno".into(), ID.into());
        device
            .props
            .insert("ro.build.fingerprint".into(), "test-firmware".into());
        if let Some((_, package)) = device.packages.iter_mut().find(|(name, _)| name == APP) {
            package.system = false;
            package.enabled = true;
            package.installed = true;
        } else {
            device.packages.push((
                APP.into(),
                sim::Package {
                    system: false,
                    enabled: true,
                    installed: true,
                },
            ));
        }
        device.texts.insert("dumpsys activity activities".into(), "ACTIVITY MANAGER ACTIVITIES\nmResumedActivity: ActivityRecord{sim u0 com.google.android.tvlauncher/.MainActivity}".into());
        device.texts.insert(
            "dumpsys activity services".into(),
            "ACTIVITY MANAGER SERVICES\n".into(),
        );
        device.texts.insert(
            "dumpsys media_session".into(),
            "MEDIA SESSION SERVICE\n".into(),
        );
        device
            .appops
            .insert(format!("{APP}|{OP}"), "default".into());
        adb.world().add_device(device);
        adb.world().attach(ID, ID, TransportState::Device);
        let state = AppState::new(
            adb.clone(),
            load_embedded_app_lists().unwrap(),
            dir.path().to_path_buf(),
        );
        (dir, state, adb)
    }

    fn mode(adb: &SimulatedAdb) -> String {
        adb.world().devices[ID].appops[&format!("{APP}|{OP}")].clone()
    }

    #[tokio::test]
    async fn restrict_and_undo_restore_default_with_durable_record() {
        let (_dir, state, adb) = setup();
        restrict(&state, ID, ID, 0, APP, &[]).await.unwrap();
        assert_eq!(mode(&adb), "ignore");
        let saved = journals(&journal_dir(&state)).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].1.previous, BackgroundMode::Default);
        assert!(report(&state, ID, ID)
            .await
            .unwrap()
            .apps
            .iter()
            .any(|a| a.package == APP && a.can_undo));
        undo(&state, ID, ID, 0, APP).await.unwrap();
        assert_eq!(mode(&adb), "default");
        assert!(journals(&journal_dir(&state)).unwrap().is_empty());
    }

    #[tokio::test]
    async fn protected_kept_foreground_and_unknown_safety_never_write() {
        let (_dir, state, adb) = setup();
        assert!(restrict(&state, ID, ID, 0, "com.google.android.gms", &[])
            .await
            .is_err());
        assert!(restrict(&state, ID, ID, 0, APP, &[APP.into()])
            .await
            .is_err());
        adb.world().devices.get_mut(ID).unwrap().texts.insert(
            "dumpsys activity activities".into(),
            format!("mResumedActivity: ActivityRecord{{u0 {APP}/.Main}}"),
        );
        assert!(restrict(&state, ID, ID, 0, APP, &[])
            .await
            .unwrap_err()
            .contains("Foreground"));
        adb.world()
            .devices
            .get_mut(ID)
            .unwrap()
            .texts
            .insert("dumpsys activity activities".into(), "unreadable".into());
        assert!(restrict(&state, ID, ID, 0, APP, &[]).await.is_err());
        assert_eq!(mode(&adb), "default");
        assert!(journals(&journal_dir(&state)).unwrap().is_empty());
    }

    #[tokio::test]
    async fn media_and_foreground_services_are_protected_even_for_benchmarks() {
        let (_dir, state, adb) = setup();
        adb.world().devices.get_mut(ID).unwrap().texts.insert(
            "dumpsys activity services".into(),
            format!(
                "ACTIVITY MANAGER SERVICES\nServiceRecord{{u0 {APP}/.Service}}\nisForeground=true"
            ),
        );
        assert!(restrict(&state, ID, ID, 0, APP, &[]).await.is_err());
        assert!(benchmark_trial(&state, ID, ID, 0, APP, true).await.is_err());
        assert_eq!(mode(&adb), "default");
    }

    #[tokio::test]
    async fn device_and_user_changes_are_rejected_including_write_guard() {
        let (_dir, state, adb) = setup();
        assert!(restrict(&state, ID, "another-tv", 0, APP, &[])
            .await
            .is_err());
        assert!(restrict(&state, ID, ID, 10, APP, &[]).await.is_err());
        assert!(
            write_mode(&*adb, ID, "another-tv", 0, APP, BackgroundMode::Ignore)
                .await
                .is_err()
        );
        assert!(write_mode(&*adb, ID, ID, 10, APP, BackgroundMode::Ignore)
            .await
            .is_err());
        assert_eq!(mode(&adb), "default");
    }

    #[tokio::test]
    async fn ignored_write_retains_undo_and_external_change_is_not_overwritten() {
        let (_dir, state, adb) = setup();
        let fault = serde_json::from_value(serde_json::json!({"scope":"shell", "matches":"appops set", "effect":{"type":"ignore", "stdout":""}})).unwrap();
        adb.world().faults.push(fault);
        assert!(restrict(&state, ID, ID, 0, APP, &[]).await.is_err());
        assert_eq!(journals(&journal_dir(&state)).unwrap().len(), 1);
        adb.world().faults.clear();
        undo(&state, ID, ID, 0, APP).await.unwrap();
        restrict(&state, ID, ID, 0, APP, &[]).await.unwrap();
        adb.world()
            .devices
            .get_mut(ID)
            .unwrap()
            .appops
            .insert(format!("{APP}|{OP}"), "deny".into());
        assert!(undo(&state, ID, ID, 0, APP)
            .await
            .unwrap_err()
            .contains("outside"));
        assert_eq!(mode(&adb), "deny");
        assert_eq!(journals(&journal_dir(&state)).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn future_and_corrupt_undo_data_fail_closed() {
        let (_dir, state, adb) = setup();
        let dir = journal_dir(&state);
        save_undo(
            &dir,
            &Undo {
                schema_version: 99,
                hardware_id: ID.into(),
                user: 0,
                package: APP.into(),
                uid: 10001,
                previous: BackgroundMode::Allow,
            },
        )
        .unwrap();
        assert!(restrict(&state, ID, ID, 0, APP, &[]).await.is_err());
        assert_eq!(mode(&adb), "default");
        assert!(journals(&dir).unwrap_err().contains("Unsupported"));
    }

    #[tokio::test(start_paused = true)]
    async fn benchmark_reports_real_command_timing_and_memory_without_restricting() {
        let (_dir, state, adb) = setup();
        let trial = benchmark_trial(&state, ID, ID, 0, APP, false)
            .await
            .unwrap();
        assert_eq!(trial.timing.total_ms, 120);
        assert_eq!(trial.timing.launch_state.as_deref(), Some("WARM"));
        assert_eq!(mode(&adb), "default");
        assert!(adb.world().devices[ID]
            .input_log
            .iter()
            .any(|l| l == "keyevent 3"));
        assert!(journals(&journal_dir(&state)).unwrap().is_empty());
    }
}
