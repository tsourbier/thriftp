<script lang="ts">
  import { untrack } from "svelte";
  import {
    navigationTarget,
    overflowSegments,
    segmentsForPath
  } from "../../lib/browser/breadcrumbs";

  let {
    path,
    hasLocation = true,
    onnavigate
  }: {
    path: string;
    hasLocation?: boolean;
    onnavigate?: (path: string) => void;
  } = $props();

  let editing = $state(false);
  // eslint-disable-next-line svelte/prefer-writable-derived
  let draft = $state(untrack(() => path));
  let input: HTMLInputElement | null = $state(null);

  const segments = $derived(segmentsForPath(path));
  const shown = $derived(overflowSegments(segments, 5));

  $effect(() => {
    draft = path;
  });

  $effect(() => {
    if (editing) {
      input?.focus();
      input?.select();
    }
  });

  function startEditing() {
    draft = path;
    editing = true;
  }

  function submit() {
    const target = draft.trim();
    if (target) {
      onnavigate?.(target);
    }
    editing = false;
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      submit();
    } else if (event.key === "Escape") {
      draft = path;
      editing = false;
    }
  }

  function globalKeydown(event: KeyboardEvent) {
    if (
      hasLocation &&
      (event.metaKey || event.ctrlKey) &&
      event.key.toLowerCase() === "l"
    ) {
      event.preventDefault();
      startEditing();
    }
  }
</script>

<svelte:window onkeydown={globalKeydown} />

{#if hasLocation}
  <nav class="breadcrumbs" aria-label="Current path">
    {#if editing}
      <input
        bind:this={input}
        bind:value={draft}
        onkeydown={keydown}
        onblur={() => (editing = false)}
        aria-label="Edit current path"
        autocomplete="off"
        spellcheck="false"
      />
      <button type="button" onclick={submit}>Go</button>
      <button type="button" onclick={() => (editing = false)}>Cancel</button>
    {:else}
      {#each shown.visible as segment, index (segment.path)}
        {#if index === 1 && shown.overflow.length > 0}
          <details class="overflow">
            <summary aria-label="Show hidden path segments">…</summary>
            <div class="overflow-menu">
              {#each shown.overflow as hidden (hidden.path)}
                <button
                  type="button"
                  onclick={() =>
                    onnavigate?.(
                      navigationTarget(segments, segments.indexOf(hidden)) ?? hidden.path
                    )}
                >
                  {hidden.label}
                </button>
              {/each}
            </div>
          </details>
        {/if}
        <button
          type="button"
          class="segment"
          class:current={segment.isCurrent}
          aria-current={segment.isCurrent ? "location" : undefined}
          aria-label={segment.fullPath}
          title={segment.fullPath}
          onclick={() =>
            segment.isCurrent ? startEditing() : onnavigate?.(segment.path)}
        >
          {segment.label}
        </button>
      {/each}
    {/if}
  </nav>
{/if}

<style>
  .breadcrumbs {
    display: flex;
    align-items: center;
    gap: 0.15rem;
    min-width: 0;
    flex: 1;
    overflow: hidden;
    font-size: 0.8rem;
  }
  .segment {
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0.15rem 0.3rem;
    font: inherit;
    background: transparent;
    border: 0;
    border-radius: var(--radius-sm, 0.3rem);
    cursor: pointer;
  }
  .segment:hover,
  .segment.current {
    background: var(--surface-muted);
  }
  input {
    flex: 1;
    min-width: 8rem;
    padding: 0.2rem 0.3rem;
    font: inherit;
  }
  .overflow {
    position: relative;
  }
  summary {
    cursor: pointer;
    padding: 0.15rem 0.3rem;
  }
  .overflow-menu {
    position: absolute;
    z-index: 10;
    top: 1.5rem;
    left: 0;
    display: grid;
    min-width: 10rem;
    padding: 0.25rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 0.3rem);
    box-shadow: var(--shadow-lg, 0 8px 24px rgb(0 0 0 / 0.2));
  }
  .overflow-menu button {
    text-align: left;
    font: inherit;
    font-size: 0.78rem;
    padding: 0.2rem 0.3rem;
  }
</style>
