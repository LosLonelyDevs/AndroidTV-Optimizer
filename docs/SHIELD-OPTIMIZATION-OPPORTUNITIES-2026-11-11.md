# Shield TV: worthwhile optimization additions

Research date: 2026-11-11. Scope: the desktop fork rebased onto upstream `9bb838f4` (2.5.0), with fork source version 2.6.0. These are proposals, not implemented features or measured gains on a physical Shield.

The best next step is a **Find my bottleneck** flow: measure the problem, offer one relevant change, then repeat the measurement. The largest likely gains come from fixing a poor streaming path, an unnecessary transcode, or slow storage. More blanket debloating is a weaker general-purpose bet.

## What the app already covers

The source audit found existing debloat and launcher workflows, curated safety gates, app cache/data controls, storage capacity/category reports, CPU/RAM/network-rate sampling, temperature and thermal-status reporting, display controls, frame-rate guidance, audio passthrough, Kodi configuration, animation/background-process settings, and other advanced tweaks. The proposals below extend those capabilities rather than rename them.

Relevant implementation points:

- [Shared health and resource sampling](../v2/crates/core/src/commands/health.rs), plus [desktop storage breakdown](../v2/src-tauri/src/commands/health.rs).
- [Device storage tools](../v2/src/lib/components/DeviceTools.svelte) and [app manager](../v2/src/lib/components/DesktopAppsTab.svelte).
- [Display controls](../v2/src/lib/components/DisplayTab.svelte), [audio and Kodi controls](../v2/src/lib/components/AudioTab.svelte), and [Kodi settings generation](../v2/crates/core/src/engine/kodi.rs).
- [Grouped device tweaks](../v2/src/lib/components/ForkTweaksTab.svelte) and [advanced tweaks](../v2/src/lib/components/AdvancedTweaksTab.svelte).

No dedicated LAN throughput benchmark, Plex session/transcode advisor, bounded storage-speed benchmark, sustained performance comparison, or repeatable app-start benchmark was found in the audited desktop paths. Existing traffic counters show current traffic, not the capacity of a connection.

## Ranked additions

Ranking is engineering judgment based on the failure modes each feature can fix. It is not a measured ranking across Shield owners. Benefit is conditional on finding the corresponding bottleneck.

| Priority | Addition | Most useful when | Potential benefit | Effort |
|---|---|---|---|---|
| 1 | LAN/NAS streaming-path test | High-bitrate local video buffers | High when the network path is limiting playback | Medium to high |
| 2 | Plex playback and transcode advisor | A capable client unexpectedly triggers conversion | High when transcoding overloads the server or reduces quality | Medium |
| 3 | Storage speed and app-placement advisor | Apps or libraries live on slow removable storage | High for affected app loads and library operations | Medium to high |
| 4 | Sustained thermal/performance session | Performance worsens after several minutes | Medium to high if throttling or the selected power mode is responsible | Medium |
| 5 | Repeatable before/after app-start test | Users cannot tell whether a tweak helped | Indirect: identifies useful changes and regressions | Medium |

### 1. Test the actual streaming path

Add a guided test between the Shield and a selected local endpoint. Separate negotiated link speed, sustained payload throughput, variability, and observed loss/retransmissions where available. Test the actual NAS/media-server path when possible; a laptop-to-Shield result cannot certify a different NAS path. Compare results with the selected media's bitrate and known peaks, while making any chosen headroom threshold visible as a heuristic.

