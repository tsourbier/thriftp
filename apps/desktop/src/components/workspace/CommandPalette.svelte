<script lang="ts">
  import { onMount } from "svelte";
  import {
    groupEntries,
    recentOrSuggested,
    searchEntries,
    type CommandPaletteEntry
  } from "../../lib/commands/registry";
  import { focusTrap } from "../../lib/accessibility/focus";
  import type { ConnectionHistoryEntry } from "../../lib/ipc/types";

  let {
    entries = [],
    recent = [],
    onrun,
    onclose
  }: {
    entries?: CommandPaletteEntry[];
    recent?: ConnectionHistoryEntry[];
    onrun?: (entry: CommandPaletteEntry) => void;
    onclose?: () => void;
  } = $props();

  let query = $state("");
  let highlighted = $state(0);
  let search: HTMLInputElement | null = $state(null);

  const results = $derived(
    query.trim() ? searchEntries(entries, query) : recentOrSuggested(entries, recent)
  );
  const groups = $derived(groupEntries(results));
  const flat = $derived(groups.flatMap((group) => group.entries));

  onMount(() => {
    search?.focus();
  });

  function move(delta: number) {
    if (flat.length === 0) {
      return;
    }
    highlighted = (highlighted + delta + flat.length) % flat.length;
  }

  function run(entry: CommandPaletteEntry | undefined) {
    if (!entry) {
      return;
    }
    if (!entry.enabled) {
      return;
    }
    onrun?.(entry);
  }

  function keydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        move(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        move(-1);
        break;
      case "Enter":
        event.preventDefault();
        run(flat[highlighted]);
        break;
      case "Escape":
        event.preventDefault();
        onclose?.();
        break;
    }
  }
</script>

<div
  class="palette-backdrop"
  role="presentation"
  onmousedown={(event) => {
    if (event.target === event.currentTarget) {
      onclose?.();
    }
  }}
>
  <div
    class="palette"
    role="dialog"
    aria-modal="true"
    aria-label="Command palette"
    tabindex="-1"
    use:focusTrap
    onkeydown={keydown}
  >
    <input
      bind:this={search}
      bind:value={query}
      oninput={() => (highlighted = 0)}
      placeholder="Search servers, favorites, tabs, and actions"
      aria-label="Search commands"
      autocomplete="off"
      spellcheck="false"
    />
    {#if flat.length === 0}
      <p class="empty">No matching commands. Keep typing or press Escape.</p>
    {:else}
      <div class="results">
        {#each groups as group (group.category)}
          <div class="group">
            <h3>{group.category}</h3>
            {#each group.entries as entry (entry.id)}
              {@const index = flat.indexOf(entry)}
              <button
                type="button"
                class="result"
                class:selected={index === highlighted}
                class:disabled={!entry.enabled}
                aria-disabled={!entry.enabled}
                onmouseenter={() => (highlighted = index)}
                onclick={() => run(entry)}
              >
                <span>{entry.label}</span>
                {#if entry.subtitle}<span class="subtitle">{entry.subtitle}</span>{/if}
                {#if !entry.enabled && entry.disabledReason}
                  <span class="reason">{entry.disabledReason}</span>
                {/if}
              </button>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
    <footer>↑↓ navigate · Enter run · Esc close</footer>
  </div>
</div>

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: start center;
    padding-top: 12vh;
    background: rgb(0 0 0 / 0.3);
    z-index: 40;
  }
  .palette {
    width: min(42rem, calc(100vw - 2rem));
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 0.5rem);
    box-shadow: var(--shadow-lg, 0 16px 40px rgb(0 0 0 / 0.25));
  }
  input {
    padding: 0.5rem;
    font: inherit;
  }
  .results {
    overflow: auto;
    display: grid;
    gap: 0.5rem;
  }
  .group {
    display: grid;
    gap: 0.15rem;
  }
  h3 {
    margin: 0;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  .result {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.1rem 0.5rem;
    text-align: left;
    padding: 0.4rem 0.5rem;
    font: inherit;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm, 0.3rem);
    cursor: pointer;
  }
  .result.selected {
    background: var(--surface-muted);
    border-color: var(--border);
  }
  .result.disabled {
    color: var(--text-muted);
    cursor: not-allowed;
  }
  .subtitle,
  .reason {
    font-size: 0.72rem;
    color: var(--text-muted);
  }
  .reason {
    grid-column: 1 / -1;
    color: var(--warning-text);
  }
  .empty {
    margin: 0;
    padding: 0.5rem;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  footer {
    font-size: 0.7rem;
    color: var(--text-muted);
    border-top: 1px solid var(--border);
    padding-top: 0.35rem;
  }
</style>
