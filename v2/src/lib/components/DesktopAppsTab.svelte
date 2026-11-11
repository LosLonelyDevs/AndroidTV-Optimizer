<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  import { page } from "$app/stores";

  import { goto } from "$app/navigation";

  import { open as openDialog } from "@tauri-apps/plugin-dialog";

  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import { Channel } from "@tauri-apps/api/core";

  import { api } from "$lib/api";

  import type {
    Device,
    HealthReport,
    LauncherStatus,
    CurrentLauncher,
    AppEntry,
    SnapshotFile,
    SnapshotApplyPlan,
    ApplyResult,
    RecoveryResult,
    RebootMode,
    InstalledPackage,
    RiskTier,
    ScreenshotResult,
    Safety,
    StorageUsage,
  } from "$lib/types";

  import { deviceTypeLabel } from "$lib/types";

  import { formatBytes, basename } from "$lib/format";

  import { recommendation as sharedRecommendation, type Recommendation } from "$lib/recommendation";

  import RamBadge from "$lib/components/RamBadge.svelte";

  import UsageBadge from "$lib/components/UsageBadge.svelte";

  import StateBadge from "$lib/components/StateBadge.svelte";

  import RiskBadge from "$lib/components/RiskBadge.svelte";

  import AppRow from "$lib/components/DesktopAppRow.svelte";

  import { idleMeasurements, loadingMeasurements, settled, type AppDetailInputs } from "$lib/app-details";

  import { reportFamily } from "$lib/app-report";

  import MediaTab from "$lib/components/MediaTab.svelte";

  import LogsTab from "$lib/components/LogsTab.svelte";

  import { openUrl } from "@tauri-apps/plugin-opener";

  import { getKeptPackages, setPackageKept, getShellAcknowledged, setShellAcknowledged, idKey } from "$lib/prefs";

  import ShellTab from "$lib/components/ShellTab.svelte";

  import AdvancedTweaksTab from "$lib/components/AdvancedTweaksTab.svelte";

  import FilesTab from "$lib/components/FilesTab.svelte";

  import TweaksTab from "$lib/components/TweaksTab.svelte";

  import SideloadTab from "$lib/components/SideloadTab.svelte";

  import RemoteTab from "$lib/components/RemoteTab.svelte";

  import OptimizeTab from "$lib/components/OptimizeTab.svelte";

  import DisplayTab from "$lib/components/DisplayTab.svelte";

  import AudioTab from "$lib/components/AudioTab.svelte";

  import SystemTab from "$lib/components/SystemTab.svelte";

  import ConsoleTab from "$lib/components/ConsoleTab.svelte";


  import { confirmAction } from "$lib/confirm";
  let { serial, device, onChanged }: { serial: string; device: Device; onChanged?: () => void } = $props();
  let report = $state<HealthReport | null>(null);

  let usage = $state<StorageUsage | null>(null);

  let safetyMap = $state<Record<string, Safety>>({});

  let safetyOverrides = $state<Record<string, boolean>>({});

  let overrideBusy = $state<string | null>(null);


  let apps = $state<AppEntry[]>([]);

  let appsLoaded = $state(false);

  let appsLoading = $state(false);

  let appsErr = $state<string | null>(null);
  /// package → 'enabled' | 'disabled' | 'missing' — refreshed alongside the app list.

  let appStates = $state<Record<string, "enabled" | "disabled" | "missing">>({});

  let appSearch = $state("");

  let expandedApp = $state<string | null>(null);
  // Default on: the catalog lists ~70 known apps, most not present on any given
  // device, so an unfiltered list is mostly un-actionable "Missing" rows. Start
  // focused on what's installed; unticking reveals the full catalog.

  let hideNotInstalled = $state(true);
  /// Which kinds of app the single table shows. Every row is exactly one kind:
  /// curated (in the catalog) → user-installed → system. The default pair is
  /// what's actionable — the curated bloat list plus whatever you sideloaded —
  /// with the several-hundred-package system tail behind its own chip.

  let kindFilter = $state<Record<AppKind, boolean>>({
    curated: true,
    user: true,
    system: false,
  });
  /// Only rows the Optimize wizard would act on (or flag for review).

  let onlyRecommended = $state(false);
  /// Every package installed on the device, catalog and not. Loaded lazily on
  /// the Apps tab.

  let installedPackages = $state<InstalledPackage[]>([]);

  let othersLoading = $state(false);
  /// package → resident RAM (MB) for apps running right now. Lazy-loaded after
  /// the list paints; most apps aren't here (not running), so a value means the
  /// app is actively holding RAM — the cue for "disable this unused app".

  let appMemory = $state<Record<string, number>>({});
  /// package → last-used / launch count, lazy-loaded alongside RAM. Powers the
  /// "remove if unused" signal (never opened / months idle).

  let appUsage = $state<Record<string, import("$lib/types").AppUsage>>({});
  /// Device uptime in seconds. `dumpsys usagestats` is rebuilt at boot, so this
  /// bounds how far back `appUsage` can see — without it a rebooted device
  /// reports every app as unused.

  let usageWindowSecs = $state<number | null>(null);

  let appStorage = $state<Record<string, import("$lib/types").AppStorage>>({});

  let appMeasures = $state(idleMeasurements());

  let keptPackages = $state<Set<string>>(new Set());

  let appMutationInFlight = $state(false);

  let inventoryGeneration = 0;

  let measuresRequest = 0;

  const hardwareId = $derived(device?.properties?.serial_number?.trim().toLowerCase() === "unknown" ? null : idKey(device?.properties?.serial_number));
  $effect(() => { keptPackages = hardwareId ? getKeptPackages(hardwareId) : new Set(); });

  function appDetails(pkg: string): AppDetailInputs {
    return { serial, measures: appMeasures, memoryMb: appMemory[pkg], usage: appUsage[pkg], storage: appStorage[pkg], liveState: appMutationInFlight ? null : appStates[pkg] ?? null, onRemeasure: loadAppMemory, report: device ? { family: reportFamily(device.device_type, device.tv_evidence), androidVersion: device.properties?.android_release ?? null, redact: [serial, device.properties?.serial_number ?? ""] } : undefined };
  }


  function matchesSearch(name: string, pkg: string): boolean {
    const q = appSearch.trim().toLowerCase();
    if (!q) return true;
    return name.toLowerCase().includes(q) || pkg.toLowerCase().includes(q);
  }

  /// One row of the App List, whatever the package's provenance. Curated rows
  /// carry their `AppEntry` so the Recommended column and the risk hovercard
  /// keep working; the long tail leaves it null.

  type AppKind = "curated" | "user" | "system";

  type AppListRow = {
    package: string;
    name: string;
    kind: AppKind;
    state: "enabled" | "disabled" | "missing";
    entry: AppEntry | null;
    risk?: RiskTier;
    description?: string;
    review: boolean;
    playStore: boolean;
  };

  /// Every catalog app (installed or not) plus every non-catalog installed
  /// package, as one list. Catalog entries come first so the actionable rows
  /// stay at the top; the tail keeps the backend's user-before-system order.

  let allRows = $derived.by((): AppListRow[] => {
    const catalogRows: AppListRow[] = apps.map((a) => ({
      package: a.package,
      name: a.name,
      kind: "curated",
      state: appStates[a.package] ?? "enabled",
      entry: a,
      risk: a.risk,
      description: a.optimize_description,
      review: a.review ?? false,
      playStore: a.play_store,
    }));
    const tailRows: AppListRow[] = installedPackages
      .filter((p) => !p.catalog)
      .map((p) => ({
        package: p.package,
        name: p.name ?? p.package,
        kind: p.system ? "system" : "user",
        state: p.enabled ? "enabled" : "disabled",
        entry: null,
        // The long tail has no curated rating; the risk pill falls back to the
        // safety map, which is what drives its color and hovercard anyway.
        risk: undefined,
        review: false,
        playStore: false,
      }));
    return [...catalogRows, ...tailRows];
  });


  let visibleRows = $derived(
    allRows.filter((r) => {
      if (!kindFilter[r.kind]) return false;
      if (hideNotInstalled && r.state === "missing") return false;
      if (onlyRecommended) {
        const rec = rowRecommendation(r);
        if (rec.kind !== "act" && rec.kind !== "review" && rec.kind !== "restore") return false;
      }
      return matchesSearch(r.name, r.package);
    }),
  );

  /// Row counts per kind, for the filter chips — always over the unfiltered set
  /// so a chip tells you what turning it on would reveal.

  let kindCounts = $derived({
    curated: allRows.filter((r) => r.kind === "curated").length,
    user: allRows.filter((r) => r.kind === "user").length,
    system: allRows.filter((r) => r.kind === "system").length,
  });

  let appActionBusy = $state<string | null>(null);

  let appActionMessage = $state("");
  /// Package whose per-row Actions dropdown is open (one at a time).

  let appMenuOpen = $state<string | null>(null);
  /// Package the "Copy to another device" panel is open for, plus targets.

  let clonePkg = $state<string | null>(null);

  let cloneTargets = $state<Device[]>([]);

  let cloneBusy = $state(false);


  let optimizeResetToken = $state(0);


  async function loadSafetyOverrides() {
    try {
      const list = await api.listSafetyOverrides(serial);
      const map: Record<string, boolean> = {};
      for (const p of list) map[p] = true;
      safetyOverrides = map;
    } catch {
      // Non-fatal; the memory table falls back to the raw classification.
    }
  }

  /// Toggle a package's manual "safe" declaration from its risk hovercard. The
  /// backend returns the device's full updated list (and rejects NEVER_DISABLE
  /// packages), so we just replace our local copy from its response.

  async function toggleSafetyOverride(pkg: string) {
    const currentlySafe = !!safetyOverrides[pkg];
    overrideBusy = pkg;
    try {
      const list = await api.setSafetyOverride(serial, pkg, !currentlySafe);
      const map: Record<string, boolean> = {};
      for (const p of list) map[p] = true;
      safetyOverrides = map;
    } catch (e) {
      appActionMessage = String(e);
    } finally {
      overrideBusy = null;
    }
  }

  /// Batch-resolve engine safety classifications into `safetyMap` for any
  /// packages not already cached. Pure const-list lookups on the backend —
  /// cheap enough to fire for whole tables.

  async function cacheSafety(pkgs: string[]) {
    const missing = pkgs.filter((p) => !safetyMap[p]);
    if (missing.length === 0) return;
    const results = await Promise.all(
      missing.map((p) => api.safetyInfo(p).catch(() => ({ kind: "safe" }) as Safety)),
    );
    const next = { ...safetyMap };
    results.forEach((s, i) => (next[missing[i]] = s));
    safetyMap = next;
  }


  let now = $state(Date.now());

  async function loadApps() {
    if (!device) return;
    const generation = inventoryGeneration;
    appsLoading = true;
    appsErr = null;
    try {
      // Pull the app list AND fresh enabled/disabled state for those packages.
      // The package state lets us render per-row Disable/Enable/Uninstall
      // buttons in their correct state.
      const list = await api.appListForDevice(device.device_type);
      apps = list;
      const states = await fetchAppStates(list.map((a) => a.package));
      if (generation !== inventoryGeneration) return;
      appStates = states;
      // Fire-and-forget: risk badges fall back to the catalog tier until the
      // engine classifications land.
      cacheSafety(list.map((a) => a.package));
    } catch (e) {
      appsErr = String(e);
    } finally {
      appsLoading = false;
      appsLoaded = true;
    }
    loadInstalledPackages();
    loadAppMemory();
  }

  /// Lazy RAM + last-used annotations: one `dumpsys meminfo` and one
  /// `dumpsys usagestats`, mapped onto the rows. Run after the list paints and
  /// never block it — a failure just leaves those cues off.

  async function loadAppMemory() {
    const request = ++measuresRequest;
    appMeasures = loadingMeasurements();
    const [mem, usage, storage] = await Promise.allSettled([
      api.appMemoryMap(serial),
      api.appUsageMap(serial),
      api.appStorageMap(serial),
    ]);
    if (request !== measuresRequest) return;
    appMemory = mem.status === "fulfilled" ? mem.value : {};
    appUsage = usage.status === "fulfilled" ? usage.value.entries : {};
    usageWindowSecs = usage.status === "fulfilled" ? usage.value.window_secs : null;
    appStorage = storage.status === "fulfilled" ? storage.value : {};
    const at = Date.now();
    appMeasures = { memory: settled(mem, at), usage: settled(usage, at), storage: settled(storage, at) };
  }

  // Every package on the device — the curated catalog's own rows come from
  // `loadApps`, this fills in the long tail plus the system/user flag. Loaded
  // after the catalog so the actionable list paints first; failures here don't
  // block the main list.

  async function loadInstalledPackages() {
    const generation = inventoryGeneration;
    othersLoading = true;
    try {
      const next = await api.listInstalledPackages(serial);
      if (generation !== inventoryGeneration) return;
      installedPackages = next;
      appStates = { ...appStates, ...Object.fromEntries(next.map((p) => [p.package, p.enabled ? "enabled" as const : "disabled" as const])) };
      cacheSafety(installedPackages.map((o) => o.package));
    } catch (e) {
      appActionMessage = `Could not list installed packages: ${e}`;
    } finally {
      othersLoading = false;
    }
  }


  function patchOtherState(pkg: string, enabled: boolean | "removed") {
    if (enabled === "removed") {
      installedPackages = installedPackages.filter((o) => o.package !== pkg);
    } else {
      installedPackages = installedPackages.map((o) => (o.package === pkg ? { ...o, enabled } : o));
    }
  }


  async function clearCacheFor(pkg: string) {
    appActionBusy = pkg;
    appActionMessage = "";
    try {
      const r = await api.clearAppCache(serial, pkg);
      appActionMessage = `${pkg}: ${r.message.trim() || (r.ok ? "cache cleared" : "failed")}`;
      if (r.ok) invalidateDeviceCaches();
    } catch (e) {
      appActionMessage = `${pkg}: ${e}`;
    } finally {
      appActionBusy = null;
    }
  }


  async function clearDataFor(pkg: string) {
    if (
      !await confirmAction(
        `Clear all data for ${pkg}?\n\nThis wipes its accounts, settings, and downloads — the app resets to a fresh install. Not reversible.`,
      )
    )
      return;
    appActionBusy = pkg;
    appActionMessage = "";
    try {
      const r = await api.clearAppData(serial, pkg);
      appActionMessage = `${pkg}: ${r.message.trim() || (r.ok ? "data cleared" : "failed")}`;
      if (r.ok) invalidateDeviceCaches();
    } catch (e) {
      appActionMessage = `${pkg}: ${e}`;
    } finally {
      appActionBusy = null;
    }
  }


  // Real state per package — one batched backend call (pm list packages +
  // pm list packages -d in parallel) so we can show Enabled/Disabled/Missing.

  async function fetchAppStates(packages: string[]): Promise<Record<string, "enabled" | "disabled" | "missing">> {
    if (packages.length === 0) return {};
    return await api.packageStates(serial, packages);
  }

  /// Re-sync the App List's cached states after the Optimize wizard runs —
  /// it cached states before the run, same as executeOptimize used to do inline.

  function setCatalogState(pkg: string, state: "enabled" | "disabled" | "missing") {
    appStates[pkg] = state;
    optimizeResetToken++;
  }

  /// Reflect a completed action in both caches the App List reads from: the
  /// catalog's state map and the installed-package list. One table means one
  /// patch — a package can be in either or both, and the row must agree with
  /// itself whichever kind it is.

  function patchRowState(pkg: string, state: "enabled" | "disabled" | "missing") {
    setCatalogState(pkg, state);
    patchOtherState(pkg, state === "missing" ? "removed" : state === "enabled");
  }


  async function disableApp(pkg: string) {
    appActionBusy = pkg;
    appActionMessage = "";
    try {
      const r = await api.disablePackage(serial, pkg);
      appActionMessage = `${pkg}: ${r.message.trim() || (r.ok ? "disabled" : "failed")}`;
      if (r.ok) {
        patchRowState(pkg, "disabled");
        invalidateDeviceCaches();
      }
    } catch (e) {
      appActionMessage = `${pkg}: ${e}`;
    } finally {
      appActionBusy = null;
    }
  }


  async function enableApp(pkg: string) {
    appActionBusy = pkg;
    appActionMessage = "";
    try {
      const r = await api.enablePackage(serial, pkg);
      appActionMessage = `${pkg}: ${r.message.trim() || (r.ok ? "enabled" : "failed")}`;
      if (r.ok) {
        patchRowState(pkg, "enabled");
        invalidateDeviceCaches();
      }
    } catch (e) {
      appActionMessage = `${pkg}: ${e}`;
    } finally {
      appActionBusy = null;
    }
  }


  async function uninstallApp(pkg: string) {
    if (!await confirmAction(`Uninstall ${pkg}? This is semi-reversible (Play Store reinstall or pm install-existing).`)) return;
    appActionBusy = pkg;
    appActionMessage = "";
    try {
      const r = await api.uninstallPackage(serial, pkg);
      appActionMessage = `${pkg}: ${r.message.trim()}`;
      if (r.ok) {
        patchRowState(pkg, "missing");
        invalidateDeviceCaches();
      }
    } catch (e) {
      appActionMessage = `${pkg}: ${e}`;
    } finally {
      appActionBusy = null;
    }
  }

  // Best-effort system-app re-install via `cmd package install-existing` —
  // works only for apps still present on /system. For third-party uninstalls
  // we route through the Play Store instead.

  async function reinstallApp(pkg: string) {
    appActionBusy = pkg;
    appActionMessage = "";
    try {
      const r = await api.reinstallExisting(serial, pkg);
      appActionMessage = `${pkg}: ${r.message.trim()}`;
      if (r.ok) patchRowState(pkg, "enabled");
    } catch (e) {
      appActionMessage = `${pkg}: ${e}`;
    } finally {
      appActionBusy = null;
    }
  }


  function recommendation(a: AppEntry, state: "enabled" | "disabled" | "missing"): Recommendation {
    const verdict = safetyMap[a.package];
    return sharedRecommendation(a, state, verdict ? { status: "ready", verdict } : { status: "checking" });
  }


  function rowRecommendation(r: AppListRow): Recommendation {
    if (keptPackages.has(r.package)) return { kind: "keep", label: "Kept" };
    return r.entry ? recommendation(r.entry, r.state) : { kind: "keep", label: "No change needed" };
  }


  function applyRecommendation(pkg: string, action: "disable" | "uninstall") {
    if (action === "disable") return disableApp(pkg);
    return uninstallApp(pkg);
  }


  async function backupApkFor(pkg: string) {
    const folder = await openDialog({ directory: true, title: "Choose a folder for the APK backup" });
    if (!folder) return;
    appActionBusy = pkg;
    appActionMessage = "";
    try {
      const r = await api.backupApk(serial, pkg, folder as string);
      appActionMessage = r.message;
    } catch (e) {
      appActionMessage = `${pkg}: ${e}`;
    } finally {
      appActionBusy = null;
    }
  }


  async function startClone(pkg: string) {
    appActionMessage = "";
    try {
      const all = await api.listDevices();
      cloneTargets = all.filter((d) => d.status === "device" && d.serial !== serial);
    } catch (e) {
      appActionMessage = String(e);
      return;
    }
    if (cloneTargets.length === 0) {
      appActionMessage = "No other connected device to copy to — connect the target device first.";
      return;
    }
    clonePkg = pkg;
  }


  async function cloneTo(target: Device) {
    if (!clonePkg) return;
    const pkg = clonePkg;
    if (
      !await confirmAction(
        `Copy ${pkg} to ${target.name} (${target.serial})?\n\nApp data does not transfer, and DRM/licensed apps may refuse to run. Paid apps should be installed via the Play Store instead.`,
      )
    )
      return;
    cloneBusy = true;
    appActionBusy = pkg;
    appActionMessage = `Copying ${pkg} to ${target.serial}… (this pulls the APK and can take a minute)`;
    try {
      const r = await api.cloneApp(serial, target.serial, pkg);
      appActionMessage = r.hint ? `${r.message}\n→ ${r.hint}` : r.message;
      if (r.ok) clonePkg = null;
    } catch (e) {
      appActionMessage = String(e);
    } finally {
      cloneBusy = false;
      appActionBusy = null;
    }
  }


  async function openInPlayStore(pkg: string) {
    appActionBusy = pkg;
    appActionMessage = "";
    try {
      const r = await api.openPlayStore(serial, pkg);
      appActionMessage = r.ok
        ? `Opened Play Store on device for ${pkg} — confirm install on the TV.`
        : `${pkg}: ${r.message.trim()}`;
    } catch (e) {
      appActionMessage = `${pkg}: ${e}`;
    } finally {
      appActionBusy = null;
    }
  }


  function invalidateDeviceCaches() { onChanged?.(); void loadAppMemory(); }
  onMount(() => { void loadApps(); void loadSafetyOverrides(); });
  onDestroy(() => { inventoryGeneration++; measuresRequest++; });
