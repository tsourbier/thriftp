<script lang="ts">
  import type {
    DirectoryFavorite,
    ServerFavoriteView,
    SidebarSectionKey,
    WorkspaceLayout
  } from "../../lib/ipc/types";
  import { filterSidebar } from "../../lib/workspace/sidebar";

  let {
    layout,
    visible = true,
    overlay = false,
    servers = [],
    directoryFavorites = [],
    automationEnabled = false,
    oncollapse,
    onreopen,
    ontogglesection,
    onconnect,
    onopenfavorite,
    onedit,
    ondelete,
    onpin,
    oncreate,
    onopenautomation,
    onsavedirectoryfavorite
  }: {
    layout: WorkspaceLayout;
    visible?: boolean;
    overlay?: boolean;
    servers?: ServerFavoriteView[];
    directoryFavorites?: DirectoryFavorite[];
    automationEnabled?: boolean;
    oncollapse?: () => void;
    onreopen?: () => void;
    ontogglesection?: (key: SidebarSectionKey, value: boolean) => void;
    onconnect?: (favorite: ServerFavoriteView) => void;
    onopenfavorite?: (favorite: DirectoryFavorite) => void;
    onedit?: (favorite: ServerFavoriteView) => void;
    ondelete?: (favorite: ServerFavoriteView) => void;
    onpin?: (favorite: ServerFavoriteView, pinned: boolean) => void;
    oncreate?: () => void;
    onopenautomation?: () => void;
    onsavedirectoryfavorite?: () => void;
  } = $props();

  let query = $state("");
  const filtered = $derived(filterSidebar({ servers, directoryFavorites, query }));

  function toggle(key: SidebarSectionKey) {
    ontogglesection?.(key, !layout.sidebarSections[key]);
  }
</script>

{#if !visible}
  <button type="button" class="reopen" onclick={onreopen} aria-label="Show sidebar">
    ☰
  </button>
{:else}
  <aside class="sidebar" class:overlay aria-label="Workspace navigation">
    <div class="sidebar-head">
      <strong>Workspace</strong>
      <button type="button" onclick={oncollapse} aria-label="Hide sidebar">‹</button>
    </div>
    <div class="search">
      <input
        bind:value={query}
        placeholder="Search servers and favorites"
        aria-label="Search saved servers and directory favorites"
        autocomplete="off"
      />
      {#if query}
        <button type="button" onclick={() => (query = "")} aria-label="Clear search"
          >×</button
        >
      {/if}
    </div>

    <section class="section">
      <button
        type="button"
        class="section-head"
        aria-expanded={layout.sidebarSections.servers}
        onclick={() => toggle("servers")}
      >
        <span>Saved Servers</span><span>{filtered.servers.length}</span>
      </button>
      {#if layout.sidebarSections.servers}
        {#if filtered.servers.length === 0}
          <p class="empty">{query ? "No matching servers." : "No saved servers yet."}</p>
        {:else}
          <ul>
            {#each filtered.servers as favorite (favorite.id)}
              <li>
                <button type="button" class="entry" onclick={() => onconnect?.(favorite)}>
                  <span class="label">{favorite.displayName}</span>
                  <span class="detail">{favorite.protocol} · {favorite.host}</span>
                  {#if favorite.remotePath}
                    <span class="detail path">{favorite.remotePath}</span>
                  {/if}
                </button>
                <div class="entry-actions">
                  <button
                    type="button"
                    onclick={() => onpin?.(favorite, !favorite.pinned)}
                  >
                    {favorite.pinned ? "Unpin" : "Pin"}
                  </button>
                  <button type="button" onclick={() => onedit?.(favorite)}>Edit</button>
                  <button type="button" onclick={() => ondelete?.(favorite)}
                    >Delete</button
                  >
                </div>
              </li>
            {/each}
          </ul>
        {/if}
        <button type="button" class="create" onclick={oncreate}>New server</button>
      {/if}
    </section>

    <section class="section">
      <button
        type="button"
        class="section-head"
        aria-expanded={layout.sidebarSections.directoryFavorites}
        onclick={() => toggle("directoryFavorites")}
      >
        <span>Directory Favorites</span><span>{filtered.directoryFavorites.length}</span>
      </button>
      {#if layout.sidebarSections.directoryFavorites}
        {#if filtered.directoryFavorites.length === 0}
          <p class="empty">
            {query ? "No matching directory favorites." : "No directory favorites yet."}
          </p>
        {:else}
          <ul>
            {#each filtered.directoryFavorites as favorite (favorite.id)}
              <li>
                <button
                  type="button"
                  class="entry"
                  onclick={() => onopenfavorite?.(favorite)}
                >
                  <span class="label">{favorite.label}</span>
                  <span class="detail path">{favorite.path}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        <button type="button" class="create" onclick={onsavedirectoryfavorite}>
          Save current folder
        </button>
      {/if}
    </section>

    <section class="section">
      <button
        type="button"
        class="section-head"
        aria-expanded={layout.sidebarSections.automation}
        onclick={() => toggle("automation")}
      >
        <span>Automation</span><span>{automationEnabled ? "on" : "off"}</span>
      </button>
      {#if layout.sidebarSections.automation}
        <button type="button" class="entry" onclick={onopenautomation}>
          <span class="label">MCP automation</span>
          <span class="detail">{automationEnabled ? "Enabled" : "Disabled"}</span>
        </button>
      {/if}
    </section>
  </aside>
{/if}

<style>
  .sidebar {
    width: 16rem;
    flex: 0 0 16rem;
    min-width: 14rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.6rem;
    border-right: 1px solid var(--border);
    background: var(--surface-muted);
    overflow: auto;
  }
  .sidebar.overlay {
    position: absolute;
    z-index: 20;
    inset: 0 auto 0 0;
    box-shadow: var(--shadow-lg);
  }
  .sidebar-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.85rem;
  }
  .sidebar-head button,
  .reopen,
  .search button,
  .section-head,
  .entry-actions button,
  .create {
    font: inherit;
    cursor: pointer;
  }
  .reopen {
    align-self: start;
    margin: 0.5rem;
    padding: 0.35rem 0.5rem;
  }
  .search {
    display: flex;
    gap: 0.25rem;
  }
  .search input {
    flex: 1;
    min-width: 0;
    padding: 0.3rem;
    font: inherit;
  }
  .section {
    display: grid;
    gap: 0.25rem;
  }
  .section-head {
    display: flex;
    justify-content: space-between;
    padding: 0.3rem 0.2rem;
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
    font-size: 0.8rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.25rem;
  }
  li {
    display: grid;
    gap: 0.15rem;
  }
  .entry {
    display: grid;
    text-align: left;
    gap: 0.1rem;
    padding: 0.35rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 0.3rem);
  }
  .label {
    font-size: 0.82rem;
    font-weight: 600;
  }
  .detail {
    font-size: 0.72rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry-actions {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }
  .entry-actions button {
    font-size: 0.7rem;
    padding: 0.1rem 0.3rem;
  }
  .create {
    justify-self: start;
    font-size: 0.75rem;
    padding: 0.2rem 0.4rem;
  }
  .empty {
    margin: 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
</style>
