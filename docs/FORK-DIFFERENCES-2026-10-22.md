# Features unique to this fork

Compared on October 22, 2026 against `bryanroscoe/shield_optimizer` at
`7f1f98bd33ec0a86d232ce0d7e780de0b30297eb`, after rebasing the desktop fork onto
that commit. This is a source comparison of the current branches, not a
comparison against upstream's last published installer. Desktop version remains
2.3.3; this sync does not create a release.

## Device connection and settings

| Fork addition | What it adds beyond upstream | Source |
| --- | --- | --- |
| Recent network devices | Remembers up to eight addresses with device labels, retries saved addresses before a LAN scan, and provides individual Clear controls. Clearing a saved address stops this app's saved-address retries; discovery or ADB pairing can still reconnect it. | [Devices page](../v2/src/routes/+page.svelte) |
| Grouped Settings | Display, Audio, Input, and Device groups, with launcher management under Display and separate advanced tweaks. | [Device page](../v2/src/routes/devices/%5Bserial%5D/+page.svelte) |
| Additional device controls | Screen timeout including Never, rotation, location, developer options, ambient display/Doze, Play Protect, NTP repair, wake/sleep, system-settings shortcuts, and launch by package name. | [System controls](../v2/src/lib/components/SystemTab.svelte), [commands](../v2/src-tauri/src/commands/system.rs) |
| Remember background process limit | Pins the chosen limit and reapplies it if it has drifted when the desktop app reconnects. This requires the desktop app to connect; it is not an autonomous service running on the TV. | [Input/device tweaks](../v2/src/lib/components/TweaksTab.svelte), [reapply logic](../v2/src-tauri/src/commands/boot_tweaks.rs) |
| Extra performance actions | Speed-profile compilation, plus app-freezer and touch-response settings in the optimized/default performance profiles. | [System commands](../v2/src-tauri/src/commands/system.rs), [performance settings](../v2/crates/core/src/commands/tuning.rs) |

## Apps and installation

| Fork addition | What it adds beyond upstream | Source |
| --- | --- | --- |
| Unified app table | Combines curated and discovered installed apps in the fork's shared row layout, with recommendations, risk badges, and maintenance actions. | [App row](../v2/src/lib/components/DesktopAppRow.svelte), [device page](../v2/src/routes/devices/%5Bserial%5D/+page.svelte) |
| Per-app cache/data clearing | Adds targeted cache clearing and confirmed data wipes, plus a device-wide Kill background action. Data wipes retain protected-package checks. Upstream already has force-stop and global cache trimming. | [App commands](../v2/src-tauri/src/commands/apps.rs) |
| Per-device risk overrides | Lets the user mark understood packages safe in the UI and remove that override later. These marks soften warnings; they cannot override the mandatory protected-package block. | [Safety overrides](../v2/src-tauri/src/commands/safety_overrides.rs) |
| Uptime-aware usage recommendations | Uses the available observation window to distinguish "not since reboot" from long-term lack of use, reducing misleading removal suggestions after a reboot. Upstream already collects app usage. | [Usage rules](../v2/src/lib/usage.ts), [usage reporting](../v2/src-tauri/src/commands/apps.rs) |
| Download-and-install catalog | Resolves release assets, selects an appropriate APK for the device ABI, downloads it, and installs it. Current entries: TizenTube Cobalt, Flicky, MATVT, ObtainX, Aurora Store, and LocalSend. Upstream's curated sideload links do not provide this same automated flow. | [Catalog](../v2/crates/core/data/app-lists/quick-install.json), [installer](../v2/src-tauri/src/commands/quick_install.rs) |
| Shizuku setup | Dedicated setup/start action and running-state checks. | [Quick-install commands](../v2/src-tauri/src/commands/quick_install.rs) |
| APK version awareness | Reads installed and candidate APK versions to label upgrades, downgrades, and reinstalls; adds the downgrade install path. This extends upstream's existing APK inspection and sideloading. | [Sideload UI](../v2/src/lib/components/SideloadTab.svelte), [sideload commands](../v2/src-tauri/src/commands/sideload.rs) |

## Media, display, and diagnostics

