// TypeScript counterparts of the Rust types in crates/core/src/engine/types.rs
// and src-tauri/src/commands/*.rs. Keep in sync.
import type { SurroundMode } from "../../shared/media";

export type ConnectionType = "network" | "usb";
export type DeviceStatus = "device" | "unauthorized" | "offline";
export type DeviceType = "shield" | "google_tv" | "unknown";
/// What the device said about being a TV. Distinct from DeviceType, whose
/// "unknown" only means "no catalog match" — see engine/detection.rs.
export type TvEvidence = "tv" | "not_tv" | "unknown";
export type ActionMethod = "disable" | "uninstall";
export type RiskTier = "safe" | "medium" | "high" | "advanced";
export type OtherPackage = Omit<InstalledPackage, "catalog">;

export interface DeviceProperties {
  friendly_name: string | null;
  brand: string;
  model: string;
  device_codename: string;
  manufacturer: string;
  android_release: string;
  sdk_level: string;
  build_id: string;
  board_platform: string;
  characteristics?: string;
  serial_number?: string;
  /// `pm has-feature android.software.leanback`. null when the device gave no
  /// readable answer — which is not the same as "no".
  leanback?: boolean | null;
}

export interface Device {
  id: number;
  serial: string;
  name: string;
  model: string;
  device_type: DeviceType;
  tv_evidence: TvEvidence;
  status: DeviceStatus;
  connection: ConnectionType;
  properties: DeviceProperties | null;
}

export interface AppEntry {
  package: string;
  name: string;
  method: ActionMethod;
  risk: RiskTier;
  optimize_description: string;
  restore_description: string;
  default_optimize: boolean;
  default_restore: boolean;
  /// Whether this package has a real Google Play listing at this exact id
  /// (audited). Controls whether the "Play Store" button shows.
  play_store: boolean;
  /// Discontinued service — safe to uninstall despite no Play Store listing.
  defunct?: boolean;
  /// "Remove if unused" tier — surfaced as a candidate with a usage signal.
  review?: boolean;
  /// When a person last reviewed this classification, `YYYY-MM-DD`.
  reviewed_at?: string;
  /// Short evidence notes or URLs behind the classification.
  sources?: string[];
  /// Device families the entry applies to; absent means the list decides.
  device_scope?: string[];
}

/// When an app was last opened (from dumpsys usagestats).
export interface AppUsage {
  /// "YYYY-MM-DD HH:MM:SS" of last use, or null if not opened in the window.
  last_used: string | null;
  launch_count: number;
}

/// Installed storage for one package, in bytes. Disk, never memory. A field
/// the device did not report is null, which renders as unavailable — never 0.
/// `data_bytes` includes the cache, so the two are never added together.
export interface AppStorage {
  app_bytes: number | null;
  data_bytes: number | null;
  cache_bytes: number | null;
}

/// Usage plus the window it was observed over. `window_secs` is device uptime:
/// `dumpsys usagestats` only reports Android's in-memory buckets, which are
/// rebuilt at boot, so nothing older than the last reboot exists to report.
export interface UsageReport {
  window_secs: number | null;
  entries: Record<string, AppUsage>;
}

export interface LauncherEntry {
  name: string;
  package: string;
  /// The launcher's official source page, available even when installed.
  /// Null for stock launchers and HOME handlers outside the catalog.
  source_url: string | null;
  /// Setup helpers (Setup Wraith) a takeover turns off together with this
  /// stock launcher. Present only on a stock entry that has one.
  disable_with?: string[];
}

export interface LauncherStatus {
  entry: LauncherEntry;
  installed: boolean;
  enabled: boolean;
  /// Preinstalled launcher — shown so users can switch back to stock.
  stock: boolean;
  /// HOME-capable app outside both catalogs (e.g. Setup Wraith, a sideloaded
  /// HOME app).
  other: boolean;
  /// Google TV's Setup Wraith: declares HOME but is the setup wizard. Never
  /// offered as the default.
  setup_helper: boolean;
}

export interface CurrentLauncher {
  package: string | null;
  activity: string | null;
  /// Set when Android's resolver named another app than the HOME role holder.
  note?: string | null;
}

export interface ConnectResult {
  ok: boolean;
  message: string;
}

export interface ForgetResult {
  ok: boolean;
  /// Every adb transport key dropped for this device, the requested one first.
  disconnected: string[];
  /// Still advertising Wireless debugging, so adb will re-attach it by itself.
  still_advertised: boolean;
  message: string;
}

