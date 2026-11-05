<script lang="ts">
  import type {
    FavoritePreferences,
    FavoriteSortKey,
    ServerFavoriteView
  } from "../../lib/ipc/types";
  import type { FavoriteConnectionState } from "../../lib/server-favorites/stores";
  import GroupMenu from "./GroupMenu.svelte";

  let {
    favorites = [],
    preferences = { sortKey: "name", pinnedFirst: true },
    activeFavoriteId = null,
    connectionStates = {},
    onconnect,
    onedit,
    ondelete,
    onpin,
    onsort,
    ongroup,
    onrenamegroup,
    onungroup,
    oncreate
  }: {
    favorites?: ServerFavoriteView[];
    preferences?: FavoritePreferences;
    activeFavoriteId?: string | null;
    connectionStates?: Record<string, FavoriteConnectionState>;
    onconnect: (favorite: ServerFavoriteView) => void;
    onedit: (favorite: ServerFavoriteView) => void;
    ondelete: (favorite: ServerFavoriteView) => void;
    onpin: (favorite: ServerFavoriteView, pinned: boolean) => void;
    onsort?: (sortKey: FavoriteSortKey) => void;
    ongroup?: (favorite: ServerFavoriteView, label: string) => void;
    onrenamegroup?: (oldLabel: string, newLabel: string) => void;
    onungroup?: (favorite: ServerFavoriteView) => void;
    oncreate?: () => void;
  } = $props();

  let pendingDelete = $state<ServerFavoriteView | null>(null);
  let expandedGroupId = $state<string | null>(null);
  let renameFrom = $state("");
  let renameTo = $state("");

  const pinnedFavorites = $derived(
    preferences.pinnedFirst ? favorites.filter((favorite) => favorite.pinned) : []
  );
  const remainingFavorites = $derived(
    preferences.pinnedFirst ? favorites.filter((favorite) => !favorite.pinned) : favorites
  );
  const groups = $derived(
    Array.from(
      new Set(
        favorites
          .map((favorite) => favorite.groupLabel)
          .filter((label): label is string => Boolean(label))
      )
    ).sort()
  );

  function statusFor(favorite: ServerFavoriteView): string {
    if (favorite.id === activeFavoriteId) {
      return "connected";
    }
    return connectionStates[favorite.id]?.state ?? "disconnected";
  }

  function errorFor(favorite: ServerFavoriteView): string | undefined {
    return connectionStates[favorite.id]?.error;
  }

  function protocolLabel(protocol: ServerFavoriteView["protocol"]): string {
    switch (protocol) {
      case "ftp":
        return "FTP";
      case "ftpsExplicit":
        return "FTPS (explicit)";
      case "ftpsImplicit":
        return "FTPS (implicit)";
      case "sftp":
        return "SFTP";
    }
  }
</script>

