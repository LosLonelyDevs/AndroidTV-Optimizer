# RAM workflow and benchmarks

Open a connected TV's **RAM** tab. This desktop feature is included in the fork's
2.6.1 source. No physical Shield RAM or speed improvement has yet been verified.

Unavailable action buttons explain why on hover, keyboard focus, or tap. Press
Escape to dismiss the explanation. Opening it does not perform the blocked action.

## Measure, change, compare

1. In **Apps in RAM**, review the **Suggestion** shared with Health and the
   background-eligibility reason beneath it. **Keep** shares the App List's saved
   choice. A missing reading is unknown, not zero RAM or proof of an exit.
2. Choose **Restrict background**, read the interruption and background-work
   consequences, then choose **Measure and restrict** once.
3. The app automatically measures three launches, applies and verifies the
   restriction, then measures three more. It returns to Home after every trial.
   A failed, mismatched, or stopped baseline makes no restriction change. If the
   comparison fails or stops after the change, the restriction stays applied and
   **Undo restriction** remains available.
4. Results appear after the first baseline trial. Compare launch times, available
   RAM, and app PSS. Missing metrics remain unknown and are never treated as zero.

Apps in RAM includes active and cached processes; it does not mean every app is
doing work. Eligible installed apps without verified memory readings can also
appear so they can be measured. Suggestion retains Health's removal advice;
background eligibility is a separate, stricter check. Restricting limits
background activity without disabling or uninstalling the app.

For a custom playback/app-switching workload, expand **Record a custom RAM
workload**. Manual recording collects up to 12 samples with 15-second waits and
requires at least three available-RAM readings in each session for comparison.

The app verifies the restriction setting separately from its effect. A matching
readback does not establish RAM savings. Restrictions are not per-app RAM caps
and do not necessarily evict cached memory immediately. Cached memory can help
apps resume faster. Available RAM, file cache, and swap usage are separate
readings; swap usage alone does not establish harmful memory pressure.

## Benchmark the delta

Automatic optimization uses the app being restricted and defaults to Resume.
After a baseline exists, **Before / after benchmark** also offers manual reruns
and a launch-mode selector:

- **Resume (pre-open):** opens the app, returns Home, waits two seconds, and times
  the next launch. Android may report a warm, hot, or cold launch depending on
  what actually happened.
- **Cold (force-stop):** force-stops only the selected app, returns Home, waits
  two seconds, and times its launch. The confirmation explains the interruption
  and possible loss of unsaved work or background activity.

Each run performs three trials, with a three-second settling period before RAM
sampling. The automatic flow applies the restriction between runs; manual
comparison reuses the baseline app and mode. Keep
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