export interface PairResult {
  ok: boolean;
  message: string;
  /// mDNS instance of the paired device's pairing service; null when adb
  /// never saw it, and then nothing advertised can be tied to this device.
  instance: string | null;
}

export interface PairedConnectResult {
  ok: boolean;
  /// It answered but its ro.serialno isn't the paired device's; it was
  /// disconnected again.
  not_the_paired_device: boolean;
  message: string;
}

/// One read of `adb mdns services` for a just-paired device's connect port.
export type PairedConnectProbe =
  | { state: "waiting" }
  | { state: "unidentified" }
  | { state: "attached"; serial: string }
  | { state: "endpoint"; address: string }
  | { state: "ambiguous"; addresses: string[] }
  | { state: "not_the_paired_device"; message: string };

export interface DisplayMode {
  resolution: string | null;
  refresh_hz: number | null;
  hdr_types: string[];
}

export type {
  SurroundMode, VerdictLevel, VideoFormat, DisplayModeEntry,
  AudioPassthrough, Verdict, MediaCapabilities,
} from "../../shared/media";

/// CPU + network rates over one device-side sampling window.
export interface ResourceSample {
  cpu_percent: number | null;
  interfaces: { name: string; rx_bytes_per_s: number | null; tx_bytes_per_s: number | null }[];
  interval_ms: number | null;
}

export interface DeviceLogOptions {
  priority: "verbose" | "debug" | "info" | "warning" | "error" | "fatal";
  lines: number;
  tag: string | null;
  package: string | null;
}

export interface DeviceLogResult {
  output: {
    stdout: string;
    stderr: string;
    exit_code: number | null;
    termination: "completed" | "output_limit" | "timeout";
  };
  package: string | null;
  pid: number | null;
}

export interface ShellRunResult {
  stdout: string;
  stderr: string;
  exit_code: number | null;
  termination: "completed" | "output_limit" | "timeout";
  /// The safety gate refused it; nothing was sent to the device.
  blocked: boolean;
  blocked_reason: string | null;
}

export interface MemoryEntry {
  /// The full process name the device reported (`com.foo:remote`,
  /// `/system/bin/surfaceflinger`, `foo@2.1-service`).
  process: string;
  pid: number | null;
  /// The package this process name would belong to, or null when it has no
  /// package shape. Unverified until matched against the installed list.
  package: string | null;
  mb: number;
}

export interface RamInfo {
  total_mb: number | null;
  used_mb: number | null;
  free_mb: number | null;
  swap_mb: number | null;
}

export interface StorageInfo {
  total: string | null;
  used: string | null;
  available: string | null;
  used_percent: number | null;
}

export interface PackageStorage {
  package: string;
  app_bytes: number;
  data_bytes: number;
  cache_bytes: number;
  total_bytes: number;
}

export interface DiskStats {
  app_bytes: number | null;
  app_data_bytes: number | null;
  app_cache_bytes: number | null;
  photos_bytes: number | null;
  videos_bytes: number | null;
  audio_bytes: number | null;
  downloads_bytes: number | null;
  system_bytes: number | null;
  other_bytes: number | null;
  data_free_bytes: number | null;
  data_total_bytes: number | null;
  packages: PackageStorage[];
}

export interface DiskUsageEntry {
  path: string;
  bytes: number;
  is_dir: boolean;
}

export interface StorageUsage {
  path: string;
  children: DiskUsageEntry[];
  largest_files: DiskUsageEntry[];
  total_bytes: number;
}

export interface HealthReport {
  display: DisplayMode;
  ram: RamInfo;
  storage: StorageInfo;
  temperature_c: number | null;
  audio_device: string | null;
  top_memory: MemoryEntry[];
  disk: DiskStats;
}

export interface DeviceReport {
  serial: string;
  name: string;
  report: HealthReport | null;
  error: string | null;
}

export interface RestartResult {
  ok: boolean;
  message: string;
}

/// "Stop ADB when quitting" as the quit path will read it. `generation` goes
/// up with every applied change; a change sent from an older one may be
/// refused, and the reply is always the setting as it now stands.
export interface StopAdbOnQuit {
  enabled: boolean;
  generation: number;
}

