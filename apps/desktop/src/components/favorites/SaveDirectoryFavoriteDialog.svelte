<script lang="ts">
  import { untrack } from "svelte";

  let {
    path,
    onsave,
    oncancel
  }: {
    path: string;
    onsave: (label: string, path: string) => void;
    oncancel: () => void;
  } = $props();

  let label = $state(untrack(() => path.split("/").filter(Boolean).pop() ?? "Favorite"));
  let draftPath = $state(untrack(() => path));

  function submit(event: SubmitEvent) {
    event.preventDefault();
    onsave(label.trim(), draftPath.trim());
  }
</script>

<form class="dialog" onsubmit={submit} aria-label="Save directory favorite">
  <h2>Save directory favorite</h2>
  <label>
    Label
    <input bind:value={label} maxlength="200" required />
  </label>
  <label>
    Path
    <input bind:value={draftPath} required />
  </label>
  <div class="actions">
    <button type="button" onclick={oncancel}>Cancel</button>
    <button type="submit" disabled={!label.trim() || !draftPath.trim()}>Save</button>
  </div>
</form>

<style>
  .dialog {
    display: grid;
    gap: 0.6rem;
    min-width: 22rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  label {
    display: grid;
    gap: 0.25rem;
    font-size: 0.85rem;
  }
  input {
    padding: 0.3rem;
    font: inherit;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.4rem;
  }
</style>
