<script lang="ts">
  import { untrack } from "svelte";
  import DirectoryList from "./DirectoryList.svelte";
  import BrowserToolbar from "./BrowserToolbar.svelte";
  import ConflictDialog from "../transfers/ConflictDialog.svelte";
  import TextPrompt from "../common/TextPrompt.svelte";
  import { confirmAction } from "../../lib/dialogs/confirm";
  import { IpcError, ipc } from "../../lib/ipc/client";
  import { openListing, releaseListing } from "../../lib/browser/listings";
  import { handleBrowserKey } from "../../lib/keyboard/browser";
  import { visibleEntries } from "../../lib/browser/view";
  import type {
    BrowserSource,
    ConflictPolicy,
    DirectoryBatchPayload,
    FileEntryView,
    SortKey
  } from "../../lib/ipc/types";

  type PaneState = {
    source: BrowserSource;
    path: string;
    entries: FileEntryView[];
    listingId: string | null;
    state: "pending" | "partial" | "complete" | "cancelled" | "error";
    loading: boolean;
    error: string | null;
    selectedPath: string | null;
    selectedPaths: string[];
    sortKey: SortKey;
    showHidden: boolean;
    filter: string;
    history: string[];
    historyIndex: number;
  };

  let {
    connectionId = null,
    connectionLabel = "",
    profileId = null,
    remotePath = null,
    localPath = ".",
    localSelectedItems = [],
    remoteSelectedItems = [],
    defaultShowHidden = false,
    defaultSortKey = "name",
    onpreview,
    oninspect,
    ontransferstarted,
    onremotepathchange,
    onlocalpathchange,
    onviewchange
  }: {
    connectionId?: string | null;
    connectionLabel?: string;
    profileId?: string | null;
    remotePath?: string | null;
    localPath?: string;
    localSelectedItems?: string[];
    remoteSelectedItems?: string[];
    defaultShowHidden?: boolean;
    defaultSortKey?: SortKey;
    onpreview?: (entry: FileEntryView, source: BrowserSource) => void;
    oninspect?: (entry: FileEntryView, source: BrowserSource) => void;
    ontransferstarted?: () => void;
    onremotepathchange?: (path: string) => void;
    onlocalpathchange?: (path: string) => void;
    onselectionchange?: (source: BrowserSource, paths: string[]) => void;
    onviewchange?: (settings: {
      sortKey: SortKey;
      showHidden: boolean;
      filter: string;
    }) => void;
  } = $props();

  function pane(source: BrowserSource, path: string): PaneState {
    return {
      source,
      path,
      entries: [],
      listingId: null,
      state: "pending",
      loading: false,
      error: null,
      selectedPath: null,
      selectedPaths: [],
      sortKey: defaultSortKey,
      showHidden: defaultShowHidden,
      filter: "",
      history: [path],
      historyIndex: 0
    };
  }

  let local = $state<PaneState>(
    untrack(() => ({
      ...pane("local", localPath),
      selectedPaths: localSelectedItems,
      selectedPath: localSelectedItems[0] ?? null
    }))
  );
  let remote = $state<PaneState>(
    untrack(() => ({
      ...pane("remote", "."),
      selectedPaths: remoteSelectedItems,
      selectedPath: remoteSelectedItems[0] ?? null
    }))
  );
  let activePane = $state<BrowserSource>("local");
  let conflict = $state<{
    direction: "upload" | "download";
    sourcePaths: string[];
    destination: string;
    message: string;
  } | null>(null);
  let textPrompt = $state<{
    title: string;
    label: string;
    initial: string;
    submit: (value: string) => void;
  } | null>(null);

  const localEntries = $derived(visibleEntries(local.entries, local));
  const remoteEntries = $derived(visibleEntries(remote.entries, remote));

  function applyBatch(state: PaneState, batch: DirectoryBatchPayload) {
    state.entries = [...state.entries, ...batch.entries];
    state.state = batch.state;
    if (batch.state === "complete") {
      state.loading = false;
    }
  }

  async function load(state: PaneState) {
    const previous = state.listingId;
    state.entries = [];
    state.loading = true;
    state.error = null;
    state.state = "pending";
    state.selectedPath = null;
    state.selectedPaths = [];
    if (previous) {
      releaseListing(previous);
      void ipc.cancelListing(previous).catch(() => undefined);
    }
    try {
      const handle = await openListing(
        () =>
          state.source === "local"
            ? ipc.openLocal(state.path)
            : ipc.openRemote(connectionId ?? "", state.path),
        {
          onBatch: (batch) => applyBatch(state, batch),
          onError: (payload) => {
            state.error = payload.error.message;
            state.loading = false;
            state.state = "error";
          }
        }
      );
      state.listingId = handle.listingId;
    } catch (error) {
      state.error = messageOf(error);
      state.loading = false;
      state.state = "error";
    }
  }

  function navigate(state: PaneState, path: string, record = true) {
    if (record && path !== state.path) {
      state.history = state.history.slice(0, state.historyIndex + 1);
      state.history.push(path);
      state.historyIndex = state.history.length - 1;
    }
    state.path = path;
    if (state.source === "remote") {
      onremotepathchange?.(path);
    } else {
      onlocalpathchange?.(path);
    }
    void load(state);
  }

  function back(state: PaneState) {
    if (state.historyIndex <= 0) {
      return;
    }
    state.historyIndex -= 1;
    navigate(state, state.history[state.historyIndex], false);
  }

  function forward(state: PaneState) {
    if (state.historyIndex + 1 >= state.history.length) {
      return;
    }
    state.historyIndex += 1;
    navigate(state, state.history[state.historyIndex], false);
  }

  function up(state: PaneState) {
    const normalized = state.path.replace(/\/+$/, "");
    if (normalized === "" || normalized === "/" || normalized === ".") {
      return;
    }
    const index = normalized.lastIndexOf("/");
    navigate(state, index <= 0 ? "/" : normalized.slice(0, index));
  }

  function openEntry(state: PaneState, entry: FileEntryView) {
    if (entry.entryType === "directory") {
      navigate(state, entry.path);
    } else if (entry.entryType === "file" && onpreview) {
      onpreview(entry, state.source);
    }
  }

  function join(base: string, name: string) {
    if (base.endsWith("/")) {
      return `${base}${name}`;
    }
    return `${base}/${name}`;
  }

  async function startTransfer(
    direction: "upload" | "download",
    sourcePaths: string[],
    destination: string,
    conflictPolicy?: ConflictPolicy
  ) {
    if (!profileId || sourcePaths.length === 0) {
      return;
    }
    try {
      const result = await ipc.transfersStart({
        direction,
        profileId,
        sources: sourcePaths,
        destination,
        conflictPolicy
      });
      if (result.skippedLinks?.length) {
        notice = `Skipped symbolic links: ${result.skippedLinks.join(", ")}`;
      }
      ontransferstarted?.();
    } catch (error) {
      if (error instanceof IpcError && error.details.code === "conflict") {
        conflict = {
          direction,
          sourcePaths,
          destination,
          message: error.details.message
        };
        return;
      }
      notice = messageOf(error);
    }
  }

  async function resolveConflict(
    policy: ConflictPolicy,
    _applyToAll: boolean,
    _renamed?: string
  ) {
    const pending = conflict;
    conflict = null;
    if (pending) {
      await startTransfer(
        pending.direction,
        pending.sourcePaths,
        pending.destination,
        policy
      );
    }
  }

  function inspectSelected(state: PaneState) {
    const entry = state.entries.find(
      (candidate) => candidate.path === state.selectedPath
    );
    if (entry) {
      oninspect?.(entry, state.source);
    }
  }

  function selectedEntries(state: PaneState) {
    const paths =
      state.selectedPaths.length > 0
        ? state.selectedPaths
        : state.selectedPath
          ? [state.selectedPath]
          : [];
    return state.entries.filter((entry) => paths.includes(entry.path));
  }

  async function uploadSelected() {
    const entries = selectedEntries(local);
    if (entries.length === 0 || !connectionId) {
      return;
    }
    await startTransfer(
      "upload",
      entries.map((entry) => entry.path),
      remote.path
    );
    await load(remote);
  }

  async function downloadSelected() {
    const entries = selectedEntries(remote);
    if (entries.length === 0 || !connectionId) {
      return;
    }
    await startTransfer(
      "download",
      entries.map((entry) => entry.path),
      local.path
    );
    await load(local);
  }

  async function dropOn(event: DragEvent, state: PaneState) {
    event.preventDefault();
    const payload = event.dataTransfer?.getData("application/thriftp-entry");
    if (!payload) {
      return;
    }
    const data = JSON.parse(payload) as { source: BrowserSource; path: string };
    if (data.source === state.source) {
      return;
    }
    const direction = state.source === "remote" ? "upload" : "download";
    await startTransfer(direction, [data.path], state.path);
    await load(state);
  }

  let notice = $state<string | null>(null);

  function newFolder(state: PaneState) {
    textPrompt = {
      title: "New folder",
      label: "Folder name",
      initial: "",
      submit: (name) => {
        void (async () => {
          try {
            await ipc.mkdir(
              state.source,
              join(state.path, name),
              state.source === "remote" ? (connectionId ?? undefined) : undefined
            );
            await load(state);
          } catch (error) {
            state.error = messageOf(error);
          }
        })();
      }
    };
  }

  function renameSelected(state: PaneState) {
    const entry = state.entries.find(
      (candidate) => candidate.path === state.selectedPath
    );
    if (!entry) {
      return;
    }
    textPrompt = {
      title: "Rename",
      label: "New name",
      initial: entry.name,
      submit: (name) => {
        if (name === entry.name) {
          return;
        }
        void (async () => {
          try {
            await ipc.rename(
              state.source,
              entry.path,
              join(parentOf(entry.path), name),
              state.source === "remote" ? (connectionId ?? undefined) : undefined
            );
            await load(state);
          } catch (error) {
            state.error = messageOf(error);
          }
        })();
      }
    };
  }

  async function deleteSelected(state: PaneState) {
    const entry = state.entries.find(
      (candidate) => candidate.path === state.selectedPath
    );
    if (!entry) {
      return;
    }
    const message =
      state.source === "remote"
        ? `Permanently delete "${entry.name}" on the server? This cannot be undone.`
        : `Move "${entry.name}" to the trash?`;
    if (!(await confirmAction(message, "Delete"))) {
      return;
    }
    try {
      await ipc.deletePath(
        state.source,
        entry.path,
        true,
        state.source === "remote" ? (connectionId ?? undefined) : undefined
      );
      await load(state);
    } catch (error) {
      state.error = messageOf(error);
    }
  }

  function parentOf(path: string) {
    const index = path.replace(/\/+$/, "").lastIndexOf("/");
    return index <= 0 ? "/" : path.slice(0, index);
  }

  function messageOf(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }

  function selectEntry(state: PaneState, entry: FileEntryView, additive = false) {
    if (additive) {
      state.selectedPaths = state.selectedPaths.includes(entry.path)
        ? state.selectedPaths.filter((path) => path !== entry.path)
        : [...state.selectedPaths, entry.path];
    } else {
      state.selectedPaths = [entry.path];
    }
    state.selectedPath = entry.path;
    onselectionchange?.(state.source, state.selectedPaths);
  }

  function keydown(event: KeyboardEvent, state: PaneState, entries: FileEntryView[]) {
    activePane = state.source;
    const handled = handleBrowserKey(event, {
      entries,
      selectedPath: state.selectedPath,
      onselect: (entry) => selectEntry(state, entry),
      onopen: (entry) => openEntry(state, entry),
      onup: () => up(state),
      onback: () => back(state),
      onforward: () => forward(state),
      onrefresh: () => void load(state),
      ontoggleHidden: () => (state.showHidden = !state.showHidden),
      onsort: (key) => (state.sortKey = key),
      ondelete: (entry) => {
        state.selectedPath = entry.path;
        void deleteSelected(state);
      },
      onrename: (entry) => {
        state.selectedPath = entry.path;
        renameSelected(state);
      },
      onnewfolder: () => newFolder(state),
      oninspect: (entry) => {
        state.selectedPath = entry.path;
        inspectSelected(state);
      }
    });
    if (handled) {
      event.preventDefault();
    }
  }

  $effect(() => {
    const id = connectionId;
    untrack(() => {
      if (id) {
        remote.path = remotePath || ".";
        void load(remote);
      } else {
        remote.entries = [];
        remote.listingId = null;
        remote.state = "pending";
      }
    });
  });

  $effect(() => {
    const path = remotePath;
    untrack(() => {
      if (path && connectionId && path !== remote.path) {
        remote.path = path;
        void load(remote);
      }
    });
  });

  $effect(() => {
    const path = localPath;
    untrack(() => {
      if (path && path !== local.path) {
        local.path = path;
        local.history = [path];
        local.historyIndex = 0;
        void load(local);
      }
    });
  });

  $effect(() => {
    untrack(() => {
      void load(local);
    });
  });

  $effect(() => {
    if (!connectionId) {
      return;
    }
    onviewchange?.({
      sortKey: remote.sortKey,
      showHidden: remote.showHidden,
      filter: remote.filter
    });
  });
