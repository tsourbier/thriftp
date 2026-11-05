<script lang="ts">
  import type { ReusableCredential } from "../../lib/ipc/types";

  let {
    credentials,
    onselect,
    oncancel
  }: {
    credentials: ReusableCredential[];
    onselect: (credentialRefId: string) => void;
    oncancel: () => void;
  } = $props();
</script>

<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="reuse-title">
  <h2 id="reuse-title">Reuse a saved credential</h2>
  {#if credentials.length === 0}
    <p>No compatible saved credentials are available.</p>
  {:else}
    <ul>
      {#each credentials as credential (credential.credentialRefId)}
        <li>
          <button type="button" onclick={() => onselect(credential.credentialRefId)}>
            {credential.protocol} · {credential.username}@{credential.host}:{credential.port}
            ({credential.affectedFavoriteCount} favorites)
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <button type="button" onclick={oncancel}>Cancel</button>
</div>

<style>
  .dialog {
    border: 1px solid var(--server-favorites-border, #d0d0d0);
    border-radius: 0.4rem;
    padding: 1rem;
    background: Canvas;
  }
  ul {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 0.35rem;
  }
</style>
