<script lang="ts">
  import type { FileEntryView, SortDirection, SortKey } from "../../lib/ipc/types";

  let {
    entries,
    loading = false,
    listingState = "pending",
    selectedPath = null,
    selectedPaths = [],
    source = "local",
    sortKey = "name",
    sortDirection = "asc",
    onselect,
    onopen,
    onsort
  }: {
    entries: FileEntryView[];
    loading?: boolean;
    listingState?: "pending" | "partial" | "complete" | "cancelled" | "error";
    selectedPath?: string | null;
    selectedPaths?: string[];
    source?: "local" | "remote";
    sortKey?: SortKey;
    sortDirection?: SortDirection;
    onselect: (
      entry: FileEntryView,
      modifiers: { shift: boolean; toggle: boolean }
    ) => void;
    onopen: (entry: FileEntryView) => void;
    onsort?: (key: SortKey) => void;
  } = $props();

  function ariaSort(key: SortKey): "ascending" | "descending" | "none" {
    if (key !== sortKey) {
      return "none";
    }
    return sortDirection === "asc" ? "ascending" : "descending";
  }

  function sortIndicator(key: SortKey): string {
    if (key !== sortKey) {
      return "";
    }
    return sortDirection === "asc" ? "▲" : "▼";
  }

  const rowHeight = 28;
  let viewportHeight = $state(360);
  let scrollTop = $state(0);

  const startIndex = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - 4));
  const visibleCount = $derived(Math.ceil(viewportHeight / rowHeight) + 8);
  const visibleEntries = $derived(entries.slice(startIndex, startIndex + visibleCount));

  function measureViewport(node: HTMLElement) {
    const update = () => {
      viewportHeight = node.clientHeight;
    };
    update();
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    observer?.observe(node);
    window.addEventListener("resize", update);
    return {
      destroy() {
        observer?.disconnect();
        window.removeEventListener("resize", update);
      }
    };
  }

  function onscroll(event: Event) {
    const element = event.currentTarget as HTMLElement;
    scrollTop = element.scrollTop;
    viewportHeight = element.clientHeight;
  }

  function icon(entry: FileEntryView) {
    switch (entry.entryType) {
      case "directory":
        return "📁";
      case "symlink":
        return "🔗";
      default:
        return "📄";
    }
  }

  function sizeLabel(entry: FileEntryView) {
    if (entry.size === null) {
      return "";
    }
    const units = ["B", "KB", "MB", "GB", "TB"];
    let size = entry.size;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return unit === 0 ? `${size} ${units[unit]}` : `${size.toFixed(1)} ${units[unit]}`;
  }

  function dateLabel(entry: FileEntryView) {
    if (!entry.modifiedAt) {
      return "";
    }
    return new Date(entry.modifiedAt).toLocaleString();
  }
</script>

<div class="directory" aria-label="Directory contents">
  <div class="header" role="row">
    <div role="columnheader" aria-sort={ariaSort("name")}>
      <button type="button" onclick={() => onsort?.("name")}>
        Name <span aria-hidden="true">{sortIndicator("name")}</span>
      </button>
    </div>
    <div role="columnheader" aria-sort={ariaSort("size")}>
      <button type="button" onclick={() => onsort?.("size")}>
        Size <span aria-hidden="true">{sortIndicator("size")}</span>
      </button>
    </div>
    <div role="columnheader" aria-sort={ariaSort("date")}>
      <button type="button" onclick={() => onsort?.("date")}>
        Modified <span aria-hidden="true">{sortIndicator("date")}</span>
      </button>
    </div>
  </div>

  {#if loading && entries.length === 0}
    <p class="status" role="status">Loading…</p>
  {:else if entries.length === 0 && listingState === "complete"}
    <p class="status" role="status">This directory is empty.</p>
  {:else}
    <div
      class="viewport"
      role="listbox"
      use:measureViewport
      {onscroll}
      tabindex="0"
      aria-label="Files"
    >
      <div style="height: {entries.length * rowHeight}px; position: relative;">
        {#each visibleEntries as entry, index (entry.path)}
          {@const top = (startIndex + index) * rowHeight}
          <div
            class="row"
            class:selected={selectedPaths.includes(entry.path) ||
              entry.path === selectedPath}
            role="option"
            aria-selected={selectedPaths.includes(entry.path) ||
              entry.path === selectedPath}
            tabindex="-1"
            draggable="true"
            style="transform: translateY({top}px); height: {rowHeight}px;"
            ondragstart={(event) =>
              event.dataTransfer?.setData(
                "application/thriftp-entry",
                JSON.stringify({ source, path: entry.path, name: entry.name })
              )}
            onclick={(event) =>
              onselect(entry, {
                shift: event.shiftKey,
                toggle: event.metaKey || event.ctrlKey
              })}
            ondblclick={() => onopen(entry)}
            onkeydown={(event) => {
              if (event.key === "Enter") {
                onopen(entry);
              }
            }}
          >
            <span class="name" title={entry.path}>
              <span aria-hidden="true">{icon(entry)}</span>
              {entry.name}
            </span>
            <span class="meta">{sizeLabel(entry)}</span>
            <span class="meta">{dateLabel(entry)}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .directory {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
  }
  .header,
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 6rem 12rem;
    gap: 0.5rem;
    align-items: center;
    padding: 0 0.5rem;
  }
  .header {
    font-size: 0.75rem;
    font-weight: 600;
    color: GrayText;
    border-bottom: 1px solid #d0d0d0;
    padding-bottom: 0.25rem;
  }
  .header button {
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .header button:hover {
    color: var(--text);
  }
  .viewport {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    cursor: default;
    border-radius: 0.25rem;
  }
  .row:hover {
    background: color-mix(in srgb, Highlight 12%, transparent);
  }
  .row.selected {
    background: Highlight;
    color: HighlightText;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: 0.75rem;
    color: GrayText;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.selected .meta {
    color: inherit;
  }
  .status {
    padding: 1rem;
    color: GrayText;
    font-size: 0.85rem;
  }
</style>