{#snippet rows(items: ServerFavoriteView[])}
  {#each items as favorite (favorite.id)}
    {@const status = statusFor(favorite)}
    <li
      class:active={favorite.id === activeFavoriteId}
      class:pinned={favorite.pinned}
      data-status={status}
    >
      <div class="identity">
        <button
          type="button"
          class="name"
          onclick={() => onconnect(favorite)}
          aria-label={`Connect to ${favorite.displayName}`}
          aria-current={favorite.id === activeFavoriteId ? "true" : undefined}
        >
          {favorite.displayName}
        </button>
        <span class="meta">
          {protocolLabel(favorite.protocol)} · {favorite.host}:{favorite.port} ·
          {favorite.username}
        </span>
        {#if favorite.groupLabel}
          <span class="group-label">Group: {favorite.groupLabel}</span>
        {/if}
        {#if favorite.remotePath}
          <span class="path">{favorite.remotePath}</span>
        {/if}
        {#if favorite.credentialPolicy === "save"}
          <span class="credential">credentials saved</span>
        {/if}
        <span class="state {status}" role="status" aria-live="polite">
          {#if status === "connecting"}
            connecting…
          {:else if status === "connected"}
            connected
          {:else if status === "error"}
            connection failed
          {:else}
            disconnected
          {/if}
        </span>
        {#if errorFor(favorite)}
          <span class="error" role="alert">{errorFor(favorite)}</span>
        {/if}
      </div>
      <div class="actions">
        <button
          type="button"
          aria-label={favorite.pinned
            ? `Unpin ${favorite.displayName}`
            : `Pin ${favorite.displayName}`}
          onclick={() => onpin(favorite, !favorite.pinned)}
        >
          {favorite.pinned ? "Unpin" : "Pin"}
        </button>
        <button type="button" onclick={() => onedit(favorite)}>Edit</button>
        <button
          type="button"
          aria-label={`Group ${favorite.displayName}`}
          aria-expanded={expandedGroupId === favorite.id}
          onclick={() =>
            (expandedGroupId = expandedGroupId === favorite.id ? null : favorite.id)}
        >
          Group
        </button>
        <button type="button" onclick={() => (pendingDelete = favorite)}> Delete </button>
      </div>
      {#if expandedGroupId === favorite.id}
        <div class="group-panel">
          <GroupMenu
            groupLabel={favorite.groupLabel}
            onassign={(label) => ongroup?.(favorite, label)}
            onrename={(oldLabel, newLabel) => onrenamegroup?.(oldLabel, newLabel)}
            onungroup={() => onungroup?.(favorite)}
          />
        </div>
      {/if}
      {#if pendingDelete?.id === favorite.id}
        <div
          class="confirm-delete"
          role="alertdialog"
          aria-label={`Confirm delete ${favorite.displayName}`}
        >
          <p>
            Delete <strong>{favorite.displayName}</strong>? The saved connection is
            removed. Active connections stay connected until you disconnect them.
          </p>
          <div class="actions">
            <button
              type="button"
              onclick={() => {
                ondelete(favorite);
                pendingDelete = null;
              }}
            >
              Confirm delete
            </button>
            <button type="button" onclick={() => (pendingDelete = null)}> Cancel </button>
          </div>
        </div>
      {/if}
    </li>
  {/each}
{/snippet}

<section class="server-favorites" aria-label="Server favorites">
  <div class="list-controls">
    <label>
      Sort by
      <select
        aria-label="Sort favorites"
        value={preferences.sortKey}
        onchange={(event) => onsort?.(event.currentTarget.value as FavoriteSortKey)}
      >
        <option value="name">Name</option>
        <option value="protocol">Protocol</option>
        <option value="host">Host</option>
        <option value="lastUsed">Last used</option>
      </select>
    </label>
    {#if oncreate}
      <button type="button" class="new-favorite" onclick={oncreate}>
        New favorite
      </button>
    {/if}
    {#if groups.length > 0}
      <div class="rename-group" role="group" aria-label="Rename group">
        <label>
          Rename group
          <select bind:value={renameFrom} aria-label="Group to rename">
            <option value="">Select a group</option>
            {#each groups as group (group)}
              <option value={group}>{group}</option>
            {/each}
          </select>
        </label>
        <label>
          New label
          <input bind:value={renameTo} maxlength="100" aria-label="New group label" />
        </label>
        <button
          type="button"
          disabled={!renameFrom || !renameTo.trim()}
          onclick={() => {
            if (renameFrom && renameTo.trim()) {
              onrenamegroup?.(renameFrom, renameTo.trim());
              renameFrom = "";
              renameTo = "";
            }
          }}
        >
          Rename
        </button>
      </div>
    {/if}
  </div>

  {#if favorites.length === 0}
    <div class="empty">
      <p>No saved servers yet. Create your first favorite.</p>
      {#if oncreate}
        <button type="button" onclick={oncreate}>Create favorite</button>
      {/if}
    </div>
  {:else}
    {#if pinnedFavorites.length > 0}
      <section aria-label="Pinned favorites">
        <h3>Pinned</h3>
        <ul>{@render rows(pinnedFavorites)}</ul>
      </section>
    {/if}
    <section aria-label={pinnedFavorites.length > 0 ? "Other favorites" : "Favorites"}>
      {#if pinnedFavorites.length > 0}<h3>Favorites</h3>{/if}
      <ul>{@render rows(remainingFavorites)}</ul>
    </section>
  {/if}
</section>

<style>
  .server-favorites {
    padding: 0.75rem;
  }
  .list-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }
  .list-controls label,
  .rename-group label {
    display: grid;
    gap: 0.2rem;
    font-size: 0.8rem;
  }
  .rename-group {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 0.35rem;
  }
  h3 {
    font-size: 0.8rem;
    margin: 0.4rem 0;
    color: var(--server-favorites-muted, GrayText);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.35rem;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.45rem;
    border: 1px solid var(--server-favorites-border, #d0d0d0);
    border-radius: 0.35rem;
  }
  li.active {
    border-color: #3b82f6;
  }
  li.pinned .name::before {
    content: "★ ";
    color: #b58900;
  }
  .identity {
    display: grid;
    gap: 0.15rem;
    text-align: left;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    border: 0;
    background: transparent;
    padding: 0;
    text-align: left;
    cursor: pointer;
  }
  .name:focus-visible,
  button:focus-visible,
  select:focus-visible,
  input:focus-visible {
    outline: 2px solid #3b82f6;
    outline-offset: 2px;
  }
  .meta,
  .credential,
  .group-label,
  .state,
  .error {
    font-size: 0.78rem;
  }
  .group-label {
    color: var(--server-favorites-group, #315b7d);
  }
  .state.connected {
    color: var(--server-favorites-connected, #1a5c1a);
    font-weight: 600;
  }
  .state.connecting {
    color: var(--server-favorites-warning, #8a5a00);
  }
  .state.error,
  .error {
    color: var(--server-favorites-error, #8a0000);
  }
  .state.disconnected {
    color: GrayText;
  }
  .actions {
    display: flex;
    align-items: start;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .group-panel {
    flex-basis: 100%;
    border-top: 1px solid var(--server-favorites-border, #e0e0e0);
    padding-top: 0.4rem;
  }
  .confirm-delete {
    flex-basis: 100%;
    border-top: 1px solid #e0e0e0;
    padding-top: 0.4rem;
  }
  .confirm-delete p {
    margin: 0 0 0.4rem;
    font-size: 0.85rem;
  }
  .empty {
    color: var(--server-favorites-muted, #666);
  }
  .empty p {
    margin: 0 0 0.5rem;
  }
  .new-favorite {
    align-self: end;
  }
</style>
