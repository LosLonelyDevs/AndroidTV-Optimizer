# Shield RAM optimization proposal

Research date: 2026-11-11. Source audit: desktop fork 2.6.0 at `8709ac73`. Proposal only; no Shield settings changed and no RAM savings measured.

Implementation follow-up: the desktop now has the [RAM workflow and launch benchmark](RAM-WORKFLOW.md). The design below remains the original research proposal. The delivered scope uses bounded session samples; OS `procstats` history and restart/kill-cause attribution remain follow-up work. Physical Shield effectiveness is still unverified.

## Recommended feature

Add **Reduce background RAM**: find apps that consume memory while unused, offer a reversible restriction on their background work, and verify the result during the same playback/app-switching workload.

The important new control is **Restrict background activity**. It fills the gap between the existing Force stop and Disable actions: a selected app stays installed and can still be opened, while Android limits its background work. AOSP documents `RUN_ANY_IN_BACKGROUND` restrictions for Android 9 and later. Restrictions are not a per-app RAM ceiling and do not guarantee immediate eviction from memory. Their sustained RAM benefit on Shield must be demonstrated. [AOSP app restrictions](https://source.android.com/docs/core/power/app_mgmt).

## What is new versus the current app

| Capability | Current implementation | Proposed addition |
|---|---|---|
| Identify memory users | Current RAM totals and per-process readings | Background residence time, typical/peak memory, and observed restarts over a session |
| Reduce persistent background work | Force stop or disable an app | Reversible per-app background restriction, after checking device support |
| Explain a high RAM reading | Aggregate used/free RAM and a ZRAM reading | Available memory, reclaimable cache, pressure evidence, and the source/limits of each reading |
| Prove a change worked | Point-in-time/live readings | Matched before/after sessions, sustained savings and app-switching regressions |

Audit anchors: [shared health sampling](../v2/crates/core/src/commands/health.rs), [RAM parser](../v2/crates/core/src/adb/parse.rs), [performance settings](../v2/crates/core/src/commands/optimize.rs), [advanced tweaks](../v2/src/lib/components/AdvancedTweaksTab.svelte), and [app management](../v2/src/lib/components/DesktopAppsTab.svelte). No typed `RUN_ANY_IN_BACKGROUND` restriction workflow or `procstats` history collector was found in the audited command/frontend paths.

## Three parts, in priority order

### 1. Restrict apps that do unnecessary background work

Offer a per-app action for a user-selected, eligible app, with its expected consequences visible before applying it. Android's background restrictions can affect services, jobs, alarms, and network activity; manufacturer behavior varies. Downloads, sync, notifications, and background playback may stop working as expected. [Android background restrictions](https://developer.android.com/topic/performance/background-optimization).

Prototype with the AOSP-documented `cmd appops set <package> RUN_ANY_IN_BACKGROUND ignore` operation through the existing ADB driver. Probe support, capture the exact previous package/user operation state, apply, and read it back. A successful command or matching setting alone proves only configuration, not memory savings. Do not silently substitute another operation if the firmware rejects it. Older-app handling with `RUN_IN_BACKGROUND` is a separate compatibility case to test, not a blanket second write. [AOSP integration and test commands](https://source.android.com/docs/core/power/app_mgmt).

Undo restores the captured prior state, including an unset/default state; it must not blindly write `allow` or reset every AppOp. Saved actions belong to verified hardware identity and the relevant Android user/package. Refuse ambiguous shared-UID targets in the initial release. Persisted rollback data needs an explicit schema/migration design before implementation.

Never apply restrictions automatically based on a memory ranking. Exclude mandatory protected packages and the current Home app; preserve user-selected keep apps and required services such as casting, Plex server, VPN, accessibility, and active media playback. Unknown classification or activity is insufficient for an automatic recommendation. Reuse curated safety classification and add checks specific to background restrictions. A keep selection protects against this optimizer's actions; it cannot reserve RAM or prevent Android from killing an app.

**Expected benefit:** less sustained background work and potentially a smaller resident working set for an offending app. An app that only holds harmless cached memory may show no useful improvement. Never promise a fixed MB gain or prevention of every restart.

### 2. Find persistent consumers and restart churn

Add a short, user-started observation session and a historical view from `dumpsys procstats`, where available. Show each app's foreground/background/cached time and sampled average/peak PSS. Android documents `procstats` as an over-time view, unlike a single `meminfo` snapshot. Use its actual reporting window and sample count, not an invented precise history. [Android memory diagnostics](https://developer.android.com/tools/dumpsys#meminfo).

Distinguish a long-running service from a cached app. Track process instances with PID plus start time where accessible, aggregate package subprocesses carefully, and mark ambiguous attribution. An observed restart is not proof of a low-memory kill or a memory leak. Require matching termination evidence to name a cause. This helps choose an intervention rather than repeatedly closing an app that immediately returns.

### 3. Verify sustained RAM relief

Record available RAM over time, package memory, and app-switching/reload observations. Add memory pressure stall information (PSI), low-memory-kill evidence, and swap activity only when the device exposes them. Android's low-memory killer uses pressure/thrashing signals on supported versions; PSI requires kernel support. Missing or denied metrics remain unavailable, never zero. [AOSP low-memory killer](https://source.android.com/docs/core/perf/lmkd).

Separate truly free RAM from available/reclaimable memory. Existing health fallback code stores `MemAvailable` in `free_mb`, while the main parser reads the `dumpsys` Free RAM summary. Preserve metric provenance before comparing sessions. Likewise, the first ZRAM number currently parsed into `swap_mb` must not be presented as a complete swap-capacity or pressure measurement.

Start with lightweight totals and slower package sampling; measure observer overhead before selecting polling intervals. Compare repeated baseline and restricted runs with the same app versions, content, launch order, and similar temperature. Report median/low available memory and app memory separately; summing process PSS changes is not an exact system-wide RAM saving. A useful result persists after app switching and does not introduce unwanted reloads or broken background functions.

## Approaches to leave out

- **Treating every cached MB as waste.** Android keeps cached processes to speed up returning to apps and can reclaim them when necessary. A lower used-RAM percentage alone is not the success criterion. [Android memory management](https://developer.android.com/topic/performance/memory-overview).
- **Selling the existing cached-app freezer as RAM relief.** It keeps processes in RAM while stopping CPU execution. It is already enabled by the current Optimize performance profile. [AOSP cached-app freezer](https://source.android.com/docs/core/perf/cached-apps-freezer).
- **Another background-process-limit setting write.** Current UI explicitly reports that the legacy global setting is ignored by Android. Do not advertise it as an effective cap without verifying the actual framework limit and behavior.
- **Automatic kill loops, storage-cache purges, or root ZRAM/LMKD tuning.** These are outside the proposed first version. A persistent background restriction and a measured result are the relevant experiment.

## Delivery and acceptance

First validate the restriction on a physical Shield with a nonessential test app: observe its background behavior, capture/apply/read back the operation, repeat the workload, restore the exact prior state, and verify recovery. Check foreground launch, reconnect/reboot behavior, and firmware differences. Test both a changed workload and an unaffected control app. Unsupported commands and readable-but-ineffective settings must produce an honest unsupported/no-demonstrated-benefit result.

Then build the guided flow: **Observe → Review apps → Restrict selected apps → Compare → Keep or undo**. Keep explanatory detail in accessible help; show the relevant service/playback consequences at the decision. Reuse the pure engine, single ADB driver, JSON package catalog, and identity/snapshot guarantees. Add parser/safety/rollback tests and offline UI coverage; physical before/after evidence remains a separate acceptance gate.

This is the strongest RAM-focused addition to investigate because it can change persistent behavior without removing an app. Its benefit remains conditional on finding an unnecessary background consumer and verifying the restriction on the target Shield. Research and source inspection are complete; implementation and physical-device validation have not been performed.
