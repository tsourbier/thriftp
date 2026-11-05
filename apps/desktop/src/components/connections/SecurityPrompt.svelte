<script lang="ts">
  let {
    title,
    message,
    confirmLabel = "Continue",
    onconfirm,
    oncancel
  }: {
    title: string;
    message: string;
    confirmLabel?: string;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();
</script>

<div class="prompt" role="alertdialog" aria-modal="true" aria-label={title} tabindex="-1">
  <h2>{title}</h2>
  <p>{message}</p>
  <div class="actions">
    <button type="button" onclick={oncancel}>Cancel</button>
    <button type="button" onclick={onconfirm}>{confirmLabel}</button>
  </div>
</div>

<style>
  .prompt {
    display: grid;
    gap: 0.6rem;
    max-width: 28rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  p {
    margin: 0;
    font-size: 0.9rem;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>