</script>

<section
  class="dual-pane"
  class:remote-active={activePane === "remote"}
  aria-label="Dual-pane file browser"
>
  <section
    class="pane"
    class:active={activePane === "local"}
    aria-label="Local files"
    onfocusin={() => (activePane = "local")}
    ondragover={(event) => event.preventDefault()}
    ondrop={(event) => void dropOn(event, local)}
  >
    <div class="pane-heading">
      <h2>Local</h2>
      <div class="actions">
        <button type="button" onclick={() => newFolder(local)}>New folder</button>
        <button type="button" onclick={() => renameSelected(local)}>Rename</button>
        <button type="button" onclick={() => deleteSelected(local)}>Delete</button>
        <button type="button" onclick={() => inspectSelected(local)}>Info</button>
        <button
          type="button"
          disabled={!connectionId}
          onclick={() => void uploadSelected()}
        >
          Upload
        </button>
      </div>
    </div>
    <BrowserToolbar
      path={local.path}
      bind:sortKey={local.sortKey}
      bind:showHidden={local.showHidden}
      bind:filter={local.filter}
      canGoUp={local.path !== "/" && local.path !== "."}
      canGoBack={local.historyIndex > 0}
      canGoForward={local.historyIndex + 1 < local.history.length}
      loading={local.loading}
      onnavigate={(path) => navigate(local, path)}
      onup={() => up(local)}
      onback={() => back(local)}
      onforward={() => forward(local)}
      onrefresh={() => void load(local)}
      onsort={(key) => (local.sortKey = key)}
    />
    {#if local.error || notice}
      <p class="pane-error" role="alert">{local.error ?? notice}</p>
    {/if}
    <div
      class="list"
      role="presentation"
      onkeydown={(event) => keydown(event, local, localEntries)}
    >
      <DirectoryList
        entries={localEntries}
        loading={local.loading}
        listingState={local.state}
        selectedPath={local.selectedPath}
        selectedPaths={local.selectedPaths}
        source="local"
        onselect={(entry, additive) => selectEntry(local, entry, additive)}
        onopen={(entry) => openEntry(local, entry)}
      />
    </div>
  </section>

  <section
    class="pane"
    class:active={activePane === "remote"}
    aria-label="Remote files"
    onfocusin={() => (activePane = "remote")}
    ondragover={(event) => event.preventDefault()}
    ondrop={(event) => void dropOn(event, remote)}
  >
    <div class="pane-heading">
      <h2>Remote {connectionLabel ? `· ${connectionLabel}` : ""}</h2>
      <div class="actions">
        <button type="button" disabled={!connectionId} onclick={() => newFolder(remote)}>
          New folder
        </button>
        <button
          type="button"
          disabled={!connectionId}
          onclick={() => renameSelected(remote)}
        >
          Rename
        </button>
        <button
          type="button"
          disabled={!connectionId}
          onclick={() => deleteSelected(remote)}
        >
          Delete
        </button>
        <button type="button" onclick={() => inspectSelected(remote)}>Info</button>
        <button
          type="button"
          disabled={!connectionId}
          onclick={() => void downloadSelected()}
        >
          Download
        </button>
      </div>
    </div>
    {#if connectionId}
      <BrowserToolbar
        path={remote.path}
        bind:sortKey={remote.sortKey}
        bind:showHidden={remote.showHidden}
        bind:filter={remote.filter}
        canGoUp={remote.path !== "/" && remote.path !== "."}
        canGoBack={remote.historyIndex > 0}
        canGoForward={remote.historyIndex + 1 < remote.history.length}
        loading={remote.loading}
        onnavigate={(path) => navigate(remote, path)}
        onup={() => up(remote)}
        onback={() => back(remote)}
        onforward={() => forward(remote)}
        onrefresh={() => void load(remote)}
        onsort={(key) => (remote.sortKey = key)}
      />
      {#if remote.error}
        <p class="pane-error" role="alert">{remote.error}</p>
      {/if}
      <div
        class="list"
        role="presentation"
        onkeydown={(event) => keydown(event, remote, remoteEntries)}
      >
        <DirectoryList
          entries={remoteEntries}
          loading={remote.loading}
          listingState={remote.state}
          selectedPath={remote.selectedPath}
          selectedPaths={remote.selectedPaths}
          source="remote"
          onselect={(entry, additive) => selectEntry(remote, entry, additive)}
          onopen={(entry) => openEntry(remote, entry)}
        />
      </div>
    {:else}
      <p class="empty">Connect to a server to browse remote files.</p>
    {/if}
  </section>
</section>

{#if conflict}
  <div class="conflict-modal" role="presentation">
    <ConflictDialog
      destination={conflict.destination}
      onexisting={resolveConflict}
      oncancel={() => (conflict = null)}
    />
  </div>
{/if}

{#if textPrompt}
  <div class="conflict-modal" role="presentation">
    <div class="prompt-panel">
      <TextPrompt
        title={textPrompt.title}
        label={textPrompt.label}
        initial={textPrompt.initial}
        onsubmit={(value) => {
          const submit = textPrompt?.submit;
          textPrompt = null;
          submit?.(value);
        }}
        oncancel={() => (textPrompt = null)}
      />
    </div>
  </div>
{/if}

<style>
  .conflict-modal {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgb(0 0 0 / 0.25);
  }
  .conflict-modal > :global(*) {
    background: Canvas;
    border-radius: 0.5rem;
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.2);
    padding: 1rem;
  }
  .prompt-panel {
    background: Canvas;
    border-radius: 0.5rem;
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.2);
    padding: 1rem;
  }
  .dual-pane {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1px;
    flex: 1;
    background: #d0d0d0;
    min-height: 0;
  }
  .pane {
    background: Canvas;
    padding: 0.75rem;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .pane.active {
    box-shadow: inset 0 0 0 2px Highlight;
  }
  .pane-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .pane-heading h2 {
    font-size: 0.9rem;
    margin: 0 0 0.5rem;
  }
  .actions {
    display: flex;
    gap: 0.25rem;
  }
  .actions button {
    font: inherit;
    font-size: 0.75rem;
  }
  .list {
    flex: 1;
    min-height: 0;
  }
  .pane-error {
    color: #8a0000;
    font-size: 0.8rem;
  }
  .empty {
    color: GrayText;
    font-size: 0.85rem;
  }
</style>
