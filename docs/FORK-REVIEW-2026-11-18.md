# Desktop fork review, November 18, 2026

Compared the desktop fork with upstream `5d829e0c`. The integration preserves published ancestry, the fork's updater public key, application identifier and MSI UpgradeCode. Source version is 2.6.0 as requested, one minor above upstream 2.5.0, even though the previous fork source version was 2.6.1. This work does not tag or publish a release.

## Fork additions retained

| Area | Source evidence |
| --- | --- |
| Manual/cancellable RAM reads, restrictions with durable undo, three-trial launch comparisons | [RAM commands](../v2/src-tauri/src/commands/ram.rs), [RAM UI](../v2/src/lib/components/RamOptimizer.svelte), [workflow limits](RAM-WORKFLOW.md) |
| Unified installed-app table, targeted cache/data actions, per-device risk overrides and uptime-aware usage | [App table](../v2/src/lib/components/DesktopAppsTab.svelte), [app commands](../v2/src-tauri/src/commands/apps.rs), [risk overrides](../v2/src-tauri/src/commands/safety_overrides.rs), [usage rules](../v2/src/lib/usage.ts) |
| Grouped device settings, additional controls and verified hardware-bound saved tweaks | [Settings tabs](../v2/src/lib/components/DesktopSettingsTab.svelte), [system commands](../v2/src-tauri/src/commands/system.rs), [saved tweaks](../v2/src-tauri/src/commands/boot_tweaks.rs) |
| Download-and-install catalog, APK version management and Shizuku | [Quick install](../v2/src-tauri/src/commands/quick_install.rs), [folder manager](../v2/src/lib/components/FolderApkTab.svelte), [sideload](../v2/src-tauri/src/commands/sideload.rs) |
| Recording, separate mirror window, storage exploration and activity console | [Recording](../v2/src-tauri/src/commands/recording.rs), [mirror](../v2/src-tauri/src/commands/mirror.rs), [device tools](../v2/src/lib/components/DeviceTools.svelte), [console](../v2/src/lib/components/ConsoleTab.svelte) |

Upstream already supplies device discovery, built-in ADB, APK inspection, the safety catalog, launcher management, snapshots and recovery. Those are credited to upstream rather than presented as fork additions. This is a source and integration review, not an exhaustive security audit or a physical-device acceptance run.

## Integration fixes

- Retained upstream's bounded APK manifest parsing and decoded nonzero install results while preserving the fork's explicit downgrade retry.
- Kept accessible badge help and uptime-aware usage together.
- Retained freezer/touch settings with read-back verification. Added coverage for failed writes and restoring defaults.
- Corrected display read-back verification to accept the device-specific width actually requested: 3839 on Shield, 3840 on other devices.
- Retained upstream changes to snapshot settings, unknown device identity and private DNS verification.

## Issue 2: naming and portable distribution

[Issue 2](https://github.com/LosLonelyDevs/AndroidTV-Optimizer/issues/2) correctly identified inconsistent names. The installer product name was Shield Optimizer, the window title Android TV Optimizer, and the app header ATV Optimizer. Two fork UI explanations also still said Shield Optimizer. Release and bug-report links pointed upstream.

The visible desktop name is now Android TV Optimizer. Historical identifiers and installer registry keys remain for compatibility. References to real Shield devices, Shield-specific tuning and upstream history remain intentional. The support note no longer says the app is built for Shield; only its wording changed, with all tier rules unchanged.

Both desktop release workflows package a Windows portable ZIP and SHA-256 checksum after the installer build. It contains the same executable, required resources and notices. A portable marker disables the installer updater. This is a no-install edition with host-local data, not a USB-contained settings profile. WebView2 remains a system prerequisite. Packaging CI checks archive contents, checksum and rejection of non-executable inputs.

## Validation before the portable and branding changes

Windows: frontend type-check had zero errors and warnings; production build passed; Clippy passed. Desktop Rust tests: 360 passed, 2 hardware tests ignored. Shared core: 811 passed. Focused UI checks passed for fork tools, RAM sessions, accessible badges, native install results and display scaling.

The screenshot pipeline captured 30 screens in each theme from offline fixtures in fresh browser profiles. The four README images use the same fixtures and documentation network addresses. The RAM screenshot includes the full comparison.

## Portable and branding checks

- Built the Windows NSIS installer and packaged its release executable in the portable ZIP. Checked the resources, portable marker and SHA-256. The packaging test also rejected an invalid executable.
- Windows NSIS maintenance callback tests passed all 14 compiled scenarios. Static assertions checked the new display name, legacy registry keys and pinned MSI UpgradeCode; this is not a real installed-copy upgrade test.
- Desktop Rust tests: 361 passed, 2 hardware tests ignored. Shared core: 811 passed. Clippy and formatting passed. Frontend type-check had zero errors and warnings; production build passed.
- Browser regressions passed for portable updates, fork tools, ADB errors, local-network guidance, app reports, bug reports and update history. Portable Settings has no installer update controls.
- Icon/layout checks passed for 120 controls across 14 screens, including both themes, 720px layouts and 200% text. Wide light-theme and narrow dark-theme captures were visually inspected; no layout changes were needed for the longer name.
- `actionlint` passed for all changed workflows. Test wiring passed: all 68 test files have scripts and every test script is included in CI.

The installer has not been installed over a real user's previous installation, and no physical TV was controlled. No release tag or updater manifest was published.