</script>

    <div class="card" role="tabpanel" tabindex={0} id="tabpanel-apps" aria-labelledby="tab-apps">
      <div class="card-header">
        <h2>App List for {deviceTypeLabel(device.device_type)}</h2>
        <div class="header-actions">
          <span class="muted">
            {visibleRows.length} shown{othersLoading ? " · scanning device…" : ` of ${allRows.length}`}
          </span>
          <button onclick={loadApps} disabled={appsLoading}>
            {appsLoading ? "Loading…" : "Refresh"}
          </button>
        </div>
      </div>
      <div class="app-toolbar">
        <input
          class="app-search"
          placeholder="Search apps by name or package…"
          bind:value={appSearch}
        />
        <div class="kind-chips" role="group" aria-label="Filter by app kind">
          <button
            class="chip"
            class:on={kindFilter.curated}
            aria-pressed={kindFilter.curated}
            onclick={() => (kindFilter.curated = !kindFilter.curated)}
            title="Apps in Shield Optimizer's curated list for this device — the ones with a recommendation"
          >Curated <span class="chip-count">{kindCounts.curated}</span></button>
          <button
            class="chip"
            class:on={kindFilter.user}
            aria-pressed={kindFilter.user}
            onclick={() => (kindFilter.user = !kindFilter.user)}
            title="Apps you installed yourself — sideloads and Play Store installs"
          >User <span class="chip-count">{kindCounts.user}</span></button>
          <button
            class="chip"
            class:on={kindFilter.system}
            aria-pressed={kindFilter.system}
            onclick={() => (kindFilter.system = !kindFilter.system)}
            title="Preinstalled system packages — disable these only if you know what they are"
          >System <span class="chip-count">{kindCounts.system}</span></button>
        </div>
        <label class="inline-check">
          <input type="checkbox" bind:checked={hideNotInstalled} />
          Hide not installed
        </label>
        <label class="inline-check">
          <input type="checkbox" bind:checked={onlyRecommended} />
          Needs action
        </label>
      </div>
      {#if appsErr}
        <div class="error">{appsErr}</div>
      {:else if appsLoading && apps.length === 0}
        <div class="muted">Loading…</div>
      {:else}
        <p class="muted small legend">
          One row per package on this device — <strong>Curated</strong> is the list we
          maintain for this hardware, <strong>User</strong> what you installed, and
          <strong>System</strong> the preinstalled tail (hidden by default).
          <strong>State</strong> is what the device reports right now, and
          <strong>Last used</strong> when it was last in the foreground — highlighted
          once it's stale, which is the cue for removing it.
          <strong>Risk</strong> explains itself on hover — including how the rating was
          decided, and a button to mark an app safe on this device.
          <strong>Recommended</strong> is what v1's Optimize wizard would pick for you —
          ✅ means nothing to do; a green or red button applies the suggested change.
          <strong>Tools</strong> holds each row's actions.
        </p>
        {#if appActionMessage}
          <p class="muted small mono action-message">
            {appActionMessage}
            <button class="dismiss" onclick={() => (appActionMessage = "")} title="Dismiss">✕</button>
          </p>
        {/if}
        {#if clonePkg}
          <div class="clone-panel">
            <span>Copy <code>{clonePkg}</code> to:</span>
            {#each cloneTargets as t (t.serial)}
              <button class="small-action" onclick={() => cloneTo(t)} disabled={cloneBusy}>
                {cloneBusy ? "Copying…" : `${t.name} (${t.serial})`}
              </button>
            {/each}
            <button class="small-action subtle" onclick={() => (clonePkg = null)} disabled={cloneBusy}>
              Cancel
            </button>
          </div>
        {/if}
        <table class="app-table">
          <thead>
            <tr>
              <th>App</th>
              <th class="center">State</th>
              <th class="center">Last used</th>
              <th class="center">Risk</th>
              <th class="center">Recommended</th>
              <th class="center">Manage</th>
            </tr>
          </thead>
          <tbody>
            {#each visibleRows as r (r.package)}
              {@const rec = rowRecommendation(r)}
              <AppRow
                name={r.name}
                description={r.description}
                package={r.package}
                review={r.review}
                state={r.state}
                kind={r.kind}
                mb={appMemory[r.package]}
                usage={appUsage[r.package]}
                usageWindowSecs={usageWindowSecs}
                showUsage={r.state !== "missing"}
                usageColumn
                risk={r.risk}
                safety={safetyMap[r.package]}
                overridden={!!safetyOverrides[r.package]}
                riskBusy={overrideBusy === r.package}
                onToggleSafe={toggleSafetyOverride}
                details={appDetails(r.package)}
                detailOpen={expandedApp === r.package}
                onToggleDetail={() => expandedApp = expandedApp === r.package ? null : r.package}
              >
                {#snippet actions()}
                <td class="rec-cell center">
                  {#if keptPackages.has(r.package)}
                    <span data-rec="Kept">Kept</span>
                    <button class="small-action subtle" onclick={() => keptPackages = setPackageKept(hardwareId, r.package, false)}>Change</button>
                  {:else}
                  {#if rec.kind === "act"}
                    <button
                      data-rec={rec.label}
                      class="small-action recommended danger"
                      onclick={() => applyRecommendation(r.package, rec.action)}
                      disabled={appActionBusy === r.package}
                      title={r.description}
                    >
                      {appActionBusy === r.package ? "…" : rec.label}
                    </button>
                  {:else if rec.kind === "review"}
                    <!-- Optional, not a default: an icon keeps it quieter than
                         the recommended actions. The glyph tracks the action —
                         a trash can on a reversible disable would overstate
                         what the button does. -->
                    <button
                      data-rec={rec.label}
                      class="small-action review-action icon-action"
                      class:danger={rec.action === "uninstall"}
                      onclick={() => applyRecommendation(r.package, rec.action)}
                      disabled={appActionBusy === r.package}
                      title={rec.action === "uninstall"
                        ? "Uninstall — check Last used before uninstalling."
                        : "Disable — you may not use this one. Check Last used, then disable it if that's right."}
                      aria-label={rec.label}
                    >
                      {appActionBusy === r.package ? "…" : rec.action === "uninstall" ? "🗑" : "⊘"}
                    </button>
                  {:else if rec.kind === "restore"}
                    <button
                      class="small-action rec-enable"
                      onclick={() => reinstallApp(r.package)}
                      disabled={appActionBusy === r.package}
                      title="cmd package install-existing — works for system apps still on /system"
                    >
                      {appActionBusy === r.package ? "…" : rec.label}
                    </button>
                  {:else if r.kind === "curated"}
                    <span
                      class="done-check"
                      role="img"
                      aria-label={rec.kind === "done" ? rec.label : "Keep — nothing to do"}
                      title={rec.kind === "done" ? rec.label : "Keep — nothing to do"}
                    >✅</span>
                  {:else}
                    <!-- Not in the curated list, so there is no recommendation to
                         make — an empty cell says that better than a ✅ would. -->
                    <span class="muted small">—</span>
                  {/if}
                  {#if hardwareId}<button class="small-action subtle" onclick={() => keptPackages = setPackageKept(hardwareId, r.package, true)}>Keep</button>{/if}
                  {/if}
                </td>
                <td class="center tools-cell">
                  {#if r.state !== "missing" || r.playStore}
                    <div class="menu-wrap">
                      <button
                        class="small-action subtle"
                        onclick={(e) => {
                          e.stopPropagation();
                          appMenuOpen = appMenuOpen === r.package ? null : r.package;
                        }}
                        disabled={appActionBusy === r.package}
                        aria-haspopup="menu"
                        aria-expanded={appMenuOpen === r.package}
                        title="Uninstall, disable, clear data, back up {r.name}"
                      >
                        {appActionBusy === r.package ? "…" : "Manage ▾"}
                      </button>
                      {#if appMenuOpen === r.package}
                        <!-- One menu for every row now. What's offered follows
                             the package's real state, not which list it came
                             from; the backend still refuses protected packages. -->
                        <div class="actions-menu" role="menu">
                          <button role="menuitem" onclick={() => { keptPackages = setPackageKept(hardwareId, r.package, !keptPackages.has(r.package)); appMenuOpen = null; }} disabled={!hardwareId}>{keptPackages.has(r.package) ? "Clear keep choice" : "Keep this app"}</button>
                          {#if r.state === "enabled"}
                            <button
                              role="menuitem"
                              onclick={() => { appMenuOpen = null; disableApp(r.package); }}
                              title="pm disable-user --user 0"
                            >Disable</button>
                          {:else if r.state === "disabled"}
                            <button
                              role="menuitem"
                              onclick={() => { appMenuOpen = null; enableApp(r.package); }}
                              title="pm enable"
                            >Enable</button>
                          {/if}
                          {#if r.state !== "missing"}
                            <button
                              role="menuitem"
                              onclick={() => { appMenuOpen = null; backupApkFor(r.package); }}
                              title="Save this app's APK(s) to a folder on this computer"
                            >Backup</button>
                            <button
                              role="menuitem"
                              onclick={() => { appMenuOpen = null; startClone(r.package); }}
                              title="Install this app onto another connected device (app data does not transfer)"
                            >Copy to…</button>
                            <button
                              role="menuitem"
                              onclick={() => { appMenuOpen = null; clearCacheFor(r.package); }}
                              title="pm clear-cache — drops cached files; safe, rebuilds on next launch"
                            >Clear cache</button>
                            <button
                              role="menuitem"
                              class="danger"
                              onclick={() => { appMenuOpen = null; clearDataFor(r.package); }}
                              title="pm clear — wipes accounts, settings, downloads; resets to fresh install (not reversible)"
                            >Clear data</button>
                            <button
                              role="menuitem"
                              class="danger"
                              onclick={() => { appMenuOpen = null; uninstallApp(r.package); }}
                              title="pm uninstall --user 0"
                            >Uninstall</button>
                          {/if}
                          {#if r.playStore}
                            <button
                              role="menuitem"
                              onclick={() => { appMenuOpen = null; openInPlayStore(r.package); }}
                              title="Open {r.name} on the Play Store on the device"
                            >Play Store</button>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {:else}
                    <span class="muted small">—</span>
                  {/if}
                </td>
                {/snippet}
              </AppRow>
            {/each}
            {#if visibleRows.length === 0}
              <tr>
                <td colspan="6" class="muted">
                  {othersLoading ? "Scanning the device…" : "No apps match your filters."}
                </td>
              </tr>
            {/if}
          </tbody>
        </table>
      {/if}
    </div>

<style>
  .card-header, .header-actions, .app-toolbar, .kind-chips { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .card-header { justify-content: space-between; }
  .app-toolbar { margin: 1rem 0; }
  .app-search { flex: 1; min-width: 12rem; }
  .chip.on { background: var(--accent-strong); color: var(--accent-ink); }
  .chip-count { opacity: 0.7; }
  table { width: 100%; border-collapse: collapse; }
  th { text-align: left; padding: 0.65rem; }
</style>
