<script lang="ts">
  let {
    linked,
    ontoggle
  }: {
    linked: boolean;
    ontoggle: () => void;
  } = $props();
</script>

<button
  type="button"
  aria-pressed={linked}
  title={linked
    ? "Stop mirroring navigation between the panes"
    : "Mirror navigation between the panes"}
  onclick={ontoggle}
>
  {linked ? "Unlink" : "Link"}
</button>
