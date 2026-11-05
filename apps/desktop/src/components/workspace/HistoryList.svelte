<script lang="ts">
  import type { ConnectionHistoryEntry } from "../../lib/ipc/types";

  let {
    entries = [],
    onconnect,
    onclear
  }: {
    entries?: ConnectionHistoryEntry[];
    onconnect?: (entry: ConnectionHistoryEntry) => void;
    onclear?: () => void;
  } = $props();
</script>

<div class="history">
  <div class="history-head">
    <span>Recent successful connections</span>
    {#if entries.length > 0}
      <button type="button" onclick={onclear}>Clear history</button>
    {/if}
  </div>
  {#if entries.length === 0}
    <p class="empty">Recent successful connections will appear here after you connect.</p>
  {:else}
    <ul>
      {#each entries as entry (entry.favoriteId ?? entry.connectionProfileId)}
        <li>
          <div>
            <strong>{entry.displayName}</strong>
            <span class="detail">{entry.protocol} · {entry.host}:{entry.port}</span>
            <span class="detail"
              >{entry.username} · {new Date(entry.lastUsedAt).toLocaleString()}</span
            >
          </div>
          <button type="button" onclick={() => onconnect?.(entry)}>Connect</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .history {
    display: grid;
    gap: 0.5rem;
  }
  .history-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .history-head button,
  li button {
    font: inherit;
    font-size: 0.75rem;
    cursor: pointer;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.35rem;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 0.3rem);
    background: var(--surface);
  }
  li > div {
    display: grid;
    gap: 0.1rem;
    min-width: 0;
  }
  .detail {
    font-size: 0.72rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    margin: 0;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
</style>