export interface SnapshotFile {
  path: string;
  filename: string;
  saved_at: string;
  label: string | null;
  device_name: string;
  device_serial: string;
  device_type: DeviceType;
  disabled_count: number;
  settings_count: number;
  launcher: string | null;
}

export interface ActionResult {
  ok: boolean;
  message: string;
}

export interface InstalledPackage {
  package: string;
  system: boolean;
  enabled: boolean;
  /// Friendly name for recognized sideloads (Artemis, Overseerr, …); null otherwise.
  name?: string | null;
  /// One line on what the app is, from known-names.json. Display only: the
  /// verdict for these packages stays Unknown.
  description?: string | null;
  catalog: boolean;
}

/// `set_home_any` — the Advanced picker. It never disables anything.
export interface SetHomeAnyResult {
  ok: boolean;
  current_launcher: string | null;
  /// null when the device can't answer (query-activities is Android 9+).
  declares_home: boolean | null;
  /// Stock still holds Home; only the separate "Disable stock launcher" step hands it over.
  stock_holds_home: boolean;
  message: string;
  diagnostics: string[];
}

export interface SetLauncherResult {
  ok: boolean;
  strategy: string | null;
  current_launcher: string | null;
  last_error: string | null;
  /// Polite strategies failed, but disabling the active stock launcher would
  /// work — the UI confirms with the user and retries with allowStockDisable.
  stock_takeover_available: boolean;
  /// Every command the attempt issued and what the device replied, in order.
  /// Offered as copyable detail on failure — launcher behavior varies enough
  /// between builds that a report is only actionable with the per-stage record.
  diagnostics: string[];
}

export interface InstallApkResult {
  ok: boolean;
  path: string;
  message: string;
  hint: string | null;
  /// Package id from the APK's manifest — only read when `downgrade_blocked`,
  /// which is the one case the UI needs it and may not already have it.
  package: string | null;
  /// The install used `adb install -d` (allow downgrade).
  downgraded: boolean;
  /// Android refused the downgrade even with `-d`. The only route left is
  /// uninstall-then-install, which erases the app's data.
  downgrade_blocked: boolean;
}

export interface DiscoveredApk {
  path: string;
  name: string;
  size_bytes: number;
  package: string | null;
  /// `versionCode` from the APK manifest — the integer used to compare against
  /// the installed copy (Upgrade / Downgrade / Reinstall).
  version_code: number | null;
  /// `versionName` from the APK manifest — the human version string, display-only.
  version_name: string | null;
}

/// The version of a package currently installed on the device.
export interface InstalledVersion {
  version_code: number | null;
  version_name: string | null;
}

/// What an APK claims about itself, read before anything is installed.
/// `abi_compatible: null` means we could not establish it — never render that
/// as a mismatch.
export interface ApkInspection {
  path: string;
  name: string;
  size_bytes: number;
  package: string | null;
  abis: string[];
  device_abis: string[];
  abi_compatible: boolean | null;
  /** null when the TV could not be asked — never read that as "no". */
  already_installed: boolean | null;
}

export interface BackupApkResult {
  ok: boolean;
  files: string[];
  /// More than one APK — a split APK that must be installed together.
  split: boolean;
  message: string;
}

export interface CloneAppResult {
  ok: boolean;
  message: string;
  hint: string | null;
}

export interface ScanResult {
  subnet: string | null;
  found: string[];
  connected: string[];
  unauthorized: string[];
  failed: string[];
  /// Pairing `host:port` for devices advertising only an Android 11+ pairing
  /// service. They need the 6-digit code from the TV before they can connect.
  needs_pairing: string[];
  message: string;
}

export interface FindResult {
  hits: string[];
  /// Directories whose search could not be run because the ADB call failed —
  /// distinct from a directory that simply holds no matches.
  unsearched: string[];
}

export interface ScreenshotResult {
  path: string;
  base64: string;
}

export interface SendTextResult {
  ok: boolean;
  message: string;
  /// "channel" = scrcpy control socket (instant), "shell" = legacy `input`
  /// fallback (~700 ms/press), "none" = nothing was sent.
  transport: "channel" | "shell" | "none";
}

export interface FileEntry {
  name: string;
  is_dir: boolean;
  is_symlink: boolean;
  size_bytes: number;
  modified: string;
}

export interface FileTransferResult {
  ok: boolean;
  message: string;
  local_path: string | null;
}

export interface AdbStatus {
  available: boolean;
  path: string | null;
  last_probe: string | null;
}

