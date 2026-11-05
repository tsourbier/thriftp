<script lang="ts">
  import type { AuditEventView } from "../../lib/automation/types";

  let {
    events = [],
    valid = null
  }: { events?: AuditEventView[]; valid?: boolean | null } = $props();
</script>

<section class="automation-audit" aria-label="Audit trail">
  <header>
    <h3>Audit trail</h3>
    {#if valid === true}
      <span class="integrity ok">Chain verified</span>
    {:else if valid === false}
      <span class="integrity broken">Chain broken</span>
    {:else}
      <span class="integrity unknown">Integrity unknown</span>
    {/if}
  </header>
  {#if events.length === 0}
    <p class="empty">No audit records yet.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Seq</th>
          <th>Time</th>
          <th>Interface</th>
          <th>Operation</th>
          <th>Result</th>
        </tr>
      </thead>
      <tbody>
        {#each events as event (event.seq)}
          <tr class:failed={event.result === "failed" || event.result === "denied"}>
            <td>{event.seq}</td>
            <td>{event.timestamp}</td>
            <td>{event.interface}</td>
            <td>{event.operation}</td>
            <td>{event.result}{event.errorCode ? ` (${event.errorCode})` : ""}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  table {
    border-collapse: collapse;
    width: 100%;
  }

  th,
  td {
    text-align: left;
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid var(--border, #ddd);
  }

  .integrity.ok {
    color: var(--success, #0a7d32);
  }

  .integrity.broken {
    color: var(--danger, #b00020);
  }
</style>
