<script lang="ts">
  import type { FileEntryView } from "../../lib/ipc/types";

  let {
    entries,
    loading = false,
    listingState = "pending",
    selectedPath = null,
    selectedPaths = [],
    source = "local",
    onselect,
    onopen
  }: {
    entries: FileEntryView[];
    loading?: boolean;
    listingState?: "pending" | "partial" | "complete" | "cancelled" | "error";
    selectedPath?: string | null;
    selectedPaths?: string[];
    source?: "local" | "remote";
    onselect: (entry: FileEntryView, additive?: boolean) => void;
    onopen: (entry: FileEntryView) => void;
  } = $props();

  const rowHeight = 28;
  let viewportHeight = $state(360);
  let scrollTop = $state(0);

  const startIndex = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - 4));
  const visibleCount = $derived(Math.ceil(viewportHeight / rowHeight) + 8);
  const visibleEntries = $derived(entries.slice(startIndex, startIndex + visibleCount));

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
    <span>Name</span>
    <span>Size</span>
    <span>Modified</span>
  </div>

  {#if loading && entries.length === 0}
    <p class="status" role="status">Loading…</p>
  {:else if entries.length === 0 && listingState === "complete"}
    <p class="status" role="status">This directory is empty.</p>
  {:else}
    <div class="viewport" role="listbox" {onscroll} tabindex="0" aria-label="Files">
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
              onselect(entry, event.metaKey || event.ctrlKey || event.shiftKey)}
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
  .viewport {
    overflow: auto;
    max-height: 100%;
    min-height: 12rem;
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
