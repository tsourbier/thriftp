<script lang="ts">
  import HistoryList from "./HistoryList.svelte";
  import type {
    ConnectionHistoryEntry,
    HubTab,
    ServerFavoriteView
  } from "../../lib/ipc/types";

  let {
    favorites = [],
    history = [],
    hubTab = "favorites",
    onselecttab,
    onconnect,
    onhistoryconnect,
    onedit,
    ondelete,
    onpin,
    oncreate,
    onclearhistory
  }: {
    favorites?: ServerFavoriteView[];
    history?: ConnectionHistoryEntry[];
    hubTab?: HubTab;
    onselecttab?: (tab: HubTab) => void;
    onconnect?: (favorite: ServerFavoriteView) => void;
    onhistoryconnect?: (entry: ConnectionHistoryEntry) => void;
    onedit?: (favorite: ServerFavoriteView) => void;
    ondelete?: (favorite: ServerFavoriteView) => void;
    onpin?: (favorite: ServerFavoriteView, pinned: boolean) => void;
    oncreate?: () => void;
    onclearhistory?: () => void;
  } = $props();

  let query = $state("");
  const filtered = $derived(
    favorites.filter((favorite) => {
      const needle = query.trim().toLocaleLowerCase();
      if (!needle) {
        return true;
      }
      return [
        favorite.displayName,
        favorite.host,
        favorite.groupLabel ?? "",
        favorite.remotePath ?? ""
      ].some((value) => value.toLocaleLowerCase().includes(needle));
    })
  );
</script>

<section class="hub" aria-label="Connection hub">
  <div class="tabs" role="tablist">
    <button
      type="button"
      role="tab"
      aria-selected={hubTab === "favorites"}
      class:active={hubTab === "favorites"}
      onclick={() => onselecttab?.("favorites")}
    >
      Favorites
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={hubTab === "history"}
      class:active={hubTab === "history"}
      onclick={() => onselecttab?.("history")}
    >
      History
    </button>
  </div>

  {#if hubTab === "favorites"}
    <div class="favorites">
      <div class="search">
        <input
          bind:value={query}
          placeholder="Search favorites"
          aria-label="Search server favorites"
          autocomplete="off"
        />
        <button type="button" onclick={oncreate}>New favorite</button>
      </div>
      {#if filtered.length === 0}
        <p class="empty">
          {query
            ? "No matching favorites."
            : "Save a server to reconnect without re-entering its details."}
        </p>
      {:else}
        <ul>
          {#each filtered as favorite (favorite.id)}
            <li>
              <button type="button" class="entry" onclick={() => onconnect?.(favorite)}>
                <strong>{favorite.displayName}</strong>
                <span>{favorite.protocol} · {favorite.host}:{favorite.port}</span>
                <span
                  >{favorite.username}{favorite.remotePath
                    ? ` · ${favorite.remotePath}`
                    : ""}</span
                >
              </button>
              <div class="actions">
                <button type="button" onclick={() => onpin?.(favorite, !favorite.pinned)}>
                  {favorite.pinned ? "Unpin" : "Pin"}
                </button>
                <button type="button" onclick={() => onedit?.(favorite)}>Edit</button>
                <button type="button" onclick={() => ondelete?.(favorite)}>Delete</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {:else}
    <HistoryList
      entries={history}
      onconnect={onhistoryconnect}
      onclear={onclearhistory}
    />
  {/if}
</section>

<style>
  .hub {
    display: grid;
    gap: 0.6rem;
    padding: 0.75rem;
    height: 100%;
    align-content: start;
    overflow: auto;
  }
  .tabs {
    display: flex;
    gap: 0.25rem;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    font: inherit;
    font-size: 0.85rem;
    padding: 0.35rem 0.6rem;
    background: transparent;
    border: 0;
    border-bottom: 2px solid transparent;
    cursor: pointer;
  }
  .tabs button.active {
    border-bottom-color: var(--accent);
    font-weight: 600;
  }
  .favorites {
    display: grid;
    gap: 0.5rem;
  }
  .search {
    display: flex;
    gap: 0.35rem;
  }
  .search input {
    flex: 1;
    min-width: 0;
    padding: 0.3rem;
    font: inherit;
  }
  .search button,
  .actions button {
    font: inherit;
    font-size: 0.75rem;
    cursor: pointer;
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
    justify-content: space-between;
    gap: 0.5rem;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 0.3rem);
    background: var(--surface);
    padding: 0.4rem;
  }
  .entry {
    display: grid;
    text-align: left;
    gap: 0.1rem;
    background: transparent;
    border: 0;
    font: inherit;
    cursor: pointer;
    min-width: 0;
  }
  .entry span {
    font-size: 0.72rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    gap: 0.2rem;
    flex-wrap: wrap;
  }
  .empty {
    margin: 0;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
</style>
