# Screenshots

The [project README](../../README.md) uses the desktop walkthroughs and four still images. Regenerate them after a UI change with the following command.

```sh
npm run screenshots    # from v2/
```

The command runs three steps.

1. **`screenshots/capture.mjs`** — boots the dev server with `VITE_DEMO=1` (fixture-backed `invoke()`, see [`../src/lib/demo-mock.ts`](../src/lib/demo-mock.ts) — no device needed), then drives a headless Chromium through all thirty screens at a fixed retina viewport in **both themes**: dark frames to `frames/`, light frames to `frames-light/` (the light run sets `prefers-color-scheme: light`, which the app's Auto theme follows).
2. **`screenshots/build-gif.sh`** — stitches each frame set into a GIF with ffmpeg (two-pass palette): `frames/` → `gallery.gif`, `frames-light/` → `gallery-light.gif`.
3. **`scripts/refresh-readme-assets.mjs`** refreshes the logo assets and checks that all four README screenshots are present.

## Requirements

- `npx playwright install chromium` once (Playwright is a devDependency; the browser binary is separate).
- `ffmpeg` on PATH (`brew install ffmpeg`).

## What's captured

Devices list → PIN pairing guidance → device Overview → Health → RAM controls and comparison → Playback → Launcher → App List → All apps → Optimize wizard → Tweaks → five device Settings groups → Device tools → Remote → Files → Install APK → Folder & version manager → Snapshot → Logs → Shell → global Snapshots page → Catalog → global Settings → no devices (connection help open).

The demo data is a faithful Nvidia Shield — real package names, the real launcher catalog, and the real merged app list ([`../src/lib/demo-apps.json`](../src/lib/demo-apps.json), regenerated from `../crates/core/data/app-lists/`). It is **not** a real device; it's fixtures so the capture is deterministic and hardware-free.

Commit the two galleries and the four still images in `assets/screenshots/`. The individual theme frames are gitignored.

## Pull-request visual review

The frontend CI job captures both theme frame sets and publishes the `desktop-demo-frames`
artifact for review. Download it into this directory, inspect the changed screens, then run
`bash build-gif.sh` and commit both generated galleries. Captures use demo data only; they do
not connect to a device. CI leaves the branch unchanged.

## Automatic regeneration on release

The `refresh-screenshots` job in [`../../.github/workflows/v2-release.yml`](../../.github/workflows/v2-release.yml) reruns this whole pipeline on every `v2-*` tag and commits the refreshed `gallery.gif` + `gallery-light.gif` back to the default branch — so a release never ships stale screenshots. Those captures render on Linux. The fonts are bundled (see [`../src/fonts/README.md`](../src/fonts/README.md)), so fonts match, but rendering can still differ subtly from a GIF regenerated on a Mac. Both are fine; it's the same UI.

## Tuning

- Screens / order: edit the capture sequence in `capture.mjs`.
- Per-screen hold time, output width: `SECONDS_PER` / `WIDTH` in `build-gif.sh`.
- Demo data (device, health, app states, launchers): `../src/lib/demo-mock.ts`.