Useful fixes include switching a weak Wi-Fi path to Ethernet, correcting a link that unexpectedly negotiated at 100 Mbps, improving AP placement/band selection, or addressing the NAS side. NVIDIA recommends Ethernet for maximum performance and specifically recommends wired connections on both sides of a NAS streaming setup. A nominal gigabit connection is not proof of gigabit application throughput. [NVIDIA network guidance](https://support-shield.nvidia.com/shield-tv-pro-user-guide/How_to_Optimize_Internet_and_Video_Performance.htm), [NVIDIA Plex/NAS guidance](https://nvidia.custhelp.com/app/answers/detail/a_id/4171/).

**MVP:** read-only link inventory plus a short, cancellable LAN test using a reviewed companion/helper. Validate ABI, permissions, licensing and firmware support first. Bind listeners narrowly, require an explicit endpoint, and clean up temporary files/processes. ADB transfer speed must be labeled as an ADB-path test, not streaming capacity. Do not infer a DNS problem from insufficient sustained throughput.

**Acceptance:** reproduce a deliberately limited connection, distinguish it from a server/storage limit, and show the improvement after changing the relevant link. No automatic router changes.

### 2. Explain why Plex is transcoding

Add an advisor that reads a user-approved server session or imported diagnostic report and explains Direct Play, Direct Stream, audio conversion, and video conversion separately. Correlate the decision with selected quality, subtitle format, codec/container compatibility, and the audio path. Offer a specific correction, then inspect a new playback session.

Plex documents that incompatible subtitles can force full video transcoding even when the media otherwise qualifies for Direct Play. Direct Stream repackaging is comparatively inexpensive and should not be flagged as a problem merely because it is not Direct Play. [Plex playback modes](https://support.plex.tv/articles/200250387-streaming-media-direct-play-and-direct-stream/).

**MVP:** session import first, optional authenticated integration second. No token scraping from app storage. Keep tokens out of logs and exports. Suggest original quality only when the link can carry it, and an alternative subtitle/audio track only when one exists. Do not promise that every file can Direct Play.

If the Shield also hosts the server, show competing workloads and relevant transcoding constraints. Model detection matters: Plex Media Server is unavailable on the 2019 tube model. [Plex Shield limitations](https://support.plex.tv/articles/221099648-limitations-when-running-plex-media-server-on-nvidia-shield/), [supported Shield setup](https://support.plex.tv/articles/221099988-setting-up-and-managing-plex-media-server-on-nvidia-shield/).

**Acceptance:** explain a subtitle-triggered transcode, leave a healthy Direct Stream session alone, and confirm the resulting playback mode after a recommended change.

### 3. Diagnose slow storage, not just full storage

Extend the current capacity report with storage topology, accessible app locations, and a bounded speed/latency test. Distinguish internal, adopted, removable, and network storage where the OS exposes that information. Label inaccessible paths unknown.

NVIDIA distinguishes microSD expansion on Shield TV from USB storage on Shield TV Pro and recommends USB 3.0 flash drives for the Pro's speed/performance. This supports checking the storage path, but does not establish the speed of an individual drive. [NVIDIA storage guidance](https://nvidia.custhelp.com/app/answers/detail/a_id/4889/).

**MVP:** test only a newly created file in an explicitly selected writable directory. Cap bytes/time, check free space, cancel safely, and delete only that file. Report sequential performance separately from latency; disclose page-cache effects. Do not claim a cold/raw-disk benchmark when unprivileged access cannot provide one.

Offer measured recommendations: move a frequently used app to faster internal storage where Android permits, relocate a large media library appropriately, or replace a demonstrably slow device. Formatting/adopting a drive is destructive and stays a separate manual operation with explicit consent.

**Acceptance:** distinguish a deliberately slow external device from healthy internal storage; verify no user files changed and temporary data was removed after cancellation.

### 4. Record a sustained performance session

The app already reports thermal information. The new feature is a timeline connecting workload, temperature/status, accessible clocks, resource load, and an observable slowdown. Compare the same workload before and after one change. Start with normal playback or an app the user chooses, not an unsolicited stress test.

Offer verified, model-specific guidance for Shield's Processor mode. NVIDIA documents Max performance and Optimized, with a noise/temperature tradeoff; menu locations and ADB controllability require firmware verification. Do not invent writable settings keys. A fan-mode shortcut can be a later addition after the supported control path is confirmed. [NVIDIA processor-mode guidance](https://support-shield.nvidia.com/shield-tv-pro-user-guide/How_to_Optimize_Internet_and_Video_Performance.htm).

**MVP:** collect a cancellable session, show unavailable signals explicitly, and offer ventilation or supported power-mode guidance when evidence supports it. Temperature alone does not prove throttling. Android warns that older devices may not update thermal status reliably; a reported NONE is not always evidence that throttling is absent. [Android Thermal API limitations](https://developer.android.com/games/optimize/adpf/thermal).

**Acceptance:** retain unknown readings, avoid universal temperature thresholds, and confirm whether a repeatable slowdown changed after the intervention. Never disable thermal protection or change root governors.

### 5. Prove that a tweak helped

Add a comparison mode for a selected app's startup and, where supported, UI frame timing. Record firmware/app versions and relevant settings, separate warm from cold starts, repeat enough runs to show a distribution, and report median/tail latency plus variability. Offer one change at a time with rollback.

Android documents different startup states and measurement through activity-manager timing. Time to initial display is not the same as fully loaded content or responsive playback. [Android startup measurement](https://developer.android.com/topic/performance/vitals/launch-time).

**MVP:** explicit app-launch trials, with confirmation before stopping the selected app for a cold run. Keep an untreated baseline. UI rendering statistics must not be presented as decoder/video dropped-frame measurements. Use verified hardware identity for saved comparisons; never join sessions solely by IP address.

**Acceptance:** distinguish warm/cold runs, handle noisy results without declaring a winner, and detect a deliberate startup regression. This supplies evidence for optimization rather than directly speeding up the device.

## Additional ideas worth a second pass

- **Playback-path validation:** expand existing audio/display controls into a guided check of the actual active refresh rate, audio format, and AVR/TV chain during a sample. Benefit is fixing mismatch or judder; existing setting toggles alone are not a new feature. Some apps/DRM paths will not expose enough evidence.
- **Shield server workload advisor:** for supported models hosting Plex, correlate background analysis/transcoding with foreground slowdowns. Recommend scheduling or moving work only after observing contention. Never disable the server just because it consumes RAM.
- **Post-update capability check:** compare readable settings and app capabilities before/after firmware updates, then highlight changed or unavailable controls. Requires a maintained model/firmware matrix; do not assume every Shield accepts the same shell commands.
- **Undo-backed optimization experiments:** reuse the existing versioned snapshots and safety rules to compare one reversible change at a time. Report both gains and regressions, including broken notifications, casting, or background playback.

## Recommended delivery order

Start with the reusable measurement/session foundation and LAN diagnosis. Add the Plex advisor next, then storage testing. Follow with sustained thermal sessions and verified model-specific controls. Read-only/manual guidance is the first milestone; automation follows only after physical validation on the relevant Shield models and firmware.

Do not spend the next cycle on a RAM booster, routine cache purge, indiscriminate background killing, root-only governor changes, or universal forced HDR/refresh settings. The current app already exposes several powerful controls. New work should show when a change helps, when it hurts, and when the available evidence cannot decide.

No physical-device benchmark, live Plex-session test, or automated optimization was performed for this research. Expected benefits and effort rankings remain proposals pending those tests.
