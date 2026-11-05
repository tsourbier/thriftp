<script lang="ts">
  import TransferProgress from "./TransferProgress.svelte";
  import type { TransferView } from "../../lib/ipc/types";

  let {
    transfers,
    onpause,
    onresume,
    oncancel,
    ondismiss
  }: {
    transfers: TransferView[];
    onpause: (transfer: TransferView) => void;
    onresume: (transfer: TransferView) => void;
    oncancel: (transfer: TransferView) => void;
    ondismiss?: (transfer: TransferView) => void;
  } = $props();
</script>

<section class="transfer-list" aria-label="Transfers">
  {#if transfers.length === 0}
    <p class="empty">No transfers yet.</p>
  {:else}
    <div class="items">
      {#each transfers as transfer (transfer.id)}
        <TransferProgress {transfer} {onpause} {onresume} {oncancel} {ondismiss} />
      {/each}
    </div>
  {/if}
</section>

<style>
  .transfer-list {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .items {
    display: grid;
    gap: 0.4rem;
    overflow: auto;
  }
  .empty {
    color: GrayText;
    font-size: 0.85rem;
  }
</style>
