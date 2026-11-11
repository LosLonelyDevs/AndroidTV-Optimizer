use std::collections::BTreeMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::is_valid_package_name;

#[derive(Debug, Clone, Serialize, Default)]
pub struct RamReading {
    pub total_mb: Option<f64>,
    pub available_mb: Option<f64>,
    pub free_mb: Option<f64>,
    pub file_cache_mb: Option<f64>,
    pub swap_used_mb: Option<f64>,
    pub pressure_some_avg10: Option<f64>,
}

pub fn reading(meminfo: &str, pressure: &str) -> RamReading {
    let values: BTreeMap<_, _> = meminfo
        .lines()
        .filter_map(|line| {
            let (key, rest) = line.split_once(':')?;
            let mut parts = rest.split_whitespace();
            let value = parts.next()?.parse::<f64>().ok()?;
            (parts.next() == Some("kB") && value.is_finite() && value >= 0.0)
                .then_some((key, value / 1024.0))
        })
        .collect();
    let get = |key| values.get(key).copied();
    let psi = pressure.lines().find(|line| line.starts_with("some "));
    RamReading {
        total_mb: get("MemTotal"),
        available_mb: get("MemAvailable"),
        free_mb: get("MemFree"),
        file_cache_mb: get("Cached"),
        swap_used_mb: get("SwapTotal")
            .zip(get("SwapFree"))
            .filter(|(total, free)| total >= free)
            .map(|(total, free)| total - free),
        pressure_some_avg10: psi.and_then(|line| {
            line.split_whitespace()
                .find_map(|word| word.strip_prefix("avg10="))
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        }),
    }
}

pub fn package_uids(text: &str) -> BTreeMap<String, u32> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.strip_prefix("package:")?.split_whitespace();
            let package = parts.next()?;
            let uid = parts.find_map(|p| p.strip_prefix("uid:"))?.parse().ok()?;
            is_valid_package_name(package).then(|| (package.to_string(), uid))
        })
        .collect()
}

pub fn component_package(text: &str) -> Option<String> {
    let re = Regex::new(r"([A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)+)/[A-Za-z0-9_.$]+").unwrap();
    re.captures(text).map(|c| c[1].to_string())
}

pub fn foreground_package(activities: &str) -> Option<String> {
    activities
        .lines()
        .filter(|line| line.contains("ResumedActivity") || line.contains("mResumed"))
        .find_map(component_package)
}

/// Attribute PSS only when both PID and numeric UID identify a unique package.
pub fn package_pss(
    meminfo: &str,
    ps: &str,
    packages: &BTreeMap<String, u32>,
) -> BTreeMap<String, f64> {
    let processes: BTreeMap<u32, (u32, &str)> = ps
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let uid = words.next()?.parse().ok()?;
            let pid = words.next()?.parse().ok()?;
            Some((pid, (uid, words.next()?)))
        })
        .collect();
    let row = Regex::new(r"^\s*([\d,]+)K:\s+(\S+)\s+\(pid\s+(\d+)").unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut result = BTreeMap::new();
    for line in meminfo.lines() {
        let Some(c) = row.captures(line) else {
            continue;
        };
        let Ok(pid) = c[3].parse::<u32>() else {
            continue;
        };
        let Some((uid, name)) = processes.get(&pid) else {
            continue;
        };
        if *name != &c[2] || !seen.insert(pid) {
            continue;
        }
        let package = name.split(':').next().unwrap_or(name);
        if packages.get(package) != Some(uid)
            || packages.values().filter(|other| *other == uid).count() != 1
        {
            continue;
        }
        if let Ok(kb) = c[1].replace(',', "").parse::<f64>() {
            *result.entry(package.to_string()).or_default() += kb / 1024.0;
        }
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundMode {
    Allow,
    Ignore,
    Deny,
    Default,
    Foreground,
}

impl BackgroundMode {
    pub fn shell_value(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Ignore => "ignore",
            Self::Deny => "deny",
            Self::Default => "default",
            Self::Foreground => "foreground",
        }
    }
}

