use regex::Regex;
use serde::{Deserialize, Serialize};
pub use shield_optimizer_core::adb::parse::*;
use std::sync::LazyLock;

/// Seconds since boot from `/proc/uptime` (first field). This bounds how much
/// usage history exists at all: `dumpsys usagestats` prints only the in-memory
/// buckets, and Android rebuilds those at boot — even the "yearly" bucket
/// starts at the last reboot. Without it, an app the user opens weekly reads
/// as "never used" on a freshly-rebooted device.
pub fn parse_uptime_secs(proc_uptime: &str) -> Option<u64> {
    proc_uptime
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(|s| s as u64)
}

/// One selectable display mode from `dumpsys display` supportedModes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayModeOption {
    /// e.g. "3840x2160".
    pub resolution: String,
    /// e.g. 59.94.
    pub refresh_hz: f64,
    /// True for the mode currently driving the panel.
    pub active: bool,
}

/// Enumerate the distinct display modes the panel advertises in
/// `dumpsys display` (`supportedModes [{id=…,width=…,height=…,fps=…}, …]`),
/// flagging the active one. Read-only: Android TV exposes no ADB way to *set*
/// the HDMI output mode, but listing them lets the UI show e.g. that 4K60 and
/// 4K59.94 both exist. Deduped by resolution+refresh; largest then fastest first.
pub fn parse_supported_display_modes(dumpsys_display: &str) -> Vec<DisplayModeOption> {
    static MODE_ENTRY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"id=(\d+),\s*width=(\d+),\s*height=(\d+),\s*fps=([\d.]+)").unwrap()
    });

    let active = parse_display_mode(dumpsys_display);
    let mut out: Vec<DisplayModeOption> = Vec::new();
    for caps in MODE_ENTRY.captures_iter(dumpsys_display) {
        let w: u32 = caps[2].parse().unwrap_or(0);
        let h: u32 = caps[3].parse().unwrap_or(0);
        let fps: f64 = caps[4].parse().unwrap_or(0.0);
        if w == 0 || h == 0 {
            continue;
        }
        let resolution = format!("{w}x{h}");
        let refresh_hz = (fps * 100.0).round() / 100.0;
        if out
            .iter()
            .any(|o| o.resolution == resolution && (o.refresh_hz - refresh_hz).abs() < 0.01)
        {
            continue;
        }
        let is_active = active.resolution.as_deref() == Some(resolution.as_str())
            && active
                .refresh_hz
                .is_some_and(|r| (r - refresh_hz).abs() < 0.01);
        out.push(DisplayModeOption {
            resolution,
            refresh_hz,
            active: is_active,
        });
    }
    out.sort_by(|a, b| {
        let area = |o: &DisplayModeOption| -> u64 {
            let mut it = o.resolution.split('x');
            let w: u64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let h: u64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            w * h
        };
        area(b).cmp(&area(a)).then(
            b.refresh_hz
                .partial_cmp(&a.refresh_hz)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    out
}

/// Best-effort list of surround encodings the connected HDMI sink reports in
/// `dumpsys audio`. Android exposes these as `AUDIO_FORMAT_<NAME>` tokens in the
/// device-capability section; the exact layout varies by firmware, so this
/// scans for known tokens and returns friendly labels (empty if none found).
/// Read-only — the sink's claimed capabilities, shown next to the surround
/// toggles.
pub fn parse_audio_supported_encodings(dumpsys_audio: &str) -> Vec<String> {
    static TOK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"AUDIO_FORMAT_([A-Z0-9_]+)").unwrap());
    let mut out: Vec<String> = Vec::new();
    for caps in TOK.captures_iter(dumpsys_audio) {
        let label = match &caps[1] {
            "AC3" => "Dolby Digital (AC-3)",
            "E_AC3" => "Dolby Digital Plus (E-AC-3)",
            "E_AC3_JOC" => "Dolby Atmos (E-AC-3 JOC)",
            "DTS" => "DTS",
            "DTS_HD" => "DTS-HD",
            "DOLBY_TRUEHD" => "Dolby TrueHD",
            _ => continue,
        };
        let s = label.to_string();
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// One package's on-disk footprint, from `dumpsys diskstats`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageStorage {
    pub package: String,
    /// The APK plus its extracted native libraries.
    pub app_bytes: u64,
    /// Private data directory — accounts, settings, whatever the app saved.
    pub data_bytes: u64,
    pub cache_bytes: u64,
    /// App plus data; Android already includes cache in the data figure.
    pub total_bytes: u64,
}

/// Device-wide storage split plus per-package footprints, from
/// `dumpsys diskstats`.
///
/// Everything is optional and `packages` can be empty. The platform fills
/// these from `DiskStatsLoggingService`, a daily job — a device that hasn't
/// run it prints the labels with empty arrays, and "no data yet" is the honest
/// answer there rather than a table of zeroes.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskStats {
    pub app_bytes: Option<u64>,
    pub app_data_bytes: Option<u64>,
    pub app_cache_bytes: Option<u64>,
    pub photos_bytes: Option<u64>,
    pub videos_bytes: Option<u64>,
    pub audio_bytes: Option<u64>,
    pub downloads_bytes: Option<u64>,
    pub system_bytes: Option<u64>,
    pub other_bytes: Option<u64>,
    /// From the `Data-Free:` line, which reports KB.
    pub data_free_bytes: Option<u64>,
    pub data_total_bytes: Option<u64>,
    /// Largest first.
    pub packages: Vec<PackageStorage>,
}

