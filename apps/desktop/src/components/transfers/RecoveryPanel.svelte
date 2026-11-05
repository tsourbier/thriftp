<script lang="ts">
  import type { TransferView } from "../../lib/ipc/types";

  let {
    transfers,
    onresume,
    onrestart,
    oncancel,
    ondiscard
  }: {
    transfers: TransferView[];
    onresume: (transfer: TransferView) => void;
    onrestart: (transfer: TransferView) => void;
    oncancel: (transfer: TransferView) => void;
    ondiscard: (transfer: TransferView) => void;
  } = $props();

  const recoverable = $derived(
    transfers.filter((transfer) =>
      ["interrupted", "failed", "paused"].includes(transfer.state)
    )
  );

  function name(transfer: TransferView) {
    return transfer.destinationPath.split(/[\\/]/).pop() ?? transfer.destinationPath;
  }
</script>

{#if recoverable.length > 0}
  <section class="recovery" aria-label="Recover interrupted transfers">
    <header>
      <h2>Recover interrupted transfers</h2>
      <span>{recoverable.length} need attention</span>
    </header>
    <ul>
      {#each recoverable as transfer (transfer.id)}
        <li>
          <div class="details">
            <strong>{name(transfer)}</strong>
            <span>
              {transfer.state} · {transfer.transferredBytes}
              {transfer.totalBytes !== null ? ` / ${transfer.totalBytes}` : ""} bytes
              {#if transfer.error}· {transfer.error}{/if}
            </span>
          </div>
          <div class="actions">
            {#if transfer.state !== "failed" || transfer.transferredBytes > 0}
              <button type="button" onclick={() => onresume(transfer)}>Resume</button>
            {/if}
            <button type="button" onclick={() => onrestart(transfer)}>Restart</button>
            <button type="button" onclick={() => ondiscard(transfer)}
              >Discard partial</button
            >
            <button type="button" onclick={() => oncancel(transfer)}>Cancel</button>
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .recovery {
    display: grid;
    gap: 0.4rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #d0d0d0;
    background: color-mix(in srgb, #ffd27f 25%, Canvas);
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
  }
  h2 {
    font-size: 0.85rem;
    margin: 0;
  }
  header span {
    font-size: 0.75rem;
    color: GrayText;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.3rem;
  }
  li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }
  .details {
    display: grid;
    min-width: 0;
    font-size: 0.75rem;
  }
  .details strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .details span {
    color: GrayText;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  button {
    font: inherit;
    font-size: 0.7rem;
  }
</style>
