<script lang="ts">
  import { untrack } from "svelte";
  import type { ConflictPolicy } from "../../lib/ipc/types";

  let {
    destination,
    onexisting,
    oncancel
  }: {
    destination: string;
    onexisting: (policy: ConflictPolicy, applyToAll: boolean, renamed?: string) => void;
    oncancel: () => void;
  } = $props();

  let applyToAll = $state(false);
  let rename = $state(untrack(() => renameCandidate(destination)));

  function renameCandidate(path: string) {
    const slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    const directory = slash >= 0 ? path.slice(0, slash + 1) : "";
    const name = slash >= 0 ? path.slice(slash + 1) : path;
    const dot = name.lastIndexOf(".");
    return dot > 0
      ? `${directory}${name.slice(0, dot)} (1)${name.slice(dot)}`
      : `${directory}${name} (1)`;
  }
</script>

<div
  class="conflict"
  role="alertdialog"
  aria-modal="true"
  aria-label="File already exists"
>
  <h2>File already exists</h2>
  <p><code>{destination}</code> already exists at the destination.</p>

  <label class="rename">
    Rename to
    <input bind:value={rename} aria-label="New name" />
  </label>

  <label class="inline">
    <input type="checkbox" bind:checked={applyToAll} />
    Apply to all remaining conflicts
  </label>

  <div class="actions">
    <button type="button" onclick={oncancel}>Cancel</button>
    <button type="button" onclick={() => onexisting("skip", applyToAll)}>Skip</button>
    <button type="button" onclick={() => onexisting("rename", applyToAll, rename)}>
      Rename
    </button>
    <button type="button" onclick={() => onexisting("overwrite", applyToAll)}>
      Overwrite
    </button>
  </div>
</div>

<style>
  .conflict {
    display: grid;
    gap: 0.6rem;
    min-width: 24rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  p {
    margin: 0;
    font-size: 0.9rem;
  }
  code {
    word-break: break-all;
  }
  .rename {
    display: grid;
    gap: 0.25rem;
    font-size: 0.85rem;
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.85rem;
  }
  input {
    padding: 0.3rem;
    font: inherit;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 0.4rem;
  }
</style>