/// Split a `[a,b,c]` / `["a","b"]` diskstats array into its elements.
fn diskstats_array(value: &str) -> Vec<&str> {
    let Some(inner) = value
        .trim()
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
    else {
        return Vec::new();
    };
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|e| e.trim().trim_matches('"'))
        .collect()
}

/// Parse `dumpsys diskstats` — the device's own storage accounting, and the
/// only per-app breakdown reachable without root. The per-package data arrives
/// as four parallel arrays (names, app, data, cache) that we zip back into
/// rows; a device that reports them at different lengths gets truncated to the
/// shortest rather than mismatched.
pub fn parse_diskstats(dumpsys: &str) -> DiskStats {
    static FREE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\d+)K\s*/\s*(\d+)K\s+total").unwrap());

    let mut stats = DiskStats::default();
    let (mut names, mut apps, mut datas, mut caches) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());

    for line in dumpsys.lines() {
        let Some((label, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match label.trim() {
            "App Size" => stats.app_bytes = value.parse().ok(),
            "App Data Size" => stats.app_data_bytes = value.parse().ok(),
            "App Cache Size" => stats.app_cache_bytes = value.parse().ok(),
            "Photos Size" => stats.photos_bytes = value.parse().ok(),
            "Videos Size" => stats.videos_bytes = value.parse().ok(),
            "Audio Size" => stats.audio_bytes = value.parse().ok(),
            "Downloads Size" => stats.downloads_bytes = value.parse().ok(),
            "System Size" => stats.system_bytes = value.parse().ok(),
            "Other Size" => stats.other_bytes = value.parse().ok(),
            "Data-Free" => {
                if let Some(caps) = FREE.captures(value) {
                    stats.data_free_bytes = caps[1]
                        .parse::<u64>()
                        .ok()
                        .map(|kb| kb.saturating_mul(1024));
                    stats.data_total_bytes = caps[2]
                        .parse::<u64>()
                        .ok()
                        .map(|kb| kb.saturating_mul(1024));
                }
            }
            "Package Names" => names = diskstats_array(value),
            "App Sizes" => apps = diskstats_array(value),
            "App Data Sizes" => datas = diskstats_array(value),
            "Cache Sizes" => caches = diskstats_array(value),
            _ => {}
        }
    }

    let rows = names
        .len()
        .min(apps.len())
        .min(datas.len())
        .min(caches.len());
    let mut packages: Vec<PackageStorage> = (0..rows)
        .filter(|&i| !names[i].is_empty())
        .map(|i| {
            let app_bytes = apps[i].parse().unwrap_or(0);
            let data_bytes = datas[i].parse().unwrap_or(0);
            let cache_bytes = caches[i].parse().unwrap_or(0);
            PackageStorage {
                package: names[i].to_string(),
                app_bytes,
                data_bytes,
                cache_bytes,
                total_bytes: app_bytes.saturating_add(data_bytes),
            }
        })
        .collect();
    packages.sort_by(|a, b| {
        b.total_bytes
            .cmp(&a.total_bytes)
            .then_with(|| a.package.cmp(&b.package))
    });
    stats.packages = packages;
    stats
}

/// One line of a `du -k` listing — a folder rollup or a single file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskUsageEntry {
    pub path: String,
    pub bytes: u64,
    pub is_dir: bool,
}

