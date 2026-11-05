<script lang="ts">
  let {
    profileName,
    message,
    secretKind = "password",
    onsubmit,
    oncancel
  }: {
    profileName: string;
    message?: string;
    secretKind?: "password" | "passphrase";
    onsubmit: (secret: string) => void;
    oncancel: () => void;
  } = $props();

  let secret = $state("");
  const label = $derived(secretKind === "passphrase" ? "SSH key passphrase" : "Password");
  const title = $derived(
    secretKind === "passphrase" ? "SSH key passphrase required" : "Credentials required"
  );

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!secret) {
      return;
    }
    onsubmit(secret);
  }
</script>

<form class="prompt" onsubmit={submit} aria-label={title}>
  <h2>{title}</h2>
  <p>Enter the {label.toLowerCase()} for <strong>{profileName}</strong>.</p>
  {#if message}
    <p class="message" role="alert">{message}</p>
  {/if}
  <label>
    {label}
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
