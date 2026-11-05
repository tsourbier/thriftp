<script lang="ts">
  import type { ServerFavoriteView } from "../../lib/ipc/types";

  let {
    favorite,
    onconfirm,
    oncancel
  }: {
    favorite: ServerFavoriteView;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();
</script>

<div
  class="dialog"
  role="dialog"
  aria-modal="true"
  aria-labelledby="delete-favorite-title"
>
  <h2 id="delete-favorite-title">Delete favorite?</h2>
  <p>
    Delete <strong>{favorite.displayName}</strong>? The saved connection is removed. Any
    active connection stays connected until you disconnect it.
  </p>
  <div class="actions">
    <button type="button" onclick={onconfirm}>Delete</button>
    <button type="button" onclick={oncancel}>Cancel</button>
  </div>
</div>

<style>
  .dialog {
    border: 1px solid var(--server-favorites-border, #d0d0d0);
    border-radius: 0.4rem;
    padding: 1rem;
    background: Canvas;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
</style>