| Fork addition | What it adds beyond upstream | Source |
| --- | --- | --- |
| Screen recording | Starts Android screen recording, stops it, and pulls the recording to the computer. | [Recording commands](../v2/src-tauri/src/commands/recording.rs) |
| Separate mirror window | Launches a scrcpy mirror window. Upstream uses scrcpy internally for remote input, but does not expose this same mirror-window command. | [Mirror command](../v2/src-tauri/src/commands/mirror.rs) |
| Expanded Display panel | Shows current and supported modes, groups display controls, and provides setup help and the secure-settings permission grant for compatible per-app refresh-rate apps. It does not configure those apps' private profiles. | [Display panel](../v2/src/lib/components/DisplayTab.svelte), [media integration](../v2/src-tauri/src/commands/media_apps.rs) |
| Expanded Audio panel | Auto/Manual/Always/Never surround modes, individual codec choices, and receiver-reported formats. Upstream already has playback analysis and basic audio tuning. | [Audio panel](../v2/src/lib/components/AudioTab.svelte), [audio commands](../v2/src-tauri/src/commands/audio.rs) |
| Kodi configuration generation | Creates an `advancedsettings.xml` from the selected profile and stages it on shared storage for manual import. Plex integration detects the app and provides guidance; it does not rewrite Plex preferences. | [Media integration](../v2/src-tauri/src/commands/media_apps.rs), [Kodi generator](../v2/crates/core/src/engine/kodi.rs) |
| Extra ADB Console and activity history | Adds a separate console view with the commands and output from background ADB/scrcpy activity. Upstream already has an expert shell with bookmarks and device logs. | [Console](../v2/src/lib/components/ConsoleTab.svelte), [activity log](../v2/src-tauri/src/activity_log.rs) |

## Storage, snapshots, and distribution

| Fork addition | What it adds beyond upstream | Source |
| --- | --- | --- |
| Storage drilldown | Directory totals and largest-file inspection under shared storage. Upstream already has a file manager, app storage sizes, and device-to-device file copying. | [Storage commands](../v2/src-tauri/src/commands/files.rs), [Health view](../v2/src/routes/devices/%5Bserial%5D/+page.svelte) |
| More snapshot settings | Adds timeout, rotation, location, Doze, developer options, both Play Protect settings, and NTP server to snapshot tracking and restore validation. These snapshots do not restore app data or undo every device change. | [Snapshot keys](../v2/crates/core/src/engine/snapshot.rs), [snapshot commands](../v2/crates/core/src/commands/snapshot.rs) |
| Windows installer maintenance fix | The custom NSIS template makes "Do not uninstall" cancel setup before invoking an existing MSI uninstaller. Includes compiled regression coverage for 14 maintenance scenarios. | [Installer notes](../v2/src-tauri/installer/README.md), [tests](../v2/tests/installer-maintenance.mjs) |
| Fork release/update channel | Fork-specific release workflow changes, a manual release workflow, updater configuration/feed, and installer identity compatibility settings. Updater signatures are distinct from OS code signing. | [Release workflow](../.github/workflows/v2-release.yml), [manual workflow](../.github/workflows/v2-manual-release.yml), [Tauri configuration](../v2/src-tauri/tauri.conf.json) |
| Fork-specific verification | Additional Rust tests, updated simulated E2E scenarios, installer tests, release-note regression fixes, and an eighteen-screen gallery in both themes. The E2E server is a Cargo example so it stays out of installers. | [E2E](../v2/e2e/README.md), [screenshots](../v2/screenshots/README.md) |

## Features shared with upstream

The basic optimizer, safety catalog, snapshots, launcher recovery, mDNS pairing
and discovery, remote control, device logs, expert shell and bookmarks, app
usage/memory/storage data, APK inspection/sideloading, and file transfer already
exist upstream. The fork adds or extends the behavior described above; it should
not be credited with introducing all of those underlying features.

The upstream launcher curation tests and PR-evidence guidance fetched in this
sync are now present in the fork and are not fork-only features.

This comparison verifies code and UI wiring. It does not establish physical-TV
compatibility for every firmware, codec, recording mode, or package installer.
