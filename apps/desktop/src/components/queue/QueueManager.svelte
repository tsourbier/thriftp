<script lang="ts">
  import { untrack } from "svelte";
  import TransferList from "../transfers/TransferList.svelte";
  import {
    formatBytes,
    formatSpeed,
    type ConnectionTransferStats
  } from "../../lib/transfers/stats";
  import type { Priority, QueueView, TransferView } from "../../lib/ipc/types";

  let {
    transfers,
    queue,
    connectionStats = [],
    onpausequeue,
    onresumequeue,
    onsetconcurrency,
    onsetbandwidth,
    onreorder,
    onsetpriority,
    onpausetransfer,
    onresumetransfer,
    oncanceltransfer,
    oncancelall,
    onclearhistory,
    ondismiss
  }: {
    transfers: TransferView[];
    queue: QueueView | null;
    connectionStats?: ConnectionTransferStats[];
    onpausequeue: () => void;
    onresumequeue: () => void;
    onsetconcurrency: (limit: number) => void;
    onsetbandwidth: (bytesPerSecond: number | null) => void;
    onreorder: (transferIds: string[]) => void;
    onsetpriority: (transferId: string, priority: Priority) => void;
    onpausetransfer: (transfer: TransferView) => void;
    onresumetransfer: (transfer: TransferView) => void;
    oncanceltransfer: (transfer: TransferView) => void;
    oncancelall: () => void;
    onclearhistory: () => void;
    ondismiss: (transfer: TransferView) => void;
  } = $props();

  let collapsed = $state(false);
  let concurrency = $state(untrack(() => queue?.concurrencyLimit ?? 4));
  let bandwidthKb = $state(
    untrack(() =>
      queue?.bandwidthLimitBytesPerSecond
        ? Math.round(queue.bandwidthLimitBytesPerSecond / 1024)
        : 0
    )
  );

  $effect(() => {
    concurrency = queue?.concurrencyLimit ?? 4;
    bandwidthKb = queue?.bandwidthLimitBytesPerSecond
      ? Math.round(queue.bandwidthLimitBytesPerSecond / 1024)
      : 0;
  });

  function move(id: string, delta: number) {
    const order = orderedIds();
    const index = order.indexOf(id);
    const target = index + delta;
    if (index < 0 || target < 0 || target >= order.length) {
      return;
    }
    const next = [...order];
    [next[index], next[target]] = [next[target], next[index]];
    onreorder(next);
  }

  function orderedIds() {
    if (!queue) {
      return visibleTransfers.map((transfer) => transfer.id);
    }
    const known = new Set(visibleTransfers.map((transfer) => transfer.id));
    return queue.orderedTransferIds.filter((id) => known.has(id));
  }

  let dismissed = $state<Set<string>>(new Set());
  const visibleTransfers = $derived(
    transfers.filter(
      (transfer) => transfer.state !== "completed" && !dismissed.has(transfer.id)
    )
  );
  const activeCount = $derived(
    visibleTransfers.filter((transfer) => transfer.state === "active").length
  );
  const activeTransfers = $derived(
    visibleTransfers.filter((transfer) =>
      ["pending", "active", "paused"].includes(transfer.state)
    )
  );
  const aggregateTotal = $derived(
    activeTransfers.reduce((sum, transfer) => sum + (transfer.totalBytes ?? 0), 0)
  );
  const aggregateTransferred = $derived(
    activeTransfers.reduce((sum, transfer) => sum + transfer.transferredBytes, 0)
  );
  const aggregatePercent = $derived(
    aggregateTotal > 0
      ? Math.min(100, Math.round((aggregateTransferred / aggregateTotal) * 100))
      : null
  );

  function dismiss(transfer: TransferView) {
    dismissed = new Set([...dismissed, transfer.id]);
    ondismiss(transfer);
  }
</script>

