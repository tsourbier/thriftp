<script lang="ts">
  let {
    code = "credential_missing",
    favoriteName = "this favorite",
    secretKind = "password",
    onsubmit,
    oncancel
  }: {
    code?: string;
    favoriteName?: string;
    secretKind?: "password" | "privateKey" | "passphrase";
    onsubmit: (secret: string, save: boolean) => void;
    oncancel: () => void;
  } = $props();

  let secret = $state("");
  let save = $state(false);

  const canSave = $derived(code !== "credential_store_unavailable");
  const credentialLabel = $derived(
    secretKind === "passphrase"
      ? "SSH key passphrase"
      : secretKind === "privateKey"
        ? "Private key"
        : "Password"
  );
  const message = $derived(
    code === "credential_store_unavailable"
      ? `The operating-system credential store is unavailable. You can use a credential for ${favoriteName} for this session only.`
      : code === "credential_rejected"
        ? `The server rejected the saved credential for ${favoriteName}. Enter a new credential.`
        : `${favoriteName} needs a ${credentialLabel.toLowerCase()} before connecting.`
  );
</script>

<form
  class="dialog"
  onsubmit={(event) => {
    event.preventDefault();
    onsubmit(secret, canSave && save);
  }}
>
  <h2>Credential required</h2>
  <p>{message}</p>
  <label>
    {credentialLabel}
    <input type="password" bind:value={secret} required />
  </label>
  <label class="checkbox">
    <input type="checkbox" bind:checked={save} disabled={!canSave} />
    Save securely
  </label>
  {#if code === "credential_store_unavailable"}
    <p class="warning" role="alert">
      Saving is unavailable; this credential will be discarded after the session.
    </p>
  {/if}
  <div class="actions">
    <button type="submit">Connect</button>
    <button type="button" onclick={oncancel}>Cancel</button>
  </div>
</form>

<style>
  .dialog {
    display: grid;
    gap: 0.5rem;
    border: 1px solid var(--server-favorites-border, #d0d0d0);
    border-radius: 0.4rem;
    padding: 1rem;
    background: Canvas;
  }
  .checkbox {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
  .warning {
    color: var(--server-favorites-warning, #9a6700);
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
</style>