pub fn background_mode(text: &str) -> Result<BackgroundMode, String> {
    if text.trim() == "No operations." {
        return Ok(BackgroundMode::Default);
    }
    // UID overrides affect more than this one package and cannot be safely undone here.
    if text.lines().any(|line| line.contains("Uid mode:")) {
        return Err("A UID-wide override exists; per-app changes are unavailable.".into());
    }
    let line = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("RUN_ANY_IN_BACKGROUND:"))
        .ok_or("Background restriction state was not reported.")?;
    let value = line.trim().split([';', ' ', '\n']).next().unwrap_or("");
    match value {
        "allow" => Ok(BackgroundMode::Allow),
        "ignore" => Ok(BackgroundMode::Ignore),
        "deny" => Ok(BackgroundMode::Deny),
        "default" => Ok(BackgroundMode::Default),
        "foreground" => Ok(BackgroundMode::Foreground),
        _ => Err("Unrecognized background restriction state.".into()),
    }
}

#[derive(Debug, Serialize)]
pub struct LaunchTiming {
    pub total_ms: u64,
    pub wait_ms: Option<u64>,
    pub launch_state: Option<String>,
}

pub fn launch_timing(text: &str) -> Result<LaunchTiming, String> {
    let fields: BTreeMap<_, _> = text
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(k, v)| (k.trim(), v.trim()))
        .collect();
    if fields.get("Status") != Some(&"ok") || text.contains("Warning:") || text.contains("Error:") {
        return Err("Android did not report a fresh timed launch. Trial discarded.".into());
    }
    let total_ms = fields
        .get("TotalTime")
        .and_then(|n| n.parse().ok())
        .ok_or("App launch timing is unavailable. Trial discarded.")?;
    Ok(LaunchTiming {
        total_ms,
        wait_ms: fields.get("WaitTime").and_then(|n| n.parse().ok()),
        launch_state: fields.get("LaunchState").map(|s| s.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_is_not_zero_and_available_is_not_free() {
        let r = reading("MemTotal: 3072000 kB\nMemFree: 102400 kB\nMemAvailable: 614400 kB\nSwapTotal: 204800 kB\nSwapFree: 51200 kB", "some avg10=2.50 avg60=1.0");
        assert_eq!(r.available_mb, Some(600.0));
        assert_eq!(r.free_mb, Some(100.0));
        assert_eq!(r.swap_used_mb, Some(150.0));
        assert_eq!(r.pressure_some_avg10, Some(2.5));
        assert_eq!(reading("Permission denied", "").available_mb, None);
        assert_eq!(reading("", "some avg10=NaN").pressure_some_avg10, None);
    }

    #[test]
    fn memory_requires_matching_pid_uid_and_unique_ownership() {
        let packages =
            package_uids("package:org.example.app uid:10123\npackage:org.example.other uid:10124");
        let mem = "12,288K: org.example.app (pid 5)\n2,048K: org.example.app:worker (pid 6)\n999K: org.example.other (pid 7)\n12,288K: org.example.app (pid 5)";
        let ps = "UID PID NAME\n10123 5 org.example.app\n10123 6 org.example.app:worker\n10123 7 org.example.other";
        assert_eq!(
            package_pss(mem, ps, &packages).get("org.example.app"),
            Some(&14.0)
        );
        assert!(!package_pss(mem, ps, &packages).contains_key("org.example.other"));
        let mut shared = packages;
        shared.insert("org.example.shared".into(), 10123);
        assert!(package_pss(mem, ps, &shared).is_empty());
    }

    #[test]
    fn modes_preserve_defaults_and_reject_ambiguous_or_unsupported_reads() {
        assert_eq!(
            background_mode("No operations."),
            Ok(BackgroundMode::Default)
        );
        assert_eq!(
            background_mode("RUN_ANY_IN_BACKGROUND: ignore; time=+2s"),
            Ok(BackgroundMode::Ignore)
        );
        for text in [
            "",
            "Unknown operation",
            "Uid mode: RUN_ANY_IN_BACKGROUND: ignore\nRUN_ANY_IN_BACKGROUND: allow",
        ] {
            assert!(background_mode(text).is_err());
        }
    }

    #[test]
    fn launch_timing_rejects_failed_or_already_foreground_launches() {
        let result =
            launch_timing("Status: ok\nLaunchState: COLD\nTotalTime: 340\nWaitTime: 350").unwrap();
        assert_eq!(result.total_ms, 340);
        assert_eq!(result.launch_state.as_deref(), Some("COLD"));
        assert!(launch_timing("Status: timeout\nTotalTime: 340").is_err());
        assert!(launch_timing("Warning: Activity not started\nStatus: ok\nTotalTime: 0").is_err());
        assert!(launch_timing("Status: ok\nWaitTime: 20").is_err());
    }
}
