<script lang="ts">
  import type { CredentialImpact } from "../../lib/ipc/types";

  let {
    impact,
    action = "update",
    onconfirm,
    oncancel
  }: {
    impact: CredentialImpact;
    action?: "update" | "remove";
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();
</script>

<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="impact-title">
  <h2 id="impact-title">
    {action === "remove" ? "Remove credential?" : "Update shared credential?"}
  </h2>
  <p>
    This credential is used by {impact.affectedFavorites.length}
    {impact.affectedFavorites.length === 1 ? "favorite" : "favorites"}:
  </p>
  <ul>
    {#each impact.affectedFavorites as favorite (favorite.id)}
      <li>{favorite.displayName}</li>
    {/each}
  </ul>
  {#if impact.isShared}
    <p class="warning" role="alert">The change applies to every favorite listed above.</p>
  {/if}
  <div class="actions">
    <button type="button" onclick={onconfirm}>Continue</button>
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
  .warning {
    color: var(--server-favorites-warning, #9a6700);
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
</style>