<section class="queue" class:collapsed aria-label="Transfer queue">
  <header class="heading">
    <div class="heading-text">
      <h2>Queue</h2>
      <span class="summary">
        {activeCount} active · {visibleTransfers.length} total
        {#if aggregatePercent !== null}
          · {aggregatePercent}% aggregate
        {/if}
      </span>
    </div>
    <button
      type="button"
      class="toggle"
      aria-expanded={!collapsed}
      aria-controls="queue-body"
      onclick={() => (collapsed = !collapsed)}
    >
      {collapsed ? "Expand" : "Collapse"}
    </button>
  </header>

  {#if !collapsed}
    <div class="body" id="queue-body">
      <div class="controls">
        <button type="button" onclick={queue?.paused ? onresumequeue : onpausequeue}>
          {queue?.paused ? "Resume queue" : "Pause queue"}
        </button>
        <label class="inline">
          Concurrent
          <input
            type="number"
            min="1"
            max="16"
            bind:value={concurrency}
            onchange={() => onsetconcurrency(concurrency)}
            aria-label="Concurrent transfers"
          />
        </label>
        <label class="inline">
          Limit KB/s
          <input
            type="number"
            min="0"
            bind:value={bandwidthKb}
            onchange={() => onsetbandwidth(bandwidthKb > 0 ? bandwidthKb * 1024 : null)}
            aria-label="Bandwidth limit in kilobytes per second"
          />
        </label>
        <button type="button" class="danger" onclick={oncancelall}>Cancel all</button>
        <button type="button" onclick={onclearhistory}>Clear history</button>
      </div>

      {#if connectionStats.length > 0}
        <div class="connection-stats" aria-label="Connection transfer statistics">
          {#each connectionStats as stat (stat.connectionId)}
            <div class="connection-stat">
              <strong>{stat.displayName}</strong>
              <span>
                {stat.completedFiles} files · {formatBytes(stat.completedBytes)} · {formatSpeed(
                  stat.averageBytesPerSecond
                )}
              </span>
            </div>
          {/each}
        </div>
      {/if}

      <TransferList
        transfers={visibleTransfers}
        onpause={onpausetransfer}
        onresume={onresumetransfer}
        oncancel={oncanceltransfer}
        ondismiss={dismiss}
      />

      <ul class="ordering" aria-label="Queue order">
        {#each orderedIds() as id (id)}
          {@const transfer = visibleTransfers.find((candidate) => candidate.id === id)}
          {#if transfer}
            <li>
              <span class="name" title={transfer.destinationPath}>
                {transfer.destinationPath.split(/[\\/]/).pop()}
              </span>
              <select
                aria-label="Priority"
                value={transfer.priority}
                onchange={(event) =>
                  onsetpriority(
                    id,
                    (event.currentTarget as HTMLSelectElement).value as Priority
                  )}
              >
                <option value="high">High</option>
                <option value="normal">Normal</option>
                <option value="low">Low</option>
              </select>
              <button type="button" onclick={() => move(id, -1)} aria-label="Move up"
                >↑</button
              >
              <button type="button" onclick={() => move(id, 1)} aria-label="Move down"
                >↓</button
              >
            </li>
          {/if}
        {/each}
      </ul>
    </div>
  {/if}
</section>

<style>
  .queue {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    flex: 0 0 auto;
    min-height: 0;
    max-height: 40vh;
    padding: 0.6rem 0.75rem;
    border-top: 1px solid var(--border);
    background: var(--surface);
  }
  .queue.collapsed {
    max-height: none;
  }
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .heading-text {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    min-width: 0;
  }
  .toggle {
    flex: 0 0 auto;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 0;
    overflow: auto;
  }
  h2 {
    font-size: 0.9rem;
    margin: 0;
  }
  .summary {
    font-size: 0.75rem;
    color: GrayText;
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.75rem;
  }
  .connection-stats {
    display: grid;
    gap: 0.25rem;
  }
  .connection-stat {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: GrayText;
  }
  .connection-stat strong {
    color: var(--text);
    font-weight: 600;
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }
  input[type="number"] {
    width: 4rem;
  }
  button,
  select,
  input {
    font: inherit;
    font-size: 0.75rem;
  }
  .danger {
    color: #8a0000;
  }
  .ordering {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.25rem;
  }
  .ordering li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 5rem auto auto;
    gap: 0.25rem;
    align-items: center;
    font-size: 0.75rem;
  }
  .ordering .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
