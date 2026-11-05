<script lang="ts">
  let {
    onkeep,
    ondelete,
    oncancel
  }: {
    onkeep: () => void;
    ondelete: () => void;
    oncancel: () => void;
  } = $props();
</script>

<div
  class="dialog"
  role="dialog"
  aria-modal="true"
  aria-labelledby="delete-credential-title"
>
  <h2 id="delete-credential-title">Keep or delete the credential?</h2>
  <p>
    No favorite references this credential anymore. Keeping it preserves it for later
    reuse.
  </p>
  <div class="actions">
    <button type="button" onclick={onkeep}>Keep</button>
    <button type="button" onclick={ondelete}>Delete</button>
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
