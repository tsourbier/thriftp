<script lang="ts">
  import { onMount } from "svelte";
  import { ipc } from "../../lib/ipc/client";
  import type { BrowserSource, FileMetadata } from "../../lib/ipc/types";

  let {
    source,
    connectionId = null,
    path,
    onclose,
    onchanged
  }: {
    source: BrowserSource;
    connectionId?: string | null;
    path: string;
    onclose: () => void;
    onchanged?: () => void;
  } = $props();

  let metadata = $state<FileMetadata | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let mode = $state("");

  onMount(refresh);

  async function refresh() {
    loading = true;
    try {
      metadata = await ipc.inspectorGet(source, path, connectionId ?? undefined);
      mode = metadata.entry.permissions ?? "";
      error = null;
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      loading = false;
    }
  }

  async function applyMode(event: SubmitEvent) {
    event.preventDefault();
    if (!connectionId) {
      return;
    }
    const parsed = Number.parseInt(mode, 8);
    if (!Number.isFinite(parsed)) {
      error = "Enter an octal permission mode such as 644.";
      return;
    }
    try {
      await ipc.chmod(connectionId, path, parsed);
      await refresh();
      onchanged?.();
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }

  function sizeLabel(size: number | null) {
    return size === null ? "Unknown" : `${size} bytes`;
  }
</script>

<div class="inspector" role="dialog" aria-modal="true" aria-label="File information">
  <header>
    <h2>File information</h2>
    <button type="button" onclick={onclose}>Close</button>
  </header>

  {#if loading}
    <p role="status">Loading metadata…</p>
  {:else if error && !metadata}
    <p role="alert" class="error">{error}</p>
  {:else if metadata}
    <dl>
      <dt>Name</dt>
      <dd>{metadata.entry.name}</dd>
      <dt>Path</dt>
      <dd class="path">{metadata.entry.path}</dd>
      <dt>Type</dt>
      <dd>{metadata.entry.entryType}</dd>
      <dt>Size</dt>
      <dd>{sizeLabel(metadata.entry.size)}</dd>
      <dt>Modified</dt>
      <dd>
        {metadata.entry.modifiedAt
          ? new Date(metadata.entry.modifiedAt).toLocaleString()
          : "Unknown"}
      </dd>
      <dt>Permissions</dt>
      <dd>
        {metadata.entry.permissions ?? "Unavailable"}
        {#if metadata.permissionsSupported}
          <span class="supported">supported</span>
        {/if}
      </dd>
      <dt>Ownership</dt>
      <dd>
        {#if metadata.entry.ownership}
          {metadata.entry.ownership.owner}
          {metadata.entry.ownership.group ? `:${metadata.entry.ownership.group}` : ""}
        {:else}
          Unavailable
        {/if}
      </dd>
    </dl>

    {#if source === "remote"}
      {#if metadata.permissionsSupported}
        <form class="chmod" onsubmit={applyMode}>
          <label>
            Octal mode
            <input bind:value={mode} maxlength="4" />
          </label>
          <button type="submit">Apply</button>
        </form>
      {:else}
        <p class="notice">This server does not support changing permissions.</p>
      {/if}
    {/if}

    {#if error}
      <p role="alert" class="error">{error}</p>
    {/if}
  {/if}
</div>

<style>
  .inspector {
    display: grid;
    gap: 0.6rem;
    width: min(32rem, 90vw);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  dl {
    display: grid;
    grid-template-columns: 8rem minmax(0, 1fr);
    gap: 0.3rem 0.6rem;
    margin: 0;
    font-size: 0.85rem;
  }
  dt {
    color: GrayText;
  }
  dd {
    margin: 0;
    word-break: break-word;
  }
  .path {
    font-family: monospace;
  }
  .supported {
    font-size: 0.65rem;
    color: GrayText;
    margin-left: 0.3rem;
  }
  .chmod {
    display: flex;
    align-items: end;
    gap: 0.5rem;
  }
  .chmod label {
    display: grid;
    gap: 0.2rem;
    font-size: 0.8rem;
  }
  .chmod input {
    width: 5rem;
    font: inherit;
  }
  .notice {
    font-size: 0.8rem;
    color: GrayText;
    margin: 0;
  }
  .error {
    color: #8a0000;
    font-size: 0.8rem;
    margin: 0;
  }
</style>