export interface UpdateInfo {
  current: string;
  latest: string | null;
  update_available: boolean;
  url: string;
  /// Notes for the version currently running — shown once after an update
  /// lands, including one installed by Homebrew or by hand.
  current_notes: string | null;
}

export interface InstallResult {
  ok: boolean;
  path: string | null;
  message: string;
}

export interface PrivateDnsState {
  mode: string | null;
  hostname: string | null;
}

export interface PrivateDnsResult {
  ok: boolean;
  message: string;
  reverted: boolean;
}

export interface SnapshotApplyPlan {
  packages_to_disable: string[];
  packages_already_disabled: string[];
  packages_not_installed: string[];
  launcher_to_set: string | null;
  settings_to_write: Record<string, string>;
  settings_to_delete: string[];
  settings_already_set: string[];
  /// Device values for every setting the snapshot mentions; a missing key is unset.
  current_values: Record<string, string>;
  /// Home app when the plan was computed; null means the device couldn't say.
  current_launcher: string | null;
  /// The snapshot's launcher when it isn't installed here, so it's skipped.
  launcher_not_installed: string | null;
  cross_device_warning: string | null;
}

export interface ApplyResult {
  packages_disabled: string[];
  packages_failed: string[];
  launcher_set: boolean;
  launcher_message: string | null;
  settings_written: string[];
  settings_deleted: string[];
  settings_failed: string[];
  summary: string;
}

export type { Safety } from "../../shared/safety";

export interface RecoveryFailure {
  package: string;
  error: string;
}

export interface RecoveryResult {
  restored: string[];
  failed: RecoveryFailure[];
  message: string;
}

export type RebootMode = "normal" | "recovery" | "bootloader";

export interface RebootResult {
  ok: boolean;
  message: string;
}

export interface TweaksState {
  hdmi_control_enabled: string | null;
  hdmi_control_auto_wakeup_enabled: string | null;
  hdmi_control_auto_device_off_enabled: string | null;
  hdmi_system_audio_control_enabled: string | null;
  match_content_frame_rate: string | null;
  long_press_timeout: string | null;
  window_animation_scale: string | null;
  transition_animation_scale: string | null;
  animator_duration_scale: string | null;
  /// global.background_process_limit, written by earlier versions. Android
  /// ignores it (#99); read only so a leftover value can be removed.
  background_process_limit: string | null;
  /// The cached background process limit Android is applying
  /// (CUR_MAX_CACHED_PROCESSES), or null when the device did not report it.
  cached_process_limit: number | null;
  /// Encoded audio passthrough: "0" Auto, "1" Never, "2" Always, "3" Manual.
  encoded_surround_output: string | null;
  /// Comma-separated AudioFormat encodings; applies only in Manual mode.
  encoded_surround_output_enabled_formats: string | null;
  /// secure.screensaver_components — the active Daydream's ComponentName, or
  /// null when no screensaver is configured.
  screensaver_components: string | null;
  /// secure.screensaver_enabled — whether Daydream runs at all. A cleared
  /// component with this still on can fall back to a framework/vendor
  /// default, so "None" has to turn this off too.
  screensaver_enabled: string | null;
}

export type SettingNamespace = "global" | "secure" | "system";

/// A setting pinned to survive a reboot. Android drops some settings (notably
/// background_process_limit) back to their default on restart, and the device
/// can't re-apply them itself — no cron, no root, and Shield ships no
/// wireless-debugging UI — so the app re-writes them on the next connect.
export interface BootTweak {
  namespace: SettingNamespace;
  key: string;
  value: string;
}

export interface ReapplyResult {
  /// Keys that had drifted and were re-written. Empty when nothing changed.
  reapplied: string[];
  ok: boolean;
  message: string;
}

export interface SystemInfo {
  battery_percent: number | null;
  input_method: string | null;
  play_protect_enabled: boolean | null;
  user_rotation: string | null;
  screen_off_timeout: string | null;
  location_mode: string | null;
  developer_options: string | null;
  doze_enabled: string | null;
}

export type SystemScreen =
  | "notification_shade"
  | "system_updates"
  | "wifi_settings"
  | "bluetooth_settings"
  | "display_settings"
  | "app_settings"
  | "developer_options";

export interface ConsoleResult {
  ok: boolean;
  output: string;
}

