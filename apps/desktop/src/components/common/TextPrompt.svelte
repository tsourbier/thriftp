<script lang="ts">
  import { untrack } from "svelte";

  let {
    title,
    label,
    initial = "",
    onsubmit,
    oncancel
  }: {
    title: string;
    label: string;
    initial?: string;
    onsubmit: (value: string) => void;
    oncancel: () => void;
  } = $props();

  let value = $state(untrack(() => initial));

  function submit(event: SubmitEvent) {
    event.preventDefault();
    const trimmed = value.trim();
    if (trimmed) {
      onsubmit(trimmed);
    }
  }
</script>

<form class="text-prompt" onsubmit={submit} aria-label={title}>
  <h2>{title}</h2>
  <label>
    {label}
    <input bind:value maxlength="200" />
  </label>
  <div class="actions">
    <button type="button" onclick={oncancel}>Cancel</button>
    <button type="submit" disabled={!value.trim()}>OK</button>
  </div>
</form>

<style>
  .text-prompt {
    display: grid;
    gap: 0.65rem;
    min-width: 22rem;
  }
  h2 {
    margin: 0;
    font-size: 1rem;
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
