<script lang="ts">
  import type { DirectoryFavorite } from "../../lib/ipc/types";

  let {
    favorites,
    canSave = false,
    onopen,
    onsave,
    ondelete,
    onedit,
    onmove
  }: {
    favorites: DirectoryFavorite[];
    canSave?: boolean;
    onopen: (favorite: DirectoryFavorite) => void;
    onsave: () => void;
    ondelete: (favorite: DirectoryFavorite) => void;
    onedit?: (favorite: DirectoryFavorite) => void;
    onmove?: (favorite: DirectoryFavorite, delta: number) => void;
  } = $props();
</script>

<aside class="favorites" aria-label="Directory favorites">
  <header>
    <h2>Favorites</h2>
    <button type="button" onclick={onsave} disabled={!canSave}>Save current</button>
  </header>
  {#if favorites.length === 0}
    <p class="empty">No favorite directories yet.</p>
  {:else}
    <ul>
      {#each favorites as favorite (favorite.id)}
        <li>
          <button
            type="button"
            class="open"
            onclick={() => onopen(favorite)}
            title={favorite.path}
          >
            {favorite.label}
            <span>{favorite.path}</span>
          </button>
          {#if onmove}
            <button
              type="button"
              aria-label={`Move ${favorite.label} up`}
              onclick={() => onmove(favorite, -1)}>↑</button
            >
            <button
              type="button"
              aria-label={`Move ${favorite.label} down`}
              onclick={() => onmove(favorite, 1)}>↓</button
            >
          {/if}
          {#if onedit}
            <button
              type="button"
              aria-label={`Edit ${favorite.label}`}
              onclick={() => onedit(favorite)}>✎</button
            >
          {/if}
          <button
            type="button"
            class="delete"
            aria-label={`Delete ${favorite.label}`}
            onclick={() => ondelete(favorite)}
          >
            ×
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .favorites {
    width: 14rem;
    padding: 0.75rem;
    border-right: 1px solid #d0d0d0;
    overflow: auto;
  }
  header {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 0.5rem;
  }
  h2 {
    font-size: 0.9rem;
    margin: 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.25rem;
  }
  li {
    display: flex;
    align-items: stretch;
    gap: 0.2rem;
  }
  .open {
    flex: 1;
    display: grid;
    text-align: left;
    font: inherit;
    font-size: 0.8rem;
    border: 1px solid transparent;
    background: transparent;
    padding: 0.25rem;
    border-radius: 0.3rem;
    min-width: 0;
  }
  .open:hover {
    border-color: #d0d0d0;
  }
  .open span {
    font-size: 0.7rem;
    color: GrayText;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .delete {
    border: none;
    background: transparent;
    color: GrayText;
    font: inherit;
  }
  .empty {
    font-size: 0.8rem;
    color: GrayText;
  }
</style>