/// One background-command log entry (Console tab's Activity section).
export interface ActivityEntry {
  id: number;
  /// Milliseconds since the Unix epoch.
  ts_ms: number;
  /// "adb" | "scrcpy" — which subsystem ran the command.
  source: string;
  command: string;
  output: string;
  ok: boolean;
}

export interface RecordResult {
  ok: boolean;
  message: string;
  path: string | null;
}

export interface QuickApp {
  name: string;
  package: string;
  description: string;
  source: string;
  repo: string | null;
  asset_match: string | null;
  /// Pinned release tag (prerelease-only builds); null = latest.
  tag: string | null;
  fallback_url: string | null;
}

export interface QuickAppRow extends QuickApp {
  installed: boolean;
}

export interface ShizukuStatus {
  installed: boolean;
  running: boolean;
}

export interface WriteResult {
  ok: boolean;
  message: string;
}

export type DisplayScalePreset = "uhd_4k" | "fhd_1080p" | "hd_720p" | "reset";

export interface DisplayScaleResult {
  ok: boolean;
  message: string;
}

export interface CurrentDisplayScaling {
  size: string;
  density: string;
}

export type OptimizeMode = "optimize" | "restore";

export type SkipReason = "not_installed" | "already_disabled" | "already_enabled" | "user_choice";

export type OptimizeAction =
  | { kind: "disable" }
  | { kind: "uninstall" }
  | { kind: "enable" }
  | { kind: "skip"; reason: SkipReason };

export interface OptimizePlanItem {
  entry: AppEntry;
  action: OptimizeAction;
  memory_mb?: number | null;
}

export interface OptimizePlan {
  mode: OptimizeMode;
  items: OptimizePlanItem[];
}

export type PerformanceProfile = "optimized" | "default";

export interface PerformanceResult {
  ok: boolean;
  message: string;
}

// --- Display / Audio wizards + media-app integration (Best Practices 2026) ---

export interface DisplayModeOption {
  resolution: string;
  refresh_hz: number;
  active: boolean;
}

export interface DisplayReport {
  current: DisplayMode;
  supported_modes: DisplayModeOption[];
  match_content_frame_rate: string | null;
}


export interface AudioFormatRow {
  id: number;
  key: string;
  label: string;
  enabled: boolean;
}

export interface AudioReport {
  mode: SurroundMode | null;
  mode_raw: string | null;
  formats: AudioFormatRow[];
  supported_encodings: string[];
  active_device: string | null;
}

export interface MediaApps {
  kodi: boolean;
  plex: boolean;
  /// Installed per-app refresh-rate app's package id, if any.
  refresh_rate_app: string | null;
}

export type AdjustRefreshRate = "off" | "always" | "on_start_stop" | "on_start";

export interface KodiProfile {
  adjust_refresh_rate: AdjustRefreshRate;
  sync_playback_to_display: boolean;
  hdr_display: boolean;
  passthrough: boolean;
  ac3_passthrough: boolean;
  eac3_passthrough: boolean;
  dts_passthrough: boolean;
  truehd_passthrough: boolean;
  dtshd_passthrough: boolean;
  ac3_transcode: boolean;
  resolution_whitelist: string[];
  whitelist_pulldown: boolean;
  whitelist_double_refresh: boolean;
}

export interface StageResult {
  ok: boolean;
  message: string;
  device_path: string;
}

export function deviceTypeLabel(t: DeviceType): string {
  switch (t) {
    case "shield":
      return "Nvidia Shield";
    case "google_tv":
      return "Google TV";
    case "unknown":
      return "Unknown";
  }
}

/// True when the connected device is the given device-specific class. Used to
/// gate Shield-only / Google-TV-only config sections. For an Unknown device
/// (detection failed) this is always false, so device-specific sections hide —
/// safer than showing controls that may not apply to an unrecognized box.
export function isDeviceType(t: DeviceType, specificTo: "shield" | "google_tv"): boolean {
  return t === specificTo;
}

/// Short noun for the connected device, for inline copy on settings that apply
/// to both device types (e.g. "lets the Shield power your TV" → device-correct
/// wording). Falls back to a neutral "device" for Unknown.
export function deviceNoun(t: DeviceType): string {
  switch (t) {
    case "shield":
      return "Shield";
    case "google_tv":
      return "Google TV";
    case "unknown":
      return "device";
  }
}

export function riskBadgeClass(r: RiskTier): string {
  return `risk-${r}`;
}
