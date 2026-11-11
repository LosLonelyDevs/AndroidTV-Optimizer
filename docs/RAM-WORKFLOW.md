# RAM workflow and benchmarks

Open a connected TV's **RAM** tab. This desktop feature is included in the fork's
2.6.0 source. No physical Shield RAM or speed improvement has yet been verified.

## Measure, change, compare

1. **Record baseline** while repeating the playback/app-switching workload that
   feels slow. Recording collects up to 12 samples, waiting 15 seconds after
   each completed read. Stop earlier if needed; comparisons need at least three
   available-RAM readings in each session.
2. Review resident apps and their sampled memory. **Keep** shares the App List's
   saved choice. A missing reading is unknown, not zero RAM or proof of an exit.
3. **Restrict background** on an eligible app, read the consequences, and confirm.
   Android may stop its background downloads, sync, notifications, and playback.
   The app stays installed and can still be opened.
4. **Record comparison** using the same workload. Compare available RAM and
   app behavior, including unwanted reloads and broken background functions.
   Use **Undo restriction** to restore the saved setting.

The app verifies the restriction setting separately from its effect. A matching
readback does not establish RAM savings. Restrictions are not per-app RAM caps
and do not necessarily evict cached memory immediately. Cached memory can help
apps resume faster. Available RAM, file cache, and swap usage are separate
readings; swap usage alone does not establish harmful memory pressure.

## Benchmark the delta

Choose a reviewed app under **Before / after benchmark**, then a launch mode:

- **Resume (pre-open):** opens the app, returns Home, waits two seconds, and times
  the next launch. Android may report a warm, hot, or cold launch depending on
  what actually happened.
- **Cold (force-stop):** force-stops only the selected app, returns Home, waits
  two seconds, and times its launch. The confirmation explains the interruption
  and possible loss of unsaved work or background activity.

Each run performs three trials, with a three-second settling period before RAM
sampling. Apply the selected change between **Benchmark baseline** and
**Benchmark comparison**. Comparison reuses the baseline app and mode. Keep
content, app settings, device temperature, and other workloads similar.

Results show median launch time, observed min/max, median available RAM, and
median selected-app PSS. Speed delta includes milliseconds and a percentage
when the baseline is nonzero. Overlapping time ranges are inconclusive, not a
speedup. Different hardware IDs, Android users, firmware, app versions, or
reported launch states invalidate the comparison. Incomplete runs are never
compared. **Stop benchmark** stops after the in-flight trial; the TV remains on
the last launched app.

Launch time is Activity Manager `TotalTime`, not first decoded video frame,
fully loaded content, network throughput, or a universal performance score.
Three trials are a quick check, not a controlled causal experiment.

## Safety and recovery

Restriction candidates must be enabled, reviewed-safe, non-system packages with
unique UIDs. Mandatory protected packages, Home, the foreground app, media
sessions, accessibility services, and foreground services are excluded. Unreadable
active-state evidence blocks restriction. The benchmark can explicitly launch
the selected foreground app but still excludes protected services. No process
is automatically killed or restricted because it ranks high in RAM.

The desktop saves an undo journal before sending the AppOp write. A failed or
ignored write retains that journal. Undo requires the same verified `ro.serialno`,
Android user, package, and UID; it refuses a conflicting external mode change.
Default is restored as Default, not Allow. Each write also checks hardware ID
and Android user in the same device shell expression. There is no automatic
reapplication after reboot or reconnection.

Journals live in the app's data folder under `ram-undo/`; retaining that folder
preserves recovery across app restarts/upgrades. Schema 1 is the initial format;
corrupt or unsupported schemas fail closed. The existing snapshot format is
unchanged. Session measurements remain in memory and clear when leaving the
device page. Switching tabs stops sampling but retains completed measurements.

## Validation boundaries

Windows validation on 2026-11-11: Rust formatting and Clippy passed; the workspace
passed 1,299 tests with five existing ignored tests. Svelte check reported zero
errors/warnings, the frontend build passed, and comparison-statistics tests
passed. All four targeted offline scenarios passed, covering the existing Health
screen plus RAM restriction/undo, unsupported AppOps, benchmark cancellation,
and matched before/after runs. Layout checks passed for both themes, the 720px
minimum, and 200% text; both screenshot galleries were regenerated. This is
local Windows and simulated-device evidence, not macOS/Linux or physical Shield
acceptance.

Automated coverage exercises real command handlers against simulated devices:
restriction readback, exact undo, ignored writes, identity/user mismatch,
protected apps/services, journal schema refusal, memory attribution, and launch
timing/comparison parsing. UI coverage includes Keep, confirmation, restriction,
undo, and matched benchmark runs. Offline data does not establish an actual
Shield memory or speed gain.

On physical hardware, verify command support, sustained before/after results,
foreground launch, app switching, background features, and undo after reconnect
and reboot. PSI and numeric process UID/PID readings are capability-dependent;
unavailable metrics stay unavailable. The initial version uses bounded samples;
historical `procstats`, restart-cause attribution, thermal-controlled trials,
and decoder/frame-time benchmarks are not included.
