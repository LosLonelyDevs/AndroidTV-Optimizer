<script lang="ts">
  import type { DeviceType } from "$lib/types";
  import DisplayTab from "./DisplayTab.svelte";
  import AudioTab from "./AudioTab.svelte";
  import SystemTab from "./SystemTab.svelte";
  import ForkTweaksTab from "./ForkTweaksTab.svelte";
  import AdvancedTweaksTab from "./AdvancedTweaksTab.svelte";

  let { serial, deviceType, onSettingsChanged, onLauncher }: {
    serial: string;
    deviceType: DeviceType;
    onSettingsChanged?: () => void;
    onLauncher?: () => void;
  } = $props();
  type Section = "display" | "audio" | "input" | "device" | "advanced";
  let section = $state<Section>("display");
  let visited = $state<Partial<Record<Section, boolean>>>({ display: true });
  $effect(() => { visited[section] = true; });
</script>

<div class="settings-sections" role="group" aria-label="Settings sections">
  {#each ["display", "audio", "input", "device", "advanced"] as name}
    <button id={`subtab-${name}`} class:active={section === name} aria-pressed={section === name} onclick={() => section = name as Section}>
      {name[0].toUpperCase() + name.slice(1)}
    </button>
  {/each}
</div>
{#if visited.display}
  <div hidden={section !== "display"}>
    <DisplayTab {serial} {deviceType} />
    <ForkTweaksTab {serial} {deviceType} group="display" />
    <SystemTab {serial} group="display" />
    <button onclick={onLauncher}>Manage launchers</button>
  </div>
{/if}
{#if visited.audio}<div hidden={section !== "audio"}><AudioTab {serial} {deviceType} /></div>{/if}
{#if visited.input}<div hidden={section !== "input"}><ForkTweaksTab {serial} {deviceType} group="input" /></div>{/if}
{#if visited.device}
  <div hidden={section !== "device"}>
    <ForkTweaksTab {serial} {deviceType} group="device" />
    <SystemTab {serial} />
  </div>
{/if}
{#if visited.advanced}<div hidden={section !== "advanced"}><AdvancedTweaksTab {serial} {onSettingsChanged} /></div>{/if}

<style>
  .settings-sections { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 1rem; }
  .active { background: var(--accent-strong); color: var(--accent-ink); }
</style>
