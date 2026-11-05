<script lang="ts">
  let {
    groupLabel = null,
    onassign,
    onrename,
    onungroup
  }: {
    groupLabel?: string | null;
    onassign: (label: string) => void;
    onrename: (oldLabel: string, newLabel: string) => void;
    onungroup: () => void;
  } = $props();

  let label = $state(groupLabel ?? "");

  function assign() {
    if (label.trim()) onassign(label.trim());
  }
</script>

<div class="group-menu" role="group" aria-label="Favorite group">
  <label>
    Group
    <input bind:value={label} maxlength="100" />
  </label>
  <button type="button" onclick={assign}>Assign</button>
  {#if groupLabel}
    <button type="button" onclick={() => onrename(groupLabel, label)}>Rename group</button
    >
    <button type="button" onclick={onungroup}>Ungroup</button>
  {/if}
</div>

<style>
  .group-menu {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    align-items: end;
  }
  label {
    display: grid;
    gap: 0.2rem;
  }
</style>