/// Parse `du -k` output (`<kilobytes>\t<path>` per line) into byte-sized
/// entries, largest first. Tab-delimited on purpose: media filenames routinely
/// contain spaces, so splitting on whitespace would truncate them. `is_dir` is
/// stamped by the caller, which knows which `du`/`find` produced the lines.
pub fn parse_du_output(du: &str, is_dir: bool) -> Vec<DiskUsageEntry> {
    let mut out: Vec<DiskUsageEntry> = du
        .lines()
        .filter_map(|line| {
            let (kb, path) = line.split_once('\t')?;
            let kb: u64 = kb.trim().parse().ok()?;
            // adb hands back CRLF on Windows; the trailing slash is `du`'s own
            // rendering of the directory we asked about.
            let path = path.trim_end().trim_end_matches('/');
            if path.is_empty() {
                return None;
            }
            Some(DiskUsageEntry {
                path: path.to_string(),
                bytes: kb.saturating_mul(1024),
                is_dir,
            })
        })
        .collect();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_uptime_first_field_only() {
        // "<uptime> <idle>", both fractional seconds.
        assert_eq!(parse_uptime_secs("66526.81 234153.95"), Some(66526));
        assert_eq!(parse_uptime_secs("66526.81 234153.95\n"), Some(66526));
        assert_eq!(parse_uptime_secs(""), None);
        assert_eq!(parse_uptime_secs("not-a-number 1.0"), None);
    }

    #[test]
    fn parses_diskstats_categories_and_packages() {
        // Real `dumpsys diskstats` shape (NVIDIA Shield, Android 11) — the
        // per-package data is four parallel arrays.
        let input = "\
Latency: 4ms [512B Data Write]\n\
Recent Disk Write Speed (kB/s) = 19777\n\
Data-Free: 1508100K / 12203000K total = 12% free\n\
Cache-Free: 1508100K / 12203000K total = 12% free\n\
System-Free: 162000K / 2031548K total = 7% free\n\
App Size: 4484263936\n\
App Data Size: 3701301248\n\
App Cache Size: 2308022272\n\
Photos Size: 11210752\n\
Videos Size: 1847173120\n\
Audio Size: 0\n\
Downloads Size: 0\n\
System Size: 16000000000\n\
Other Size: 246988800\n\
Package Names: [\"com.plexapp.android\",\"com.android.vending\",\"com.nvidia.tegrazone3\"]\n\
App Sizes: [152604672,71389184,4096]\n\
App Data Sizes: [507904,16707584,32768]\n\
Cache Sizes: [90112,4444160,16384]\n";

        let stats = parse_diskstats(input);
        assert_eq!(stats.app_bytes, Some(4_484_263_936));
        assert_eq!(stats.app_data_bytes, Some(3_701_301_248));
        assert_eq!(stats.app_cache_bytes, Some(2_308_022_272));
        assert_eq!(stats.photos_bytes, Some(11_210_752));
        assert_eq!(stats.videos_bytes, Some(1_847_173_120));
        assert_eq!(stats.audio_bytes, Some(0));
        assert_eq!(stats.system_bytes, Some(16_000_000_000));
        assert_eq!(stats.other_bytes, Some(246_988_800));
        assert_eq!(stats.data_free_bytes, Some(1_508_100 * 1024));
        assert_eq!(stats.data_total_bytes, Some(12_203_000 * 1024));

        // Sorted by total, largest first.
        assert_eq!(stats.packages.len(), 3);
        assert_eq!(stats.packages[0].package, "com.plexapp.android");
        assert_eq!(stats.packages[0].app_bytes, 152_604_672);
        assert_eq!(stats.packages[0].data_bytes, 507_904);
        assert_eq!(stats.packages[0].cache_bytes, 90_112);
        assert_eq!(stats.packages[0].total_bytes, 152_604_672 + 507_904);
        assert_eq!(stats.packages[1].package, "com.android.vending");
        assert_eq!(stats.packages[2].package, "com.nvidia.tegrazone3");
    }

    #[test]
    fn diskstats_survives_a_device_that_has_no_per_app_data_yet() {
        // DiskStatsLoggingService runs on a daily job; before its first run the
        // labels are present but the arrays are empty. Categories still parse.
        let input = "\
App Size: 4484263936\n\
Package Names: []\n\
App Sizes: []\n\
App Data Sizes: []\n\
Cache Sizes: []\n";
        let stats = parse_diskstats(input);
        assert_eq!(stats.app_bytes, Some(4_484_263_936));
        assert!(stats.packages.is_empty());
    }

    #[test]
    fn diskstats_truncates_mismatched_arrays_instead_of_misaligning() {
        let input = "\
Package Names: [\"a\",\"b\",\"c\"]\n\
App Sizes: [10,20]\n\
App Data Sizes: [1,2,3]\n\
Cache Sizes: [0,0,0]\n";
        let stats = parse_diskstats(input);
        assert_eq!(stats.packages.len(), 2);
        assert_eq!(stats.packages[0].package, "b");
        assert_eq!(stats.packages[0].total_bytes, 22);
    }

    #[test]
    fn diskstats_on_empty_input_is_all_none() {
        assert_eq!(parse_diskstats(""), DiskStats::default());
    }

    #[test]
    fn parses_du_output_with_crlf_and_spaced_filenames() {
        // adb hands back CRLF on Windows, and media filenames have spaces.
        let input = "579520\t/sdcard/Wallpapers/Lofi hip hop mix [2018].mp4\r\n\
            113232\t/sdcard/GAYERFy_TV_v0.1.22-debug.apk\r\n\
            2056024\t/sdcard/\r\n";
        let entries = parse_du_output(input, false);
        assert_eq!(entries.len(), 3);
        // Largest first.
        assert_eq!(entries[0].path, "/sdcard");
        assert_eq!(entries[0].bytes, 2_056_024 * 1024);
        assert_eq!(
            entries[1].path,
            "/sdcard/Wallpapers/Lofi hip hop mix [2018].mp4"
        );
        assert_eq!(entries[1].bytes, 579_520 * 1024);
        assert!(!entries[1].is_dir);
    }

    #[test]
    fn du_output_skips_noise_lines() {
        let input = "du: /sdcard/x: Permission denied\n\nnot a du line\n12\t/sdcard/Music\n";
        let entries = parse_du_output(input, true);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/sdcard/Music");
        assert!(entries[0].is_dir);
    }

    #[test]
    fn parses_supported_modes_and_flags_active() {
        let input = "modeId 20, supportedModes [{id=1, width=3840, height=2160, fps=29.97003}, {id=20, width=3840, height=2160, fps=59.94006}, {id=30, width=1920, height=1080, fps=60.0}]";
        let modes = parse_supported_display_modes(input);
        assert_eq!(modes.len(), 3, "deduped, one per resolution+refresh");
        // 4K sorts ahead of 1080p; within 4K, 59.94 ahead of 29.97.
        assert_eq!(modes[0].resolution, "3840x2160");
        assert_eq!(modes[0].refresh_hz, 59.94);
        assert!(modes[0].active, "active mode id 20 is the 4K59.94 entry");
        assert!(modes
            .iter()
            .any(|m| m.resolution == "1920x1080" && !m.active));
    }

    #[test]
    fn supported_encodings_from_dumpsys_audio_tokens() {
        let input =
            "Sink formats: AUDIO_FORMAT_AC3, AUDIO_FORMAT_E_AC3, AUDIO_FORMAT_AC3, AUDIO_FORMAT_PCM_16_BIT";
        let enc = parse_audio_supported_encodings(input);
        assert_eq!(
            enc,
            vec!["Dolby Digital (AC-3)", "Dolby Digital Plus (E-AC-3)"]
        );
    }

    #[test]
    fn supported_encodings_empty_when_absent() {
        assert!(parse_audio_supported_encodings("no formats here").is_empty());
    }
}
