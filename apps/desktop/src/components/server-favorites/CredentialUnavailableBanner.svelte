<script lang="ts">
  let {
    code = "credential_store_unavailable",
    onretry,
    onrepair
  }: {
    code?: string;
    onretry?: () => void;
    onrepair?: () => void;
  } = $props();
</script>

<div class="banner" role="alert">
  {#if code === "credential_store_unavailable"}
    The operating-system credential store is unavailable. No plaintext fallback will be
    used.
  {:else if code === "credential_missing"}
    The saved credential is missing. Re-enter it to repair this favorite.
  {:else}
    The saved credential could not be used. Re-enter it to repair this favorite.
  {/if}
  <div class="actions">
    {#if onretry}<button type="button" onclick={onretry}>Retry</button>{/if}
    {#if onrepair}<button type="button" onclick={onrepair}>Repair credential</button>{/if}
  </div>
</div>

<style>
  .banner {
    border: 1px solid #d0a000;
    border-radius: 0.4rem;
    padding: 0.6rem;
    background: #fff8e1;
    color: #5c4300;
    display: grid;
    gap: 0.4rem;
  }
  .actions {
    display: flex;
    gap: 0.4rem;
  }
</style>
