<script lang="ts">
  import { onMount } from "svelte";
  import { confirmAction } from "../../lib/dialogs/confirm";
  import { ipc } from "../../lib/ipc/client";
  import type { BrowserSource, FileMetadata, PermissionBits } from "../../lib/ipc/types";

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
  let bits = $state({
    user: { read: false, write: false, execute: false },
    group: { read: false, write: false, execute: false },
    world: { read: false, write: false, execute: false }
  });
  let copied = $state(false);
  let recursiveNotice = $state<string | null>(null);
  let recursiveFailures = $state<{ path: string; message: string }[]>([]);
  let recursiveBusy = $state(false);

  onMount(refresh);

  const octal = $derived(modeFromBits(bits).toString(8).padStart(3, "0"));
  const symbolic = $derived(symbolicFromBits(bits));
  const hasPermissionValues = $derived(
    metadata?.permissionDetail?.mode != null &&
      metadata?.permissionDetail?.symbolic != null
  );
  const editable = $derived(
    source === "remote" &&
      Boolean(metadata?.permissionsSupported) &&
      Boolean(metadata?.permissionDetail?.editable)
  );
  const isDirectory = $derived(metadata?.entry.entryType === "directory");

  async function refresh() {
    loading = true;
    try {
      metadata = await ipc.inspectorGet(source, path, connectionId ?? undefined);
      if (metadata.permissionDetail) {
        bits = {
          user: { ...metadata.permissionDetail.user },
          group: { ...metadata.permissionDetail.group },
          world: { ...metadata.permissionDetail.world }
        };
      }
      error = null;
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      loading = false;
    }
  }

  function modeFromBits(value: {
    user: PermissionBits;
    group: PermissionBits;
    world: PermissionBits;
  }): number {
    const classBits = (bits: PermissionBits) =>
      (bits.read ? 4 : 0) + (bits.write ? 2 : 0) + (bits.execute ? 1 : 0);
    return (
      (classBits(value.user) << 6) |
      (classBits(value.group) << 3) |
      classBits(value.world)
    );
  }

  function symbolicFromBits(value: {
    user: PermissionBits;
    group: PermissionBits;
    world: PermissionBits;
  }): string {
    const classText = (bits: PermissionBits) =>
      `${bits.read ? "r" : "-"}${bits.write ? "w" : "-"}${bits.execute ? "x" : "-"}`;
    return `${classText(value.user)}${classText(value.group)}${classText(value.world)}`;
  }

  async function applyMode() {
    if (!connectionId || !editable) {
      return;
    }
    try {
      await ipc.chmod(connectionId, path, modeFromBits(bits));
      await refresh();
      onchanged?.();
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }

  async function applyRecursive() {
    if (!connectionId || !editable || !isDirectory) {
      return;
    }
    const confirmed = await confirmAction(
      `Apply mode ${octal} (${symbolic}) to every folder and file under "${path}"? Symbolic links will not be followed.`,
      "Apply to enclosed items"
    );
    if (!confirmed) {
      return;
    }
    recursiveBusy = true;
    try {
      const result = await ipc.chmodRecursive(
        connectionId,
        path,
        modeFromBits(bits),
        true
      );
      recursiveNotice =
        `${result.changed} changed` +
        (result.skipped ? `, ${result.skipped} skipped` : "") +
        (result.failed.length ? `, ${result.failed.length} failed` : "");
      recursiveFailures = result.failed;
      await refresh();
      onchanged?.();
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      recursiveBusy = false;
    }
  }

  async function copyPath() {
    if (!metadata) {
      return;
    }
    try {
      await navigator.clipboard.writeText(metadata.entry.path);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      error = "The path could not be copied to the clipboard.";
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
      <dd class="path">
        {metadata.entry.path}
        <button type="button" class="copy" onclick={() => void copyPath()}>
          Copy path
        </button>
        {#if copied}
          <span role="status" class="copied">Path copied</span>
        {/if}
      </dd>
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
        {#if metadata.permissionDetail}
          {#if hasPermissionValues}
            <div class="permission-summary">
              <span>Octal: {octal}</span>
              <span>{symbolic}</span>
            </div>
            <div class="permission-matrix">
              <div class="permission-row">
                <span>User</span>
                <label
                  ><input
                    type="checkbox"
                    aria-label="User Read"
                    bind:checked={bits.user.read}
                    disabled={!editable}
                  /> Read</label
                >
                <label
                  ><input
                    type="checkbox"
                    aria-label="User Write"
                    bind:checked={bits.user.write}
                    disabled={!editable}
                  /> Write</label
                >
                <label
                  ><input
                    type="checkbox"
                    aria-label="User Execute"
                    bind:checked={bits.user.execute}
                    disabled={!editable}
                  /> Execute</label
                >
              </div>
              <div class="permission-row">
                <span>Group</span>
                <label
                  ><input
                    type="checkbox"
                    aria-label="Group Read"
                    bind:checked={bits.group.read}
                    disabled={!editable}
                  /> Read</label
                >
                <label
                  ><input
                    type="checkbox"
                    aria-label="Group Write"
                    bind:checked={bits.group.write}
                    disabled={!editable}
                  /> Write</label
                >
                <label
                  ><input
                    type="checkbox"
                    aria-label="Group Execute"
                    bind:checked={bits.group.execute}
                    disabled={!editable}
                  /> Execute</label
                >
              </div>
              <div class="permission-row">
                <span>World</span>
                <label
                  ><input
                    type="checkbox"
                    aria-label="World Read"
                    bind:checked={bits.world.read}
                    disabled={!editable}
                  /> Read</label
                >
                <label
                  ><input
                    type="checkbox"
                    aria-label="World Write"
                    bind:checked={bits.world.write}
                    disabled={!editable}
                  /> Write</label
                >
                <label
                  ><input
                    type="checkbox"
                    aria-label="World Execute"
                    bind:checked={bits.world.execute}
                    disabled={!editable}
                  /> Execute</label
                >
              </div>
            </div>
            {#if metadata.permissionDetail.reason}
              <p class="notice">{metadata.permissionDetail.reason}</p>
            {:else if !metadata.permissionsSupported}
              <p class="notice">This server does not support changing permissions.</p>
            {/if}
          {:else}
            <p class="notice">
              {metadata.permissionDetail.reason ??
                "Permission detail is unavailable for this item."}
            </p>
          {/if}
        {:else}
          <p class="notice">Permission detail is unavailable for this item.</p>
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

    {#if editable}
      <div class="chmod-actions">
        <button type="button" onclick={() => void applyMode()}>Apply</button>
        {#if isDirectory}
          <button
            type="button"
            disabled={recursiveBusy}
            onclick={() => void applyRecursive()}
          >
            {recursiveBusy ? "Applying…" : "Apply to enclosed…"}
          </button>
        {/if}
      </div>
    {/if}
    {#if recursiveNotice}
      <p class="notice" role="status">{recursiveNotice}</p>
      {#if recursiveFailures.length > 0}
        <ul class="recursive-failures" aria-label="Recursive permission failures">
          {#each recursiveFailures as failure (failure.path)}
            <li>{failure.path}: {failure.message}</li>
          {/each}
        </ul>
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
  .copy {
    margin-left: 0.4rem;
    font: inherit;
    font-size: 0.75rem;
  }
  .copied {
    margin-left: 0.4rem;
    color: var(--success-text);
    font-size: 0.75rem;
  }
  .permission-summary {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 0.35rem;
    font-family: monospace;
  }
  .permission-matrix {
    display: grid;
    gap: 0.3rem;
  }
  .permission-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }
  .permission-row > span {
    width: 3.5rem;
    color: GrayText;
  }
  .permission-row label {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
  }
  .chmod-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .recursive-failures {
    margin: 0;
    padding-left: 1.2rem;
    color: var(--danger-text);
    font-size: 0.75rem;
  }
  .notice {
    font-size: 0.8rem;
    color: GrayText;
    margin: 0;
  }
  .error {
    color: var(--danger-text);
    font-size: 0.8rem;
    margin: 0;
  }
</style>
