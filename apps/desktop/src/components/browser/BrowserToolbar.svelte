<script lang="ts">
  import BreadcrumbBar from "./BreadcrumbBar.svelte";
  import type { SortKey } from "../../lib/ipc/types";

  let {
    path,
    sortKey = $bindable("name"),
    showHidden = $bindable(false),
    filter = $bindable(""),
    canGoUp = true,
    loading = false,
    onnavigate,
    onup,
    onrefresh,
    onsort
  }: {
    path: string;
    sortKey?: SortKey;
    showHidden?: boolean;
    filter?: string;
    canGoUp?: boolean;
    loading?: boolean;
    onnavigate: (path: string) => void;
    onup: () => void;
    onrefresh: () => void;
    onsort: (key: SortKey) => void;
  } = $props();

  function changeSort(event: Event) {
    const key = (event.currentTarget as HTMLSelectElement).value as SortKey;
    sortKey = key;
    onsort(key);
  }
</script>

<div class="toolbar" role="toolbar" aria-label="Browser controls">
  <button
    type="button"
    onclick={onup}
    disabled={!canGoUp}
    title="Parent directory"
    aria-label="Parent directory">↑</button
  >
  <button
    type="button"
    onclick={onrefresh}
    disabled={loading}
    title="Refresh"
    aria-label="Refresh"
  >
    <span class="refresh-icon" aria-hidden="true">⟳</span>
  </button>
  <BreadcrumbBar {path} hasLocation={Boolean(path)} {onnavigate} />
  <input
    class="filter"
    bind:value={filter}
    placeholder="Filter"
    aria-label="Filter entries"
    autocomplete="off"
  />
  <label class="inline">
    Sort
    <select value={sortKey} onchange={changeSort} aria-label="Sort entries">
      <option value="name">Name</option>
      <option value="size">Size</option>
      <option value="date">Date</option>
      <option value="type">Type</option>
    </select>
  </label>
  <label class="inline">
    <input type="checkbox" bind:checked={showHidden} />
    Hidden
  </label>
</div>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    margin-bottom: 0.5rem;
  }
  .filter {
    flex: 0 1 8rem;
    padding: 0.25rem;
    font: inherit;
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8rem;
  }
  button {
    font: inherit;
  }
  .refresh-icon {
    display: inline-block;
    font-size: 1.5em;
    line-height: 1;
    vertical-align: -0.1em;
  }
</style>
