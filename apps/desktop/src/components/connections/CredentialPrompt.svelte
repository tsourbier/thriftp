<script lang="ts">
  let {
    profileName,
    message,
    onsubmit,
    oncancel
  }: {
    profileName: string;
    message?: string;
    onsubmit: (secret: string) => void;
    oncancel: () => void;
  } = $props();

  let secret = $state("");

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!secret) {
      return;
    }
    onsubmit(secret);
  }
</script>

<form class="prompt" onsubmit={submit} aria-label="Credentials required">
  <h2>Credentials required</h2>
  <p>Enter the credential for <strong>{profileName}</strong>.</p>
  {#if message}
    <p class="message" role="alert">{message}</p>
  {/if}
  <label>
    Credential
    <input type="password" bind:value={secret} autocomplete="current-password" required />
  </label>
  <div class="actions">
    <button type="button" onclick={oncancel}>Cancel</button>
    <button type="submit" disabled={!secret}>Connect</button>
  </div>
</form>

<style>
  .prompt {
    display: grid;
    gap: 0.6rem;
    min-width: 20rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  p {
    margin: 0;
    font-size: 0.9rem;
  }
  .message {
    color: #8a0000;
  }
  label {
    display: grid;
    gap: 0.25rem;
    font-size: 0.85rem;
  }
  input {
    padding: 0.35rem;
    font: inherit;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>
