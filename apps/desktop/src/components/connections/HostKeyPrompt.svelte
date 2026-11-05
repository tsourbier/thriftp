<script lang="ts">
  let {
    profileName,
    host,
    port,
    fingerprint,
    changed,
    ontrust,
    oncancel
  }: {
    profileName: string;
    host: string;
    port: number;
    fingerprint: string;
    changed: boolean;
    ontrust: () => void;
    oncancel: () => void;
  } = $props();

  let acknowledged = $state(false);

  const displayFingerprint = $derived(
    fingerprint.startsWith("SHA256:") ? fingerprint : `SHA256:${fingerprint}`
  );
  const canTrust = $derived(!changed || acknowledged);
</script>

<div
  class="prompt"
  role="alertdialog"
  aria-modal="true"
  aria-label={changed ? "SSH host key changed" : "Verify SSH host key"}
  tabindex="-1"
>
  <h2>{changed ? "SSH host key changed" : "Verify SSH host key"}</h2>
  <p>
    {#if changed}
      The host key presented by <strong>{host}:{port}</strong> does not match the key you trusted
      before.
    {:else}
      <strong>{profileName}</strong> is connecting to <strong>{host}:{port}</strong> for the
      first time.
    {/if}
  </p>
  <p>Compare this fingerprint with the one shown by the server administrator:</p>
  <code class="fingerprint">{displayFingerprint}</code>
  {#if changed}
    <p class="warning" role="alert">
      This may indicate interception. Do not trust the new key unless you know why it
      changed.
    </p>
    <label class="acknowledge">
      <input type="checkbox" bind:checked={acknowledged} />
      I understand the host key changed and want to replace the trusted key.
    </label>
  {:else}
    <p class="warning">
      Only trust this key if the fingerprint matches the server you expect to reach.
    </p>
  {/if}
  <div class="actions">
    <button type="button" onclick={oncancel}>Cancel</button>
    <button type="button" onclick={ontrust} disabled={!canTrust}>
      Trust and connect
    </button>
  </div>
</div>

<style>
  .prompt {
    display: grid;
    gap: 0.6rem;
    max-width: 30rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  p {
    margin: 0;
    font-size: 0.9rem;
  }
  .fingerprint {
    display: block;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 0.3rem;
    background: var(--surface-muted, #f4f4f4);
    font-size: 0.85rem;
    overflow-wrap: anywhere;
    user-select: all;
  }
  .warning {
    color: #8a0000;
  }
  .acknowledge {
    display: flex;
    align-items: flex-start;
    gap: 0.45rem;
    font-size: 0.85rem;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>
