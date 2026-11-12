<script module lang="ts">
  import type { Recommendation } from "$lib/recommendation";
  import type { SafetyStatus } from "$lib/safety";
  export type SuggestionValue =
    | { kind: "recommendation"; rec: Recommendation }
    | { kind: "verdict" | "process"; status: SafetyStatus | undefined };
</script>

<script lang="ts">
  import { safetyClass, safetyLabel } from "$lib/safety";
  let { value, kept = false }: { value: SuggestionValue; kept?: boolean } = $props();
</script>

{#if value.kind === "recommendation"}
  {@const preserve = kept && value.rec.kind !== "restore"}
  <span class={`suggestion suggestion--${preserve ? "keep" : value.rec.kind}`}>{preserve ? "Kept" : value.rec.label}</span>
{:else if value.kind === "process"}
  <span class="suggestion suggestion--keep">Not an app</span>
{:else}
  <span class={safetyClass(value.status)}>{safetyLabel(value.status)}</span>
{/if}

<style>
  .suggestion { display: inline-block; padding: 0.1rem 0.5rem; border: 1px solid var(--border); border-radius: var(--radius-pill); font-size: 0.75rem; color: var(--fg-secondary); }
  .suggestion--act { color: var(--fg-primary); border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
  .suggestion--review, .suggestion--unavailable { color: var(--warn); background: var(--warn-surface); border-color: var(--warn-border); }
  .suggestion--done, .suggestion--keep, .suggestion--restore { color: var(--fg-muted); }
</style>
