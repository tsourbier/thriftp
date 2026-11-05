<script lang="ts">
  import type { McpClientView, SessionView } from "../../lib/automation/types";

  let {
    clients = [],
    sessions = [],
    onrevoke,
    ondisconnect
  }: {
    clients?: McpClientView[];
    sessions?: SessionView[];
    onrevoke: (fingerprint: string) => void;
    ondisconnect: (fingerprint: string) => void;
  } = $props();

  function sessionCount(fingerprint: string): number {
    return sessions.filter(
      (session) => session.fingerprint === fingerprint && !session.disconnected
    ).length;
  }
</script>

<section class="automation-clients" aria-label="Enrolled MCP clients">
  {#if clients.length === 0}
    <p class="empty">No MCP clients are enrolled.</p>
  {:else}
    <ul>
      {#each clients as client (client.id)}
        <li class:revoked={client.state === "revoked"}>
          <div class="identity">
            <strong>{client.displayName}</strong>
            <span class="fingerprint" title={client.fingerprint}>
              {client.fingerprint.slice(0, 16)}…
            </span>
            <span class="state">{client.state}</span>
            <span class="sessions"
              >{sessionCount(client.fingerprint)} active sessions</span
            >
          </div>
          <div class="actions">
            <button
              type="button"
              onclick={() => ondisconnect(client.fingerprint)}
              aria-label={`Disconnect ${client.displayName}`}
            >
              Disconnect
            </button>
            <button
              type="button"
              class="danger"
              onclick={() => onrevoke(client.fingerprint)}
              aria-label={`Revoke ${client.displayName}`}
            >
              Revoke
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .automation-clients ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .automation-clients li {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border, #ddd);
  }

  .fingerprint {
    font-family: monospace;
    color: var(--muted, #666);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .danger {
    color: var(--danger, #b00020);
  }
</style>
